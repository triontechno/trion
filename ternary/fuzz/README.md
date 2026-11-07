# Phase 2 fuzzing harness

Fuzz targets are source-only during Phase 2 implementation. Do not execute campaigns until the consolidated post-Phase-2 validation pass.

## Targets

- packed_trit_buffer: malformed packed ternary data and bounded logical lengths.
- trn3_deserialize: arbitrary TRN3 streams and canonical serialization.
- pe32_plus: hostile PE32+ headers, sections, imports, image mappings and truncation boundaries.
- macho64: hostile Mach-O 64 headers, load commands, segments, dependencies and entry-point metadata.
- app_bundle_plist: hostile XML Info.plist data and executable-name selection.

## Run after Phase 2 implementation

Install cargo-fuzz with nightly Rust. From ternary/fuzz:

    cargo fuzz run packed_trit_buffer -- -max_total_time=60 -max_len=1048584
    cargo fuzz run trn3_deserialize -- -max_total_time=60 -max_len=1048576
    cargo fuzz run pe32_plus -- -max_total_time=60 -max_len=1048576
    cargo fuzz run macho64 -- -max_total_time=60 -max_len=1048576
    cargo fuzz run app_bundle_plist -- -max_total_time=60 -max_len=1048576

Preserve corpus, crash artifacts, toolchain versions, duration, and coverage output. These targets are not evidence of a completed security review until they have actually run and findings have been resolved.
