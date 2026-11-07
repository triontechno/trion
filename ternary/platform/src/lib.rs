//! Target capability discovery and adaptive runtime integration for TRION.
//!
//! This crate reports facts available from the compilation target and exposes
//! explicit binary-interface boundaries. It does not infer native ternary
//! hardware from an architecture name or claim to probe devices without a
//! verified platform adapter.

#![forbid(unsafe_code)]

pub use trion_adaptive::{
    Boundary, CandidateMetrics, CandidateSet, Representation, Selection, SelectionError,
    SelectionReason, SelectionRequest, select_representation,
};

/// Stable architecture identifier used in capability reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostArchitecture {
    X86_64,
    Aarch64,
    Riscv64,
    Other,
}

/// Operations implemented by the bounded legacy TIR test interpreter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TirOperation {
    Nop,
    MovImm64,
    Ret,
}

/// Facts guaranteed by the current build target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityReport {
    pub schema_version: u16,
    pub architecture: HostArchitecture,
    pub operating_system: &'static str,
    pub pointer_width: u8,
    pub little_endian: bool,
    pub binary_execution: bool,
    /// False until a hardware-specific adapter positively verifies native ternary.
    pub native_ternary_hardware: bool,
    /// True only because the bounded research/test interpreter is built in.
    pub software_tir_runtime: bool,
    pub tir_operations: &'static [TirOperation],
    /// Established device, DMA, MMIO, syscall, and wire interfaces remain binary.
    pub binary_external_boundaries: bool,
    pub atomic_8: bool,
    pub atomic_16: bool,
    pub atomic_32: bool,
    pub atomic_64: bool,
    pub atomic_128: bool,
}

impl CapabilityReport {
    /// Report compile-target facts; never infer optional hardware features.
    pub fn for_build_target() -> Self {
        let architecture = match std::env::consts::ARCH {
            "x86_64" => HostArchitecture::X86_64,
            "aarch64" => HostArchitecture::Aarch64,
            "riscv64" => HostArchitecture::Riscv64,
            _ => HostArchitecture::Other,
        };

        Self {
            schema_version: 2,
            architecture,
            operating_system: std::env::consts::OS,
            pointer_width: usize::BITS as u8,
            little_endian: cfg!(target_endian = "little"),
            binary_execution: true,
            native_ternary_hardware: false,
            software_tir_runtime: true,
            tir_operations: &[TirOperation::Nop, TirOperation::MovImm64, TirOperation::Ret],
            binary_external_boundaries: true,
            atomic_8: cfg!(target_has_atomic = "8"),
            atomic_16: cfg!(target_has_atomic = "16"),
            atomic_32: cfg!(target_has_atomic = "32"),
            atomic_64: cfg!(target_has_atomic = "64"),
            atomic_128: cfg!(target_has_atomic = "128"),
        }
    }

    pub fn supports_tir(&self, operation: TirOperation) -> bool {
        self.software_tir_runtime && self.tir_operations.contains(&operation)
    }

    /// Delegate a workload decision to the shared policy using caller-supplied
    /// measured costs and validation evidence. No costs are fabricated here.
    pub fn select_runtime_path(
        &self,
        request: SelectionRequest,
    ) -> Result<Selection, SelectionError> {
        select_representation(request)
    }

    /// Stable key/value diagnostic representation (schema version 2).
    pub fn to_report_text(&self) -> String {
        let architecture = match self.architecture {
            HostArchitecture::X86_64 => "x86_64",
            HostArchitecture::Aarch64 => "aarch64",
            HostArchitecture::Riscv64 => "riscv64",
            HostArchitecture::Other => "other",
        };
        let operations = self.tir_operations.iter().map(|op| match op {
            TirOperation::Nop => "NOP",
            TirOperation::MovImm64 => "MOV_IMM64",
            TirOperation::Ret => "RET",
        }).collect::<Vec<_>>().join(",");

        format!(
            "TRION-CAPABILITIES {}\narchitecture={}\nos={}\npointer_width={}\nlittle_endian={}\nbinary_execution={}\nnative_ternary_hardware={}\nsoftware_tir_runtime={}\ntir_operations={}\nbinary_external_boundaries={}\natomic_8={}\natomic_16={}\natomic_32={}\natomic_64={}\natomic_128={}\n",
            self.schema_version, architecture, self.operating_system, self.pointer_width,
            self.little_endian, self.binary_execution, self.native_ternary_hardware,
            self.software_tir_runtime, operations, self.binary_external_boundaries,
            self.atomic_8, self.atomic_16, self.atomic_32, self.atomic_64, self.atomic_128,
        )
    }
}

