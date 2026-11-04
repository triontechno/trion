//! Adaptive binary/ternary representation selection for TRION.
//!
//! This crate chooses among logical representations from explicit capability,
//! compatibility, validation, and cost inputs. It does not detect hardware,
//! invent benchmark values, translate machine code, or provide native ternary
//! execution. Callers must supply target-specific evidence.

#![forbid(unsafe_code)]

use core::fmt;
use trion_ternary::{Ternary, TernaryError};

/// Convert a signed binary integer into its canonical balanced-ternary value.
pub fn binary_i128_to_balanced(value: i128) -> Ternary {
    Ternary::from_i128(value)
}

/// Convert a balanced-ternary value to i128, reporting overflow explicitly.
pub fn balanced_to_binary_i128(value: &Ternary) -> Result<i128, TernaryError> {
    value.to_i128()
}

/// Convert an unsigned binary integer into its canonical balanced-ternary value.
pub fn binary_u128_to_balanced(value: u128) -> Ternary {
    Ternary::from_u128(value)
}

/// Convert a balanced-ternary value to u128, rejecting negative values and overflow.
pub fn balanced_to_binary_u128(value: &Ternary) -> Result<u128, TernaryError> {
    value.to_u128()
}

/// A logical representation selected for a workload.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Representation {
    /// Conventional bits, bytes, and machine integers.
    Binary,
    /// Balanced-ternary logical values (-1, 0, +1).
    BalancedTernary,
    /// Ternary-capable internal work with conversion at an external boundary.
    Hybrid,
}

/// The interface at which the selected representation will be used.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// Internal computation with no externally mandated byte layout.
    Internal,
    /// MMIO, DMA, device drivers, or conventional processor/device interfaces.
    ConventionalHardware,
    /// A conventional network protocol or wire format.
    NetworkWire,
    /// An established binary file or persistent storage format.
    PersistentBinaryFormat,
    /// A security-sensitive cryptographic operation or boundary.
    Cryptographic,
}

/// Evidence supplied for one candidate implementation.
///
/// `total_cost` must be comparable across candidates for the same workload.
/// It may represent a weighted score or a measured total cost, but the unit and
/// weighting are the caller's responsibility. Missing candidates are expressed
/// as `None`, not as an invented large/small score.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateMetrics {
    /// Total comparable cost; lower is better.
    pub total_cost: u128,
    /// Whether the target can execute or faithfully emulate this candidate.
    pub supported: bool,
    /// Whether semantic correctness has been validated for this workload.
    pub semantically_validated: bool,
    /// Whether the candidate preserves the required external compatibility.
    pub compatible: bool,
}

impl CandidateMetrics {
    pub const fn new(
        total_cost: u128,
        supported: bool,
        semantically_validated: bool,
        compatible: bool,
    ) -> Self {
        Self {
            total_cost,
            supported,
            semantically_validated,
            compatible,
        }
    }

    const fn eligible(self, require_compatibility: bool) -> bool {
        self.supported
            && self.semantically_validated
            && (!require_compatibility || self.compatible)
    }
}

/// Candidate evidence for the three available paths.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CandidateSet {
    pub binary: Option<CandidateMetrics>,
    pub ternary: Option<CandidateMetrics>,
    pub hybrid: Option<CandidateMetrics>,
}

/// Inputs to the representation selector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectionRequest {
    pub boundary: Boundary,
    pub candidates: CandidateSet,
}

/// Why a representation was selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionReason {
    LowestValidatedInternalCost,
    CompatibleHybridAtBinaryBoundary,
    BinaryCompatibilityFallback,
    ValidatedCryptographicCandidate,
}

/// Result of an adaptive selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub representation: Representation,
    pub estimated_cost: u128,
    pub reason: SelectionReason,
}

/// Returned when no supported, validated path satisfies the boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionError {
    NoSupportedValidatedCandidate,
    NoCompatibleBinaryBoundaryPath,
}

impl fmt::Display for SelectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSupportedValidatedCandidate => {
                f.write_str("no supported and semantically validated representation candidate")
            }
            Self::NoCompatibleBinaryBoundaryPath => {
                f.write_str("no compatible path is available for the binary boundary")
            }
        }
    }
}

impl std::error::Error for SelectionError {}

/// Select the lowest-cost safe representation for a workload.
///
/// Internal workloads may select Binary, BalancedTernary, or Hybrid.
/// Conventional hardware, network, and persistent binary boundaries require
/// compatibility and prefer Hybrid only when it is supported, validated,
/// compatible, and no more costly than the binary candidate. Otherwise they
/// fall back to a validated compatible Binary candidate.
///
/// Cryptographic workloads require semantic validation and compatibility for
/// every candidate; a ternary candidate is never preferred merely because it
/// is ternary.
pub fn select_representation(
    request: SelectionRequest,
) -> Result<Selection, SelectionError> {
    match request.boundary {
        Boundary::Internal => select_internal(request.candidates),
        Boundary::ConventionalHardware
        | Boundary::NetworkWire
        | Boundary::PersistentBinaryFormat => select_binary_boundary(request.candidates),
        Boundary::Cryptographic => select_cryptographic(request.candidates),
    }
}

