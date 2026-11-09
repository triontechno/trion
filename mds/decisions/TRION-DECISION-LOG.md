# TRION — Consolidated Decision Record

**Location:** `mds/decisions/TRION-DECISION-LOG.md`  
**Repository:** `triontechno/trion`  
**Status:** Living source of truth; reconstructed from available prior conversations, project notes, and the current `main` documentation.  
**Purpose:** Preserve durable TRION decisions when earlier chats are deleted.

> **Record integrity:** This document distinguishes confirmed decisions from proposals, estimates, implementation status, and details that could not be recovered. It does not invent missing quiz answers. When a later explicit decision supersedes an earlier direction, the later decision governs.

---

## 1. Core TRION identity

- TRION is intended to be a genuinely new technology company and ecosystem.
- It is a **hardware + software + AI technology company**, not merely an app catalogue or an all-in-one application.
- The focus is automation, intelligent systems, and solving serious everyday problems with good solutions.
- Product proposals should have real customer value and defensible technical differentiation. Repackaging an existing product or adding superficial features is not sufficient.
- Hardware ambition: custom electronics, initially through prototypes; balanced ambition rather than either a purely theoretical or immediately mass-manufactured approach.

## 2. OS architecture — governing decision

**Approved direction: adaptive binary/ternary computing.**

TRION is neither binary-only with decorative ternary features nor ternary-only at the expense of compatibility. Each subsystem may choose binary, balanced ternary, unbalanced ternary, or a hybrid representation according to actual capabilities, correctness, compatibility, and measured total cost.

### Governing principles

1. Correctness and safety outrank representation choice.
2. Binary and ternary are both first-class design targets.
3. Ternary is not automatically faster, smaller, safer, or more energy-efficient.
4. Use ternary where it is supported, semantically correct, and demonstrably beneficial.
5. Use binary where it is required, better, or necessary for compatibility.
6. Keep conversions explicit, checked, and semantics-preserving.
7. Include conversion, memory, storage, latency, energy, and compatibility overhead in comparisons.
8. Never infer physical ternary hardware from software representations or emulation.
9. Preserve conventional binary interfaces at hardware, device, network, file-format, and ABI boundaries where those interfaces require binary layouts.
10. Unsupported capabilities must fail clearly or use a verified fallback; never silently claim support.

### Representation vocabulary

- **Balanced ternary (B3):** logical digits `{-1, 0, +1}`.
- **Unbalanced ternary (U3):** logical digits `{0, 1, 2}`.
- **Packed ternary:** ternary values encoded into ordinary binary bytes/words.
- **Physical ternary:** hardware whose physical elements natively represent three stable states. This is a future capability and must not be confused with software packing.

## 3. Twenty-question architecture review — recovered record

The 20-question quiz established the adaptive architecture baseline. The complete verbatim question text and every answer label are not present in the retained record. The table below preserves what is recoverable and explicitly marks gaps.

| Question | Recovered answer / decision | Record quality |
|---|---|---|
| Q1 | C — Adaptive hybrid architecture: use ternary wherever it is best; use binary wherever it is best or required. | Reconfirmed 2026-11-08 |
| Q2 | C — Both binary and ternary data representations; choose by data type and operation. | Reconfirmed 2026-11-08 |
| Q3 | C — Support both instruction types; choose according to hardware support and what works best. | Reconfirmed 2026-11-08 |
| Q4 | C — Adaptive registers and CPU state: native ternary where supported and beneficial; otherwise binary. | Reconfirmed 2026-11-08 |
| Q5 | C — Adaptive RAM: binary or packed ternary according to hardware, workload and efficiency. | Reconfirmed 2026-11-08 |
| Q6 | C — Support binary and packed-ternary storage, choosing the suitable format. | Reconfirmed 2026-11-08 |
| Q7 | C — Support existing binary hardware and future native ternary hardware through an adaptive architecture. | Reconfirmed 2026-11-08 |
| Q8 | Both directions, with optimization and compatibility (binary ↔ ternary). | Answer direction recorded |
| Q9 | Both binary and ternary executable formats. | Explicit answer recorded |
| Q10 | RAM / ternary-instruction-to-binary compatibility bridge was the subject of the answer. The exact original option wording is unavailable. | Partial |
| Q11 | Option D. The option text is unavailable. | Letter only |
| Q12 | “Whatever is best.” | Explicit answer recorded |
| Q13 | Option D. The option text is unavailable. | Letter only |
| Q14 | Option D. The option text is unavailable. | Letter only |
| Q15 | Option D, with `/compat`. | Explicit answer fragment |
| Q16 | User requested clarification comparing binary and ternary; no reliable final selection is retained. | Unresolved in record |
| Q17 | Option D. The option text is unavailable. | Letter only |
| Q18 | Option D. The option text is unavailable. | Letter only |
| Q19 | “Dream later” — defer the long-term evolution/adoption decision rather than force it now. | Explicit answer recorded |
| Q20 | E / AOTA — “All Of The Above.” The combined ambition includes ternary as a practical alternative, a fully ternary OS, hybrid computing beyond binary-only systems, and an adaptive ecosystem. | Explicit answer recorded |

