//! Explicit host capability reporting for TRION's adaptive runtime.
//!
//! This crate reports only capabilities established by the build target and
//! implemented software. It does not probe for hypothetical ternary hardware.
//! Hardware drivers may construct a report from independently verified device
//! data in a future integration layer.

#![forbid(unsafe_code)]

/// Stable architecture identifier used in capability reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostArchitecture {
    X86_64,
    Aarch64,
    Other,
}

/// Operations implemented by the current bounded TIR v1 interpreter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TirOperation {
    Nop,
    MovImm64,
    Ret,
}

/// A versioned, serializable-in-principle snapshot of host/runtime capabilities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityReport {
    pub schema_version: u16,
    pub architecture: HostArchitecture,
    /// Ordinary binary machine code executes on the host CPU.
    pub binary_execution: bool,
    /// True only if native ternary hardware was positively verified.
    pub native_ternary_hardware: bool,
    /// True when the bounded software TIR interpreter is compiled in.
    pub software_tir_runtime: bool,
    /// Operations accepted by the current TIR parser/interpreter.
    pub tir_operations: &'static [TirOperation],
    /// Conventional external interfaces use their specified binary encodings.
    pub binary_device_and_wire_boundaries: bool,
}

impl CapabilityReport {
    /// Report capabilities guaranteed by this build, not guessed hardware features.
    pub fn for_build_target() -> Self {
        let architecture = match std::env::consts::ARCH {
            "x86_64" => HostArchitecture::X86_64,
            "aarch64" => HostArchitecture::Aarch64,
            _ => HostArchitecture::Other,
        };
        Self {
            schema_version: 1,
            architecture,
            binary_execution: true,
            native_ternary_hardware: false,
            software_tir_runtime: true,
            tir_operations: &[TirOperation::Nop, TirOperation::MovImm64, TirOperation::Ret],
            binary_device_and_wire_boundaries: true,
        }
    }

    pub fn supports_tir(&self, operation: TirOperation) -> bool {
        self.software_tir_runtime && self.tir_operations.contains(&operation)
    }

    /// A compact, stable text representation suitable for diagnostics and bug reports.
    pub fn to_report_text(&self) -> String {
        let architecture = match self.architecture {
            HostArchitecture::X86_64 => "x86_64",
            HostArchitecture::Aarch64 => "aarch64",
            HostArchitecture::Other => "other",
        };
        let operations = self
            .tir_operations
            .iter()
            .map(|op| match op {
                TirOperation::Nop => "NOP",
                TirOperation::MovImm64 => "MOV_IMM64",
                TirOperation::Ret => "RET",
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "TRION-CAPABILITIES {}\narchitecture={}\nbinary_execution={}\nnative_ternary_hardware={}\nsoftware_tir_runtime={}\ntir_operations={}\nbinary_external_boundaries={}\n",
            self.schema_version,
            architecture,
            self.binary_execution,
            self.native_ternary_hardware,
            self.software_tir_runtime,
            operations,
            self.binary_device_and_wire_boundaries,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_report_never_claims_unverified_ternary_hardware() {
        let report = CapabilityReport::for_build_target();
        assert_eq!(report.schema_version, 1);
        assert!(!report.native_ternary_hardware);
        assert!(report.binary_execution);
        assert!(report.binary_device_and_wire_boundaries);
    }

    #[test]
    fn reports_only_the_implemented_tir_subset() {
        let report = CapabilityReport::for_build_target();
        assert!(report.supports_tir(TirOperation::Nop));
        assert!(report.supports_tir(TirOperation::MovImm64));
        assert!(report.supports_tir(TirOperation::Ret));
        assert_eq!(report.tir_operations.len(), 3);
    }

    #[test]
    fn report_text_has_a_version_and_explicit_hardware_status() {
        let text = CapabilityReport::for_build_target().to_report_text();
        assert!(text.starts_with("TRION-CAPABILITIES 1\n"));
        assert!(text.contains("native_ternary_hardware=false\n"));
        assert!(text.contains("tir_operations=NOP,MOV_IMM64,RET\n"));
    }
}