fn select_internal(candidates: CandidateSet) -> Result<Selection, SelectionError> {
    let mut best: Option<Selection> = None;

    consider(
        &mut best,
        candidates.binary,
        Representation::Binary,
        SelectionReason::LowestValidatedInternalCost,
        false,
    );
    consider(
        &mut best,
        candidates.ternary,
        Representation::BalancedTernary,
        SelectionReason::LowestValidatedInternalCost,
        false,
    );
    consider(
        &mut best,
        candidates.hybrid,
        Representation::Hybrid,
        SelectionReason::LowestValidatedInternalCost,
        false,
    );

    best.ok_or(SelectionError::NoSupportedValidatedCandidate)
}

fn select_binary_boundary(
    candidates: CandidateSet,
) -> Result<Selection, SelectionError> {
    let binary = candidates.binary.filter(|m| m.eligible(true));
    let hybrid = candidates.hybrid.filter(|m| m.eligible(true));

    match (binary, hybrid) {
        (Some(binary), Some(hybrid)) if hybrid.total_cost <= binary.total_cost => Ok(Selection {
            representation: Representation::Hybrid,
            estimated_cost: hybrid.total_cost,
            reason: SelectionReason::CompatibleHybridAtBinaryBoundary,
        }),
        (Some(binary), _) => Ok(Selection {
            representation: Representation::Binary,
            estimated_cost: binary.total_cost,
            reason: SelectionReason::BinaryCompatibilityFallback,
        }),
        (None, Some(hybrid)) => Ok(Selection {
            representation: Representation::Hybrid,
            estimated_cost: hybrid.total_cost,
            reason: SelectionReason::CompatibleHybridAtBinaryBoundary,
        }),
        (None, None) => Err(SelectionError::NoCompatibleBinaryBoundaryPath),
    }
}

fn select_cryptographic(
    candidates: CandidateSet,
) -> Result<Selection, SelectionError> {
    // Validation is mandatory for all crypto paths. Prefer the lowest-cost
    // validated candidate, but do not infer security from its number system.
    let mut best: Option<Selection> = None;
    consider(
        &mut best,
        candidates.binary,
        Representation::Binary,
        SelectionReason::ValidatedCryptographicCandidate,
        true,
    );
    consider(
        &mut best,
        candidates.ternary,
        Representation::BalancedTernary,
        SelectionReason::ValidatedCryptographicCandidate,
        true,
    );
    consider(
        &mut best,
        candidates.hybrid,
        Representation::Hybrid,
        SelectionReason::ValidatedCryptographicCandidate,
        true,
    );
    best.ok_or(SelectionError::NoSupportedValidatedCandidate)
}