**Important:** Option letters without their original option text are preserved as letters only. They must not be reverse-engineered or guessed. The architecture decisions below are the usable normative record.

## 4. Approved subsystem directions

| Area | Decision |
|---|---|
| Overall OS | Fully adaptive hybrid binary/ternary architecture |
| Kernel | Representation-neutral interfaces; subsystem-level adaptive choices |
| CPU / instruction strategy | Use supported binary or ternary operations according to target capabilities and measured benefit; distinguish native instructions from emulation |
| Registers | Adaptive logical register representation; do not claim commodity physical CPU registers are ternary |
| Languages | Continue supporting established binary-oriented languages, including C, C++, and Rust; provide a path for a native ternary language |
| Compiler | Adaptive compiler that considers target, workload, semantics, support, compatibility, and measured total cost |
| Executable formats | Support both binary and ternary formats as a design goal; current application architecture specifically loads supported original binary formats directly |
| Application execution | One direct executable engine for supported original application formats; no application translation pipeline |
| RAM | Ternary-oriented logical memory where beneficial, backed/bridged to conventional binary RAM when required |
| Storage | Explore ternary encodings and packing; retain ordinary binary device and file interfaces |
| Internal data | Choose representation by data type, workload, hardware, and cost |
| Mathematics | Adaptive calculation path; choose the best verified implementation per operation |
| GPU | Use what the GPU actually supports and what benchmarks justify; do not assume native ternary GPU execution |
| Networking | Binary at conventional interfaces and wire protocols; ternary internal processing only where useful |
| Security / cryptography | Use validated, secure algorithms; preserve established binary cryptographic compatibility |
| Long-term transition | Deliberately deferred (Q19) |
| Overall ambition | AOTA (Q20): all four strategic directions listed above |

## 5. Translation and compatibility decisions

### Earlier design direction

The quiz captured interest in bidirectional binary/ternary semantic translation and a compatibility bridge so ternary-oriented operations could interoperate with binary hardware.

### Later superseding application-execution decision

The application translator architecture was subsequently retired.

- **No application-to-TIR translation.**
- No TIR conversion as an application execution route.
- No `.trx` application container.
- No generated intermediate executable.
- No translation cache.
- No TRIONEX application execution path.
- Original application files remain unchanged.
- Supported original executable formats are loaded directly by the direct executable engine.
- Parsing or mapping executable bytes is not itself execution.
- Unsupported formats, instructions, APIs, and ABI requirements fail closed.

The bidirectional bridge remains a representation/data compatibility concept where explicitly implemented; it is **not** authorization to revive an application translator.

### TIR

- TIR is a standalone research/test runtime.
- TIR is not an application executable format, application loader, or fallback execution path.
- Stage C may retain its bounded interpreter foundation for research and tests.
- TIR must not be used to translate user applications.

## 6. Memory decisions

