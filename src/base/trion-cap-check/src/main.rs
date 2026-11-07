use std::fs;

fn field<'a>(report: &'a str, key: &str) -> Option<&'a str> {
    report.lines().find_map(|line| line.strip_prefix(key))
}

fn required_field<'a>(report: &'a str, key: &str) -> &'a str {
    field(report, key).unwrap_or_else(|| panic!("missing report field: {key}"))
}

fn main() {
    common::setup_logging(
        "misc",
        "trion",
        "trion-cap-check",
        common::output_level(),
        common::file_level(),
    );

    let capabilities = fs::read_to_string("/scheme/trion/capabilities")
        .expect("trion-cap-check: cannot read capability report");
    assert!(
        capabilities.starts_with("TRION-CAPABILITIES 3\n"),
        "trion-cap-check: unsupported capability schema"
    );
    assert!(
        !required_field(&capabilities, "architecture=").is_empty(),
        "trion-cap-check: target architecture is empty"
    );
    assert!(
        !required_field(&capabilities, "os=").is_empty(),
        "trion-cap-check: target operating system is empty"
    );
    assert_eq!(
        required_field(&capabilities, "native_ternary_hardware="),
        "false",
        "trion-cap-check: unverified native ternary hardware was advertised"
    );
    assert_eq!(
        required_field(&capabilities, "binary_external_boundaries="),
        "true",
        "trion-cap-check: binary hardware boundary contract is missing"
    );

    let runtime_policy = fs::read_to_string("/scheme/trion/runtime-policy")
        .expect("trion-cap-check: cannot read adaptive runtime policy");
    assert!(
        runtime_policy.starts_with("TRION-ADAPTIVE-POLICY 1\n"),
        "trion-cap-check: unsupported runtime-policy schema"
    );
    assert_eq!(
        required_field(&runtime_policy, "samples="),
        "5",
        "trion-cap-check: expected five measurement samples"
    );
    let iterations = required_field(&runtime_policy, "iterations=")
        .parse::<usize>()
        .expect("trion-cap-check: invalid iteration count");
    assert!((1..=100_000).contains(&iterations));
    for key in ["binary_cost_ns=", "ternary_cost_ns="] {
        let cost = required_field(&runtime_policy, key)
            .parse::<u128>()
            .expect("trion-cap-check: invalid measured cost");
        assert!(cost > 0, "trion-cap-check: measured cost must be positive");
    }
    required_field(&runtime_policy, "checksum=")
        .parse::<i128>()
        .expect("trion-cap-check: invalid workload checksum");
    assert!(
        matches!(
            required_field(&runtime_policy, "selected="),
            "Binary" | "BalancedTernary"
        ),
        "trion-cap-check: adaptive selector returned an unknown representation"
    );

    let hardware = fs::read_to_string("/scheme/trion/hardware-evidence")
        .expect("trion-cap-check: cannot read hardware evidence");
    assert!(
        hardware.starts_with("TRION-HARDWARE-EVIDENCE 2\n"),
        "trion-cap-check: unsupported hardware-evidence schema"
    );
    for (available_key, count_key) in [
        ("pci_scheme_available=", "pci_function_entries="),
        ("acpi_symbols_available=", "acpi_symbol_entries="),
    ] {
        let available = required_field(&hardware, available_key)
            .parse::<bool>()
            .expect("trion-cap-check: invalid hardware scheme availability flag");
        let count = required_field(&hardware, count_key)
            .parse::<usize>()
            .expect("trion-cap-check: invalid hardware inventory count");
        assert!(
            available || count == 0,
            "trion-cap-check: inventory entries reported while scheme is unavailable"
        );
    }
    assert_eq!(
        required_field(&hardware, "optional_device_capabilities="),
        "not_probed"
    );
    assert_eq!(
        required_field(&hardware, "native_ternary_hardware="),
        "unverified"
    );

    log::info!("capability reports validated from /scheme/trion");
}
