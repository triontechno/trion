# TRION OS — Phase 3: Core Platform and Hardware Excellence

**Status:** In progress — P3-A repository and Phase 2 closeout audit. P3-B through P3-G remain gated on verified Phase 2 closeout.  
**Parent roadmap:** ROADMAP.md  
**Architecture:** ../architecture/TRION-ARCHITECTURE.md  
**Requirements:** ../architecture/CROSS-PLATFORM-REQUIREMENTS.md

## 1. Mission

Build the dependable platform layer that turns Phase 2's native execution foundation into a practical general-purpose OS.

Phase 3 prioritizes reliability, device breadth, data integrity, networking, security, updates, recovery and measurable performance. It is not the final desktop-polish phase or an AI-first phase.

## 2. Baseline and constraints

The current Phase 2 plan reports Stages A–C and E–J implemented to their declared scopes, with Stage D retired. Its consolidated validation gate was deferred. These claims are not verified until source and CI are reviewed.

Mandatory constraints:
- Preserve adaptive binary/ternary architecture and native Redox interfaces.
- Do not revive application translation, TIR application execution, TRIONEX or generated intermediate executables.
- Do not claim Windows/macOS/Linux compatibility from restricted PE/Mach-O profiles or the native Redox ELF ABI.
- Conventional MMIO, DMA, block, network and driver interfaces remain binary unless real hardware says otherwise.
- Use capability security and fail-closed behavior.
- No ads, forced cloud account, forced telemetry or undeletable promotional apps.
- Preserve upstream licenses and isolate TRION-specific changes.

## 3. Implementation workstreams

### P3-A — Repository and Phase 2 closeout audit
1. Reconcile Phase 2 stage documents against actual source and merged history.
2. Inspect PR #30 and its CI result; confirm whether the preflight regression guard is merged and whether the full image build passes.
3. Identify pending Phase 2 tests/builds/fuzz/benchmark/QEMU checks.
4. Run consolidated Phase 2 validation under the approved gate; record commands, commit SHA, environment and results.
5. Fix blockers without weakening or deleting unrelated tests.
6. Publish a Phase 2 closeout report and known-limitations list.

**Gate:** no Phase 3 platform API may assume unverified Phase 2 behavior.

### P3-B — Hardware abstraction and device lifecycle
1. Inventory Redox device, PCI, ACPI and hardware-manager interfaces.
2. Define device classes, identifiers, capability descriptors and versioned driver contracts.
3. Define discovery, attach/detach, hot-plug, power state, reset and failure states.
4. Keep privileged MMIO/DMA in trusted kernel/driver contexts.
5. Add signed-driver/trust metadata where the boot/security chain can enforce it.
6. Create device compatibility report and reference-hardware matrix.
7. Provide a future native-ternary capability extension point without fabricating support.

**Deliverables:** hardware/driver specification, device-manager integration, capability schema, test fixtures and supported-device matrix.

### P3-C — Storage and filesystem reliability
1. Audit Redox schemes, filesystem drivers, block interfaces and mount flow.
2. Define stable storage service and mount/namespace policy.
3. Implement safe removable-media lifecycle and error reporting.
4. Add transactional/atomic file operations where supported.
5. Define integrity checking, crash-consistency guarantees and recovery tools.
6. Integrate encryption through reviewed providers and hardware-backed key storage where available.
7. Design snapshot/rollback interfaces; do not invent unsupported filesystem features.
8. Keep conventional binary disk formats and interfaces as default.

**Deliverables:** storage architecture, filesystem reliability matrix, recovery utilities and fault-injection tests.

### P3-D — Network platform
1. Audit existing Redox network stack and device drivers.
2. Define network manager service and configuration API.
3. Integrate Ethernet/Wi-Fi where drivers support them.
4. Establish IPv4/IPv6, TCP/UDP, DNS, DHCP and TLS via reviewed components.
5. Define per-app network permission and firewall integration.
6. Add connection diagnostics and safe credential storage.
7. Keep wire protocols binary-compatible.

**Deliverables:** network service, configuration interface, protocol coverage report and security tests.

### P3-E — Security, identity and privacy
1. Define user/session identity and service identity boundaries.
2. Standardize capability grants for files, IPC, network and devices.
3. Audit privilege transitions and privileged service attack surfaces.
4. Integrate secure-boot evidence and driver/package trust where available.
5. Define secrets storage and disk-encryption integration.
6. Make audit events structured, inspectable and privacy-conscious.
7. Make telemetry opt-in; local setup and core operation work offline.
8. Document threat model, recovery behavior and security limitations.

**Deliverables:** security model, permission API, audit schema, threat model and conformance tests.

### P3-F — Package, update and recovery
1. Define signed package metadata and repository format.
2. Implement dependency resolution and conflict detection.
3. Make install/remove/upgrade transactional.
4. Integrate signature and hash verification.
5. Design atomic OS update and known-good boot selection using supported boot/storage mechanisms.
6. Provide rollback and offline recovery/import workflows.
7. Keep user data safe across interrupted updates.
8. Do not introduce ads, sponsored search or forced bundles.

**Deliverables:** package manager, repository tooling, updater, rollback path and recovery environment.

### P3-G — Resource management and performance
1. Add observability for CPU, memory, storage, network, thermal and power where hardware exposes it.
2. Define resource budgets for system services.
3. Measure boot, idle RAM/CPU, launch, filesystem and network latency on fixed reference hardware.
4. Add memory-pressure reporting and predictable reclaim behavior.
5. Audit background services and startup work.
6. Feed measured target capabilities into adaptive binary/ternary policy.
7. Compare ternary/hybrid paths against binary baselines including conversion cost.

**Deliverables:** telemetry interfaces, benchmark suite, reference baselines and regression thresholds.

## 4. Dependency order

1. P3-A Phase 2 closeout and repository audit.
2. P3-B hardware/device contracts.
3. P3-C storage and P3-D networking, coordinated with device interfaces.
4. P3-E security/identity contracts across services.
5. P3-F package/update/recovery after storage and trust foundations.
6. P3-G instrumentation throughout; final baseline after integration.

Security and test design begin alongside each workstream, not at the end.

## 5. Global implementation rules

- Prefer small, independently reviewable changes.
- Preserve existing tests and add focused regression tests.
- Do not run destructive scripts or remove unrelated workflows.
- Do not change upstream interfaces without documenting why.
- Use exact capability detection; unknown means unsupported.
- Test update/rollback with interrupted operations.
- Every privileged service requires a threat model and least-privilege policy.
- CI runs fast static/preflight checks early; expensive integration/image checks are separate.
- QEMU is a validation runner, not a prerequisite for producing an image.
- Record measured build times; do not promise targets until measured.

## 6. Phase 3 exit criteria

Phase 3 is complete only when:
- Device discovery/lifecycle work on declared reference hardware.
- Driver/device permissions are enforced and tested.
- Filesystem/storage guarantees are documented and fault-tested.
- Networking has documented protocol coverage and security tests.
- User/session and capability permissions have regression coverage.
- Package operations verify signatures and are transactional.
- OS updates recover to a known-good state after interruption.
- Performance/resource baselines exist for every declared reference profile.
- Offline setup/core operation work without cloud login.
- System UI and bundled apps contain no ads or sponsored placements.
- Release blockers are fixed or explicitly scoped out with documented reasons.

## 7. First implementation task

Start with **P3-A**, not speculative feature coding: inspect PR #30, current main, Phase 2 validation workflow and latest image-build results. Produce a concrete source-based gap list. Implement remaining Phase 2 blockers first. Then build Phase 3 against actual Redox interfaces found in the repository, not invented APIs.