- The selected direction is ternary-oriented logical memory with a binary compatibility bridge for conventional hardware.
- Five balanced trits are packed into one ordinary byte in the current Stage B representation.
- Mapping: `-1 → 0`, `0 → 1`, `+1 → 2`.
- Packing formula: `byte = d0 + 3*d1 + 9*d2 + 27*d3 + 81*d4`.
- Values 0–242 are valid; 243–255 are reserved.
- Logical trit count is stored separately; unused positions in a partial final byte use canonical zero trits.
- This is **logical ternary data in binary storage**, not physical ternary RAM, ternary DRAM, ternary paging, DMA, or a filesystem format.
- A previously stated intuition that “40 GB in binary is 25 GB in ternary” is **not a proven compression ratio or capacity result**. Any such claim requires a defined encoding, workload, metadata accounting, and measurements.

## 7. Hardware and platform boundaries

- Commodity x86-64/ARM64 hardware is assumed to be binary unless actual capability discovery proves otherwise.
- Do not claim native ternary CPU instructions, registers, RAM, or GPU execution without hardware evidence.
- Hardware-facing interfaces, MMIO, DMA, syscalls, driver ABIs, conventional storage and network protocols retain their required binary layouts.
- Capability discovery must be conservative. Unknown or unprobed hardware is not treated as ternary-capable.
- Future custom TRION hardware may add native ternary capabilities; software must distinguish that from emulation and packed representations.

## 8. Direct application engine decisions

- Stage G is the single application execution path for supported original PE32+ `.exe` and Mach-O64 `.app` executables.
- Guest code must execute in user mode, never kernel mode.
- Original files must not be rewritten or translated.
- W^X is required; writable-executable mappings are prohibited.
- Unsupported imports, APIs, runtime features, architectures, or ABI requirements fail closed.
- Current declared native profiles are restricted freestanding x86-64 profiles; they do **not** establish general Windows or macOS compatibility.
- Stage I owns broader Windows runtime services.
- Stage J owns broader macOS runtime services and bundle integration.
- Stage H owns guest isolation, capability enforcement, resource controls, and containment.

## 9. Phase and stage decisions

### Phase 2

- Phase 2 is the adaptive binary/ternary computing implementation phase.
- Stage A: adaptive representation foundation.
- Stage B: packed ternary logical memory.
- Stage C: bounded TIR research/interpreter foundation.
- Stage D: retired by architecture decision; no translator or TRIONEX application path.
- Stage E: runtime, kernel, hardware capability and initfs integration.
- Stage F: measurement, hardening and release infrastructure.
- Stage G: direct executable engine.
- Stage H: isolated guest process backend and security containment.
- Stage I: Windows compatibility runtime.
- Stage J: macOS compatibility runtime and unified dispatch.

### Phase 2 validation gate

**Do not run tests, builds, benchmarks, fuzz campaigns, or QEMU validation until all Phase 2 implementation stages are complete.** Test and fuzz source files may be authored, but execution remains deferred. After implementation is complete, run the consolidated validation pass and report actual results.

Implementation status must never be confused with validation status. A stage marked implemented but untested is not verified.

### Later phases

- Phase 6 is the final project phase, focused on UI/UX/UC (desktop shell, interaction, accessibility, app discovery/launch/file associations, settings, notifications, recovery and acceptance criteria).
- Phase 6 was formerly called Phase H; it is distinct from Phase 2 Stage H.
- No confirmed specifications for Phases 3–5 are available. Do not invent them.

## 10. Stage H security decisions and current boundary

Stage H must provide real kernel-enforced isolation, not merely a portable model.

Required outcomes:
- Kernel-enforced per-process guest syscall policy.
- Explicit filesystem, network, device and IPC capability allowlists.
- Enforcement over inherited, duplicated and newly created descriptors/handles.
- Brokered I/O with no generic syscall route that bypasses the broker.
- Kernel-enforced CPU, memory and file-descriptor accounting/limits.
- Safe guest thread creation, cancellation, descendants and teardown policy.
- Structured fault details for the launcher.
- Audited cleanup for partial loads and process failures.
- Isolation integrated with any broader platform runtimes added by Stages I/J.