/// Conventional hardware and OS interfaces are byte-oriented contracts.
/// This wrapper intentionally performs no ternary reinterpretation or copying.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryBoundaryKind {
    MmioRegister,
    DmaDescriptor,
    DeviceDriverAbi,
    SystemCallAbi,
    NetworkFrame,
    PersistentFormat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BinaryBoundary<'a> {
    kind: BinaryBoundaryKind,
    bytes: &'a [u8],
}

impl<'a> BinaryBoundary<'a> {
    pub const fn new(kind: BinaryBoundaryKind, bytes: &'a [u8]) -> Self {
        Self { kind, bytes }
    }

    pub const fn kind(&self) -> BinaryBoundaryKind {
        self.kind
    }

    pub const fn as_bytes(&self) -> &'a [u8] {
        self.bytes
    }

    pub const fn len(&self) -> usize {
        self.bytes.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics(cost: u128) -> CandidateMetrics {
        CandidateMetrics::new(cost, true, true, true)
    }

    #[test]
    fn target_report_contains_build_facts_and_never_guesses_ternary_hardware() {
        let report = CapabilityReport::for_build_target();
        assert_eq!(report.schema_version, 2);
        assert_eq!(report.pointer_width, usize::BITS as u8);
        assert_eq!(report.operating_system, std::env::consts::OS);
        assert!(!report.native_ternary_hardware);
        assert!(report.binary_execution);
        assert!(report.binary_external_boundaries);
    }

    #[test]
    fn reports_only_the_implemented_legacy_tir_subset() {
        let report = CapabilityReport::for_build_target();
        assert!(report.supports_tir(TirOperation::Nop));
        assert!(report.supports_tir(TirOperation::MovImm64));
        assert!(report.supports_tir(TirOperation::Ret));
        assert_eq!(report.tir_operations.len(), 3);
    }

    #[test]
    fn report_text_exposes_target_and_boundary_facts() {
        let text = CapabilityReport::for_build_target().to_report_text();
        assert!(text.starts_with("TRION-CAPABILITIES 2\n"));
        assert!(text.contains("native_ternary_hardware=false\n"));
        assert!(text.contains("binary_external_boundaries=true\n"));
        assert!(text.contains("pointer_width="));
    }

    #[test]
    fn runtime_uses_shared_adaptive_policy_and_requires_evidence() {
        let report = CapabilityReport::for_build_target();
        let selected = report.select_runtime_path(SelectionRequest {
            boundary: Boundary::Internal,
            candidates: CandidateSet {
                binary: Some(metrics(100)),
                ternary: Some(metrics(50)),
                hybrid: None,
            },
        }).unwrap();
        assert_eq!(selected.representation, Representation::BalancedTernary);
    }

    #[test]
    fn hardware_boundary_preserves_original_bytes() {
        let descriptor = [0x10, 0x00, 0xFF, 0x80];
        let boundary = BinaryBoundary::new(BinaryBoundaryKind::DmaDescriptor, &descriptor);
        assert_eq!(boundary.kind(), BinaryBoundaryKind::DmaDescriptor);
        assert_eq!(boundary.as_bytes(), &descriptor);
        assert_eq!(boundary.len(), descriptor.len());
        assert!(!boundary.is_empty());
    }

    #[test]
    fn empty_binary_boundary_is_valid_and_explicit() {
        let boundary = BinaryBoundary::new(BinaryBoundaryKind::NetworkFrame, &[]);
        assert!(boundary.is_empty());
        assert_eq!(boundary.as_bytes(), &[]);
    }
}
