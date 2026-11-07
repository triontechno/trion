# TRION Phase 2 — Stage J: macOS Compatibility Runtime and Dispatch

**Status: bundle-planning foundation implemented; Mach-O runtime and unified launcher not implemented.**

## Implemented source
- Broker-fed bundle-member path validation rejecting absolute paths and traversal.
- Bounded member counts and resource sizes.
- Bundle executable planning from bounded XML Info.plist metadata.
- Deterministic resource list construction.
- Explicit unresolved-framework diagnostics.

## Remaining
- Binary Info.plist parsing and full bundle/resource semantics.
- Mach-O relocation, dyld rebasing/binding, chained fixups and symbol resolution.
- Dependency/framework catalog, version and code-signature/entitlement policy.
- macOS x86-64 process startup and ABI.
- Kernel-backed selected system and framework services.
- Unified launcher/file-manager dispatch for PE and Mach-O with exact blocker reporting.
- End-to-end compatibility matrix based on actual post-Phase-2 validation.

No general macOS application compatibility is claimed.
