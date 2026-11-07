use redox_scheme::scheme::register_sync_scheme;
use redox_scheme::Socket;
use scheme_utils::Blocking;

mod scheme;

fn main() {
    daemon::Daemon::new(daemon);
}

fn pci_inventory_with_retry(
    mut probe: impl FnMut() -> Option<usize>,
    max_attempts: usize,
    retry_delay: std::time::Duration,
) -> (bool, usize, usize) {
    for attempt in 1..=max_attempts {
        if let Some(entries) = probe() {
            return (true, entries, attempt);
        }
        if attempt < max_attempts {
            std::thread::sleep(retry_delay);
        }
    }
    (false, 0, max_attempts)
}

#[cfg(test)]
mod tests {
    use super::pci_inventory_with_retry;
    use std::time::Duration;

    #[test]
    fn pci_inventory_waits_for_registration_and_returns_enumerated_count() {
        let mut attempts = 0;
        let result = pci_inventory_with_retry(
            || {
                attempts += 1;
                (attempts >= 3).then_some(7)
            },
            5,
            Duration::ZERO,
        );
        assert_eq!(result, (true, 7, 3));
        assert_eq!(attempts, 3);
    }

    #[test]
    fn pci_inventory_reports_unavailable_after_bounded_retries() {
        let mut attempts = 0;
        let result = pci_inventory_with_retry(
            || {
                attempts += 1;
                None
            },
            4,
            Duration::ZERO,
        );
        assert_eq!(result, (false, 0, 4));
        assert_eq!(attempts, 4);
    }

    #[test]
    fn pci_inventory_accepts_an_empty_but_registered_scheme() {
        let result = pci_inventory_with_retry(|| Some(0), 3, Duration::ZERO);
        assert_eq!(result, (true, 0, 1));
    }
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
    // Give the hardware manager and PCI spawner a bounded settling window.
    // A registered but empty scheme is distinct from an unavailable scheme.
    let (pci_scheme_available, pci_function_entries, pci_inventory_attempts) =
        pci_inventory_with_retry(
            || inventory("/scheme/pci"),
            10,
            std::time::Duration::from_millis(100),
        );
    let (acpi_symbols_available, acpi_symbol_entries, acpi_inventory_attempts) =
        pci_inventory_with_retry(
            || inventory("/scheme/acpi/symbols"),
            10,
            std::time::Duration::from_millis(100),
        );
    let hardware_evidence = format!(
        "TRION-HARDWARE-EVIDENCE 2\npci_scheme_available={}\npci_function_entries={}\npci_inventory_attempts={}\nacpi_symbols_available={}\nacpi_symbol_entries={}\nacpi_inventory_attempts={}\nacpi_device_tree_discovery=delegated_to_hwd\noptional_device_capabilities=not_probed\nnative_ternary_hardware=unverified\n",
        pci_scheme_available,
        pci_function_entries,
        pci_inventory_attempts,
        acpi_symbols_available,
        acpi_symbol_entries,
        acpi_inventory_attempts,
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
fn inventory(path: &str) -> Option<usize> {
    std::fs::read_dir(path)
        .ok()
        .map(|entries| entries.filter_map(Result::ok).count())
}
