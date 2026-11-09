# TRION Phase 2 — Stage J: macOS Compatibility Runtime and Dispatch

**Status: restricted freestanding Mach-O launch and XML bundle dispatch implemented; general macOS runtime remains incomplete.**

## Selected compatibility boundary

The current profile is limited to little-endian x86-64 `MH_EXECUTE` images with `LC_MAIN`, no dynamic dependencies or runtime-dependent load commands, and an empty argument/environment block. `.app` dispatch accepts bounded XML `Info.plist` metadata and launches the declared executable. This is a deliberately constrained Mach-O execution profile, not compatibility with a particular macOS release or general macOS application support.

## Implemented source
- Broker-fed bundle-member path validation rejecting absolute paths, traversal, symlinks and special files.
- Bounded member counts and resource sizes.
- Bundle executable selection from bounded XML Info.plist metadata.
- COSMIC Files resolves .app/Contents/MacOS and routes the original Mach-O executable through Redox's native process/address-space path.
- Native Mach-O64 loader maps the declared freestanding LC_MAIN segments with W^X and rejects dyld, dylib, chained-fixup, signing and runtime-dependent commands.
- Library-level unified dispatch for original PE32+ files, Mach-O files and .app bundles, with bounded argument/environment validation and explicit compatibility blockers.
- Explicit unresolved-framework diagnostics.

## Remaining
- Binary Info.plist parsing and full bundle/resource semantics.
- Mach-O relocation, dyld rebasing/binding, chained fixups and symbol resolution.
- Dependency/framework catalog, version and code-signature/entitlement policy.
- Standard macOS x86-64 process startup, ABI and environment/argument blocks.
- Kernel-backed selected system and framework services.
- Exact runtime blocker reporting and compatibility matrix based on actual post-Phase-2 validation.

No general macOS application compatibility is claimed.
