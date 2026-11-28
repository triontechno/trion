# Phase 3 P3-A — Repository and Phase 2 closeout audit

**Audit baseline:** `main` at `617e544e637b8e6dc125674370c19fd30ab5b466` (2026-11-26)  
**Status:** In progress; this report records verified repository and CI evidence, not Phase 2 completion.

## 1. PR #30 — preflight regression guard

- PR #30, “Add regression guard for redox-rt vec macro build failure”, is merged.
- Merge commit: `68ab29f33822d7bda32de897dcdfc206c74ea54b`.
- Its stated scope was a fast SimpleBuild preflight regression check and removal of repeated-failure image/ISO generation workflows.
- The PR's validation intentionally did not build an image or ISO. Therefore PR #30's successful SimpleBuild validation is not evidence of a successful OS image build.

## 2. Main-branch Phase 2 validation

- Main commit `617e544e637b8e6dc125674370c19fd30ab5b466` has a successful TRION Phase 2 Validation run: [run 45766843943](https://github.com/triontechno/trion/actions/runs/45766843943).
- The run's three jobs passed: Rust tests/formatting/lint, bounded fuzz campaigns, and final report.
- The workflow currently executes host-crate tests, formatting, Clippy, the comparative benchmark correctness gate, and bounded fuzz campaigns.
- It does **not** build a Redox desktop disk image/live ISO and does **not** perform a QEMU boot test. The Phase 2 Stage F documentation currently describes those as workflow coverage, so that documentation is ahead of the checked-in workflow implementation.

## 3. Main-branch SimpleBuild result

- On the same main commit, SimpleBuild Prerequisite Validation run [45766843928](https://github.com/triontechno/trion/actions/runs/45766843928) failed.
- It did not produce a successful prerequisite artifact for the split ISO workflow.
- PR #34 is the separate attempt to restore prerequisite caching plus ISO generation; its ISO build failed on commit `16d7b4133218b9a83eb816530427c1452e1f85c8`. The detailed log shows the failure surfaced as the Rust `async_fn_in_trait` warning during the containerized image build. The current investigation found that `simplebuild/build.sh` exports `RUSTFLAGS=-A async_fn_in_trait`, but `build/mk/podman.mk` did not forward `RUSTFLAGS` into the Podman container.
- A focused fix has been committed to the PR #34 branch: Podman now inherits the host `RUSTFLAGS`. Its rerun is pending; no successful ISO is claimed.

## 4. Workflow inventory at the audit baseline

The checked-in workflow directory contains only:
- `.github/workflows/phase2-validation.yml`
- `.github/workflows/simplebuild.yml`

The Phase 2 Stage F document references separate `trion-release.yml` and `release-iso.yml` workflows, but neither exists at this baseline. This discrepancy must be reconciled before Phase 2 can be declared closed.

## 5. Phase 2 closeout gaps

| Gate | Evidence | Status |
|---|---|---|
| Host Rust tests, formatting and Clippy | Main Phase 2 validation run | Passed |
| Bounded fuzz campaigns | Main Phase 2 validation run | Passed |
| Comparative benchmark correctness | Included in passing Rust job | Passed |
| Actual x86_64 desktop image + live ISO build | No successful run/artifact verified | Open |
| QEMU boot and Phase 2 service smoke tests | Not present in current Phase 2 workflow | Open |
| Phase 2 Stage F documentation vs workflow inventory | Referenced release workflows absent | Open |
| SimpleBuild prerequisite build/cache | Main run failed; PR #34 fix under test | Open |

## 6. Next actions

1. Complete PR #34's focused Podman environment fix and obtain a successful ISO artifact. If it fails again after the corrected environment is tested, follow the agreed fallback: remove SimpleBuild and move ISO generation into the Phase 2 build path.
2. Restore or implement the documented image/QEMU validation gates without weakening the existing Rust/fuzz checks.
3. Reconcile Phase 2 Stage F documentation with actual workflow files and verified artifacts.
4. Publish the final Phase 2 closeout report only after all required gates have evidence.
5. Keep P3-B hardware/device API design blocked until the Phase 2 closeout gate is met. P3-A source inventory and repository audit may proceed in parallel.

## 7. Scope discipline

This branch starts Phase 3 through P3-A only. It does not introduce speculative hardware, storage, network, package or security APIs before the Phase 2 execution foundation is verified.
