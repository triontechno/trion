# TRION Phase 2 — Stage J: macOS Compatibility Runtime and Dispatch

**Status: restricted freestanding Mach-O launch and XML bundle dispatch implemented; general macOS runtime remains incomplete.**

## Selected compatibility boundary

The current profile is limited to little-endian x86-64 `MH_EXECUTE` images with `LC_MAIN`, no dynamic dependencies or runtime-dependent load commands, only `argv[0]`, and an empty environment. `.app` dispatch accepts bounded XML `Info.plist` metadata and launches the declared executable. A small OS-owned x86-64 adapter constructs a Darwin-style initial stack (`argc`, `argv[0]`, and empty `envp`/Apple vectors), calls `LC_MAIN` with those four startup arguments, and sends the returned status through authenticated guest exit. This supplies neither dyld nor CRT initialization, TLS, or system libraries; only freestanding main-entry images built for that restricted contract are in scope. This is not compatibility with a particular macOS release or general macOS application support.

## Implemented source
- Broker-fed bundle-member path validation rejecting absolute paths, traversal, symlinks and special files.
- Bounded member counts and resource sizes.
- Bundle executable selection from bounded XML Info.plist metadata.
- COSMIC Files resolves .app/Contents/MacOS and routes the original Mach-O executable through Redox's native process/address-space path.
- Native Mach-O64 loader maps the declared freestanding LC_MAIN segments with W^X and rejects dyld, dylib, chained-fixup, signing and runtime-dependent commands.
- The OS-owned entry adapter constructs the bounded Darwin-style stack vectors, calls `LC_MAIN` using the x86-64 C calling convention, and converts its return value into restricted guest exit. It intentionally does not provide dyld initialization, TLS, general arguments/environment, or CRT services.
- Library-level unified dispatch for original PE32+ files, Mach-O files and .app bundles, with bounded argument/environment validation and explicit compatibility blockers.
- Explicit unresolved-framework diagnostics.

## Remaining
- Binary Info.plist parsing and full bundle/resource semantics.
- Mach-O relocation, dyld rebasing/binding, chained fixups and symbol resolution.
- Dependency/framework catalog, version and code-signature/entitlement policy.
- Full Darwin x86-64 process startup, dyld/CRT runtime, TLS, system libraries, and general argument/environment construction.
- Kernel-backed selected system and framework services.
- Exact runtime blocker reporting and compatibility matrix based on actual post-Phase-2 validation.

No general macOS application compatibility is claimed.
