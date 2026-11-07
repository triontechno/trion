use redox_scheme::scheme::register_sync_scheme;
use redox_scheme::Socket;
use scheme_utils::Blocking;

mod scheme;

fn main() {
    daemon::Daemon::new(daemon);
}

fn daemon(daemon: daemon::Daemon) -> ! {
    common::setup_logging(
        "misc",
        "trion",
        "trion-capd",
        common::output_level(),
        common::file_level(),
    );

    let capability_report = trion_platform::CapabilityReport::for_build_target().to_report_text();
    let adaptive_probe = trion_platform::run_adaptive_integer_probe(16_384)
        .expect("trion-capd: adaptive workload failed semantic validation");
    let runtime_policy = format!(
        "TRION-ADAPTIVE-POLICY 1\nworkload=integer-roundtrip\niterations={}\nsamples={}\nbinary_cost_ns={}\nternary_cost_ns={}\nchecksum={}\nselected={:?}\nselection_reason={:?}\n",
        adaptive_probe.iterations,
        adaptive_probe.samples,
        adaptive_probe.binary_cost_ns,
        adaptive_probe.ternary_cost_ns,
        adaptive_probe.checksum,
        adaptive_probe.selection.representation,
        adaptive_probe.selection.reason,
    );

    // hwd owns ACPI/DeviceTree backend discovery. These read-only directory
    // inventories record what its published schemes expose; they do not claim
    // that optional device capabilities have been probed.
    let (pci_scheme_available, pci_function_entries) = inventory("/scheme/pci");
    let (acpi_symbols_available, acpi_symbol_entries) = inventory("/scheme/acpi/symbols");
    let hardware_evidence = format!(
        "TRION-HARDWARE-EVIDENCE 2\npci_scheme_available={}\npci_function_entries={}\nacpi_symbols_available={}\nacpi_symbol_entries={}\nacpi_device_tree_discovery=delegated_to_hwd\noptional_device_capabilities=not_probed\nnative_ternary_hardware=unverified\n",
        pci_scheme_available,
        pci_function_entries,
        acpi_symbols_available,
        acpi_symbol_entries,
    );

    let mut capability_scheme =
        scheme::CapabilityScheme::new(capability_report, runtime_policy, hardware_evidence);
    let socket = Socket::nonblock().expect("trion-capd: failed to create scheme socket");
    let handler = Blocking::new(Box::new(socket), 16);

    register_sync_scheme(handler.socket(), "trion", &mut capability_scheme)
        .expect("trion-capd: failed to register trion scheme");
    log::info!("capability scheme registered at /scheme/trion");
    daemon.ready();

    libredox::call::setrens(0, 0).expect("trion-capd: failed to enter null namespace");

    handler
        .process_requests_blocking(capability_scheme)
        .expect("trion-capd: failed to process scheme requests");
}

/// Return whether a Redox scheme directory is available and how many entries
/// it currently publishes. A missing scheme is reported, never guessed.
fn inventory(path: &str) -> (bool, usize) {
    match std::fs::read_dir(path) {
        Ok(entries) => (true, entries.filter_map(Result::ok).count()),
        Err(_) => (false, 0),
    }
}