**Current known status:** native process hand-off, real Redox address spaces/user contexts, scheduler integration, dedicated restricted guest exit, a deny-all syscall baseline, initial address-space/descriptor/CPU quotas, signal-handler reset, exception-to-signal mapping, and existing process teardown are present. Explicit filesystem/network/device/IPC capability enforcement, brokered I/O, guest-specific structured fault reports, complete descendant/thread policy, and a comprehensive cleanup audit remain incomplete. Stage H must remain **N** until the security boundary is implemented. Do not run untrusted applications under the restricted native profile as though it were already a complete sandbox.

## 11. Repository and collaboration decisions

- Repository: [github.com/triontechno/trion](https://github.com/triontechno/trion).
- Work directly on `main` when practical.
- If a separate branch is necessary, merge completed and verified work into `main`; do not leave completed work only on a feature branch.
- Preserve existing Redox-origin documentation and code ownership context when reorganizing TRION-authored material.
- TRION-authored Markdown documentation is organized under `mds/`; the root `README.md` remains the repository entry point.
- Stage and implementation reports must reflect actual code, not aspirations.
- Never claim a change, commit, merge, test, build, benchmark, fuzz run, or QEMU validation occurred unless it actually did.

## 12. Product and roadmap constraints

- TRION's differentiation must be technical and grounded in real problems.
- Prefer genuinely new hardware/software/AI systems and automation over superficial feature additions or catalogues of existing products.
- Prototype custom electronics before assuming manufacturing readiness.
- **PackSense is permanently discontinued.** Do not recommend, include, or revive it in TRION roadmaps unless the user explicitly reverses this decision.

## 13. Current implementation snapshot (not a new design decision)

This section is a point-in-time record and should be updated as code changes.

| Phase 2 stage | Known status | Important qualification |
|---|---|---|
| A | Implementation delivered | Validation deferred |
| B | Implementation delivered | Logical packed memory only; validation deferred |
| C | Interpreter foundation delivered | Standalone research/test runtime; validation deferred |
| D | Retired | Translator/application TIR path discontinued |
| E | Implementation delivered | Validation deferred |
| F | Infrastructure authored | Benchmarks/fuzz/build workflows not executed |
| G | Implemented for restricted freestanding x86-64 profiles | No general Windows/macOS compatibility; validation deferred |
| H | Incomplete | Base deny-all guest policy and quotas exist; capability broker and containment remain |
| I | Incomplete | General Windows runtime services missing |
| J | Incomplete | General macOS runtime services missing |

No Phase 2 tests, builds, fuzz campaigns, benchmarks, or QEMU validation have been run under the standing deferral rule.

## 14. Open items and evidence gaps

1. Recover the verbatim wording and exact option text for Q1–Q7 and Q11/Q13/Q14/Q17/Q18 if the original quiz transcript is ever available.
2. Recover the final Q16 choice, if one was made after the clarification request.
3. Recover the exact original Q10 and Q15 option text; retain only the answer fragments currently known.
4. Define hardware capability discovery evidence for any future native ternary CPU, RAM, or GPU.
5. Define workload-specific benchmarks before claiming ternary performance, density, energy, or storage wins.
6. Complete Stage H kernel-enforced isolation before treating native guest applications as sandboxed.
7. Complete Stages I and J before claiming broad Windows/macOS application compatibility.
8. Run the consolidated validation pass only after all Phase 2 implementation is complete.

## 15. Change-control rule for this record

This file is the durable decision log, not a substitute for the detailed technical specifications in `mds/architecture/`, `mds/ternary/`, and `mds/phases/`.

When a decision changes:
1. Update the relevant normative section here.
2. Record the date, previous decision, new decision, and reason in the history below.
3. Update affected technical specifications and implementation plans.
4. Do not silently erase superseded decisions; mark them superseded.

### Decision history

- **2026-11-08:** Initial consolidated decision record created from retained project memory, prior-conversation summaries, and the current `main` architecture/phase documents. Missing quiz option text and unresolved answers are explicitly identified rather than inferred.
- **2026-11-08:** Re-ran Q1–Q7 interactively and recorded the user's explicit answers (all C) with their confirmed meanings. Q8–Q20 retained from the recovered quiz record; Q16 remains unresolved and unavailable option wording remains explicitly unknown.