fn consider(
    best: &mut Option<Selection>,
    candidate: Option<CandidateMetrics>,
    representation: Representation,
    reason: SelectionReason,
    require_compatibility: bool,
) {
    let Some(metrics) = candidate.filter(|m| m.eligible(require_compatibility)) else {
        return;
    };

    let replace = best
        .map(|current| metrics.total_cost < current.estimated_cost)
        .unwrap_or(true);

    if replace {
        *best = Some(Selection {
            representation,
            estimated_cost: metrics.total_cost,
            reason,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_binary_ternary_bridges_round_trip() {
        for value in [i128::MIN, -100, -1, 0, 1, 100, i128::MAX] {
            let ternary = binary_i128_to_balanced(value);
            assert_eq!(balanced_to_binary_i128(&ternary).unwrap(), value);
        }
        for value in [0, 1, 2, u64::MAX as u128, u128::MAX] {
            let ternary = binary_u128_to_balanced(value);
            assert_eq!(balanced_to_binary_u128(&ternary).unwrap(), value);
        }
        assert_eq!(
            balanced_to_binary_u128(&binary_i128_to_balanced(-1)),
            Err(TernaryError::NegativeUnsigned)
        );
    }

    fn metrics(cost: u128) -> CandidateMetrics {
        CandidateMetrics::new(cost, true, true, true)
    }

    #[test]
    fn internal_workload_selects_lowest_validated_cost() {
        let selected = select_representation(SelectionRequest {
            boundary: Boundary::Internal,
            candidates: CandidateSet {
                binary: Some(metrics(100)),
                ternary: Some(metrics(60)),
                hybrid: Some(metrics(80)),
            },
        })
        .unwrap();

        assert_eq!(selected.representation, Representation::BalancedTernary);
        assert_eq!(selected.estimated_cost, 60);
    }

    #[test]
    fn internal_tie_prefers_binary_deterministically() {
        let selected = select_representation(SelectionRequest {
            boundary: Boundary::Internal,
            candidates: CandidateSet {
                binary: Some(metrics(50)),
                ternary: Some(metrics(50)),
                hybrid: None,
            },
        })
        .unwrap();

        assert_eq!(selected.representation, Representation::Binary);
    }

    #[test]
    fn binary_boundary_uses_hybrid_when_compatible_and_no_more_costly() {
        let selected = select_representation(SelectionRequest {
            boundary: Boundary::ConventionalHardware,
            candidates: CandidateSet {
                binary: Some(metrics(100)),
                ternary: Some(metrics(20)),
                hybrid: Some(metrics(90)),
            },
        })
        .unwrap();

        assert_eq!(selected.representation, Representation::Hybrid);
        assert_eq!(
            selected.reason,
            SelectionReason::CompatibleHybridAtBinaryBoundary
        );
    }

    #[test]
    fn binary_boundary_falls_back_when_hybrid_costs_more() {
        let selected = select_representation(SelectionRequest {
            boundary: Boundary::NetworkWire,
            candidates: CandidateSet {
                binary: Some(metrics(70)),
                ternary: Some(metrics(10)),
                hybrid: Some(metrics(90)),
            },
        })
        .unwrap();

        assert_eq!(selected.representation, Representation::Binary);
        assert_eq!(
            selected.reason,
            SelectionReason::BinaryCompatibilityFallback
        );
    }

    #[test]
    fn incompatible_hybrid_is_never_selected_at_binary_boundary() {
        let selected = select_representation(SelectionRequest {
            boundary: Boundary::PersistentBinaryFormat,
            candidates: CandidateSet {
                binary: Some(metrics(80)),
                ternary: Some(metrics(10)),
                hybrid: Some(CandidateMetrics::new(1, true, true, false)),
            },
        })
        .unwrap();

        assert_eq!(selected.representation, Representation::Binary);
    }

    #[test]
    fn unsupported_or_unvalidated_candidate_is_never_selected() {
        let selected = select_representation(SelectionRequest {
            boundary: Boundary::Internal,
            candidates: CandidateSet {
                binary: Some(metrics(100)),
                ternary: Some(CandidateMetrics::new(1, false, true, true)),
                hybrid: Some(CandidateMetrics::new(2, true, false, true)),
            },
        })
        .unwrap();

        assert_eq!(selected.representation, Representation::Binary);
    }

    #[test]
    fn no_eligible_internal_candidate_returns_error() {
        let result = select_representation(SelectionRequest {
            boundary: Boundary::Internal,
            candidates: CandidateSet {
                binary: None,
                ternary: Some(CandidateMetrics::new(1, true, false, true)),
                hybrid: None,
            },
        });

        assert_eq!(result, Err(SelectionError::NoSupportedValidatedCandidate));
    }

    #[test]
    fn crypto_requires_validation_and_does_not_prefer_ternary_by_name() {
        let selected = select_representation(SelectionRequest {
            boundary: Boundary::Cryptographic,
            candidates: CandidateSet {
                binary: Some(metrics(10)),
                ternary: Some(CandidateMetrics::new(1, true, false, true)),
                hybrid: Some(metrics(20)),
            },
        })
        .unwrap();

        assert_eq!(selected.representation, Representation::Binary);
        assert_eq!(
            selected.reason,
            SelectionReason::ValidatedCryptographicCandidate
        );
    }

    #[test]
    fn crypto_rejects_unvalidated_or_incompatible_ternary_candidates() {
        let selected = select_representation(SelectionRequest {
            boundary: Boundary::Cryptographic,
            candidates: CandidateSet {
                binary: Some(metrics(40)),
                ternary: Some(CandidateMetrics::new(1, true, true, false)),
                hybrid: None,
            },
        })
        .unwrap();

        assert_eq!(selected.representation, Representation::Binary);
    }

    #[test]
    fn crypto_can_select_a_validated_ternary_candidate_on_evidence() {
        let selected = select_representation(SelectionRequest {
            boundary: Boundary::Cryptographic,
            candidates: CandidateSet {
                binary: Some(metrics(50)),
                ternary: Some(metrics(30)),
                hybrid: None,
            },
        })
        .unwrap();

        assert_eq!(selected.representation, Representation::BalancedTernary);
    }

    #[test]
    fn binary_boundary_fails_if_no_compatible_path_exists() {
        let result = select_representation(SelectionRequest {
            boundary: Boundary::ConventionalHardware,
            candidates: CandidateSet {
                binary: Some(CandidateMetrics::new(10, true, true, false)),
                ternary: Some(metrics(1)),
                hybrid: Some(CandidateMetrics::new(2, true, true, false)),
            },
        });

        assert_eq!(result, Err(SelectionError::NoCompatibleBinaryBoundaryPath));
    }
}
