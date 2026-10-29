# TRION Phase 1 — Ternary Foundations and Translator Contract

**Status:** Proposed engineering specification  
**Scope:** Phase 1 only: define representation semantics, the binary/ternary boundary, and the translator's initial contract.  
**Audience:** GHC and contributors implementing the next phase.

The terms MUST, MUST NOT, SHOULD, and MAY are normative.

## 1. Objective

TRION supports both binary and ternary representations on conventional x86-64 and ARM64 hardware. Ternary should be used where its representation or semantics are useful; binary remains available where it is more efficient, required for compatibility, or required by hardware interfaces.

Phase 1 defines interoperable rules before implementation expands. It does not claim that ternary automatically saves energy, that current CPUs execute native ternary instructions, or that the translator already translates whole executables.

**Governing rule:** Preserve program semantics and safety first. Select a representation based on explicit requirements and measured total cost, including storage, conversion, execution, memory use, and compatibility.

## 2. Terms and value domains

### 2.1 Trit

A trit is a digit in a base-3 representation. TRION distinguishes two domains and MUST NOT silently interchange them:

- **Unbalanced trit:** U3 = {0, 1, 2}. This is the canonical digit domain for dense positional packing.
- **Balanced trit:** B3 = {-1, 0, +1}. This is a signed logical/arithmetic domain.

Conversion between domains MUST be explicit. For a single digit, balanced = unbalanced - 1 and unbalanced = balanced + 1. This digit mapping does not define how a multi-trit signed integer is encoded; that requires a separately specified conversion routine.

### 2.2 Ternary word versus machine word

A **ternary word** is a sequence of trits with a declared count and encoding. A **machine word** is a binary integer such as u32 or u64. A packed ternary word is stored inside a binary machine word; this does not mean the physical CPU has ternary registers or instructions.

### 2.3 Representation choice

A component MUST document the representation of values crossing its public interfaces. A component MAY use binary internally even when adjacent components use ternary. Representation selection MUST NOT change the observable meaning of a value.

## 3. Canonical packed representation

### 3.1 Positional encoding

For N unbalanced trits t[i], each in {0,1,2}, the packed unsigned value is:

V = sum(t[i] × 3^i), for i = 0 through N-1.

Trit index zero is the least-significant trit. Encoding and decoding MUST use this order unless a separately versioned format explicitly states otherwise.

### 3.2 Capacity limits

- u32: at most 20 unbalanced trits, because 3^20 = 3,486,784,401 <= 2^32 - 1, while 3^21 > 2^32 - 1.
- u64: at most 40 unbalanced trits, because 3^40 <= 2^64 - 1, while 3^41 > 2^64 - 1.

Implementations MUST NOT assume every u32 or u64 value is valid for a given trit count. For declared count N, valid values are in [0, 3^N - 1]. Values outside that range are invalid and MUST be rejected by checked decoding, not silently normalized.

### 3.3 Required core operations

The initial core API SHOULD provide:

- checked encoding from a slice of unbalanced trits into u32 or u64;
- checked decoding from a packed word and explicit trit count;
- validation of packed-word range for a trit count;
- explicit conversion of one trit between unbalanced and balanced domains;
- conversion of a documented signed integer format to/from a documented trit sequence;
- comparison of decoded sequences or packed values when their trit counts are equal.

Encoding MUST reject a trit outside {0,1,2} and a sequence longer than target capacity. Decoding MUST reject invalid values and impossible counts. Checked public operations MUST return errors for malformed user input rather than panic.

### 3.4 Arithmetic and overflow

Phase 1 MUST NOT imply one universal overflow rule for every representation.

- Checked arithmetic MUST report overflow as an error.
- Wrapping/modular arithmetic, if provided, MUST use a separately named API and document its modulus.
- Saturating arithmetic, if provided, MUST use a separately named API.
- Balanced signed arithmetic and unbalanced positional encoding MUST have separate documented semantics.

No operation may silently switch between checked, wrapping, and saturating behavior.

### 3.5 Byte order and serialization

The numeric value of a packed word is independent of host byte order. Any serialized format MUST specify byte order explicitly; the recommended default for multi-byte packed words is little-endian. Readers MUST validate the declared trit count and encoded value before exposing decoded data.

Persistent formats MUST include enough version, type, and count information to decode safely. A raw packed integer without a declared trit count is not a self-describing serialized ternary value.

## 4. Binary/ternary bridge contract

### 4.1 Boundary rules

A bridge function MUST declare:

- input and output types and representations;
- trit count or length metadata where applicable;
- signedness and overflow behavior;
- ownership and lifetime of buffers;
- error behavior for malformed values;
- whether conversion copies data or borrows it.

Conversions SHOULD happen at clear interface boundaries. Implementations SHOULD avoid repeated pack/unpack cycles by keeping data in a stable representation for a workload. Batch conversion MAY be provided where it improves measured throughput.

### 4.2 Native binary path

Hardware-facing operations—including MMIO registers, DMA descriptors, device-driver interfaces, network packets, and conventional file-format bytes—MUST retain the exact binary layouts required by their interfaces. Ternary encoding MUST NOT be applied to such buffers unless the interface explicitly defines a ternary-aware format.

### 4.3 Semantic preservation

A bridge MUST preserve values exactly or return a documented error when exact conversion is not possible. It MUST NOT silently truncate, reinterpret signedness, discard invalid encodings, or change floating-point, atomic, memory-ordering, or exception semantics.

### 4.4 Security

Bridge code MUST validate lengths and ranges before reading or writing buffers. It MUST use checked arithmetic for size calculations and handle malformed input without panic, out-of-bounds access, or undefined behavior. Data from files, executables, devices, or applications MUST be treated as untrusted unless independently validated.

## 5. Translator Phase 1 contract

### 5.1 Current scope

The existing translator frontend is an ELF inspection and x86-64 instruction-analysis prototype. Phase 1 does not claim full binary-to-ternary translation or production application compatibility.

The initial translator contract is to:

1. identify supported ELF metadata and architecture;
2. decode only instructions covered by its selected decoder and analysis scope;
3. classify findings transparently;
4. report unsupported or undecodable cases explicitly;
5. never label a binary as translated unless valid translated output was actually produced and verified.

### 5.2 Classification

The initial conceptual classifications are:

- **Native binary:** preserve execution or representation in its original binary form.
- **Ternary candidate:** an operation or data value that may use ternary representation after semantics and performance are established.
- **Bridge required:** a defined conversion or interface boundary is necessary.
- **Unsupported/unknown:** behavior is not yet modeled; the translator MUST NOT guess.

These are specification-level categories, not a promise that all categories are implemented today. Instruction classification alone is not code generation.

### 5.3 Translation and execution are separate capabilities

The following stages MUST be reported separately:

1. ELF parsing;
2. instruction decoding and analysis;
3. ternary data encoding;
4. semantic instruction translation;
5. native code generation or VM/interpreter output;
6. executable validation and execution.

Completing one stage MUST NOT be described as completing a later stage. If no translated executable or defined executable representation is emitted, the tool is an analysis frontend, not an executable translator.

### 5.4 Compatibility contract

Any future translator implementation MUST specify the supported source architecture, instruction subset, ABI, system-call strategy, relocation handling, exception behavior, and fallback policy. Unsupported instructions MUST cause a clear diagnostic or take a verified fallback path; they MUST NOT be silently dropped or approximated.

A fallback MUST preserve source program semantics. If semantic preservation cannot be guaranteed, the translator MUST stop with a clear error.

### 5.5 Proposed annotations

The spellings native_binary, ternary_logic, and trit_bridge are proposed TRION concepts, not assumed to be compiler-standard attributes. Before implementation, the project MUST choose concrete syntax and define:

- which compiler or intermediate representation recognizes it;
- whether it applies to a function, type, module, or expression;
- conflict and inheritance rules;
- behavior when a target compiler does not support it;
- diagnostics and testing requirements.

An annotation MUST NOT be treated as implemented until a compiler path consumes it and tests verify its behavior.

## 6. Representation-selection policy

TRION MUST NOT assume ternary is always smaller, faster, or more energy-efficient.

A feature SHOULD use ternary when its value domain or measured implementation justifies it. Binary SHOULD be retained when required for hardware compatibility or when it performs better. A hybrid implementation MAY select representations by data type or workload, provided conversions are explicit and semantics remain stable.

Benchmarks SHOULD compare equivalent workloads and include total storage size, metadata, conversion time, throughput, latency, peak memory, and energy per completed task when reliable measurement is available. Results MUST identify hardware, compiler/build settings, workload, and baseline. Energy savings MUST NOT be inferred from instruction counts alone.

## 7. Required tests and acceptance criteria

Phase 1 is complete only when the specification and implementation plan are reviewed and the following obligations are captured. This document alone does not mean code has passed them.

### 7.1 Core representation tests

- Round-trip every possible trit sequence for small counts.
- Test zero trits, maximum supported count, all-zero, all-two, and alternating patterns.
- Reject trits outside {0,1,2}.
- Reject invalid packed values for a declared count.
- Reject unsupported counts and over-capacity sequences.
- Test u32 20-trit and u64 40-trit boundaries.
- Test balanced/unbalanced single-trit conversions and signed multi-trit conversions once the latter's format is specified.
- Verify serialized byte order and format-version behavior.
- Ensure malformed input returns errors rather than panicking.

### 7.2 Bridge tests

- Round-trip representative values at API boundaries.
- Verify overflow, truncation, length mismatch, and invalid-encoding behavior.
- Test buffer boundaries and zero-length inputs.
- Fuzz decoding, length handling, and serialization parsers.
- Verify native binary buffers remain byte-for-byte unchanged through binary-only interfaces.

### 7.3 Translator tests

- Test ELF files for supported and unsupported architectures.
- Test malformed and truncated ELF input.
- Distinguish decode/analysis results from actual translated output.
- Verify unsupported instructions produce explicit diagnostics.
- Do not claim semantic equivalence until differential tests exist for the implemented translated subset.

### 7.4 Release gates

A Phase 1 implementation MUST NOT be considered production-ready solely because it compiles. It needs passing automated tests, documented limitations, and reproducible benchmark instructions. Full OS builds, ISO boot, driver compatibility, and whole-program translation are outside this phase and must be reported separately.

## 8. Non-goals

Phase 1 does not attempt to:

- invent or claim physical ternary CPU hardware;
- replace all binary data or hardware interfaces;
- promise universal storage or energy savings;
- translate every x86-64 or ARM64 executable;
- guarantee compatibility with Windows/Linux application ABIs;
- introduce a new persistent filesystem format without migration/recovery design;
- claim production safety or performance without tests;
- merge experimental behavior into default OS execution paths.

## 9. Implementation handoff for GHC

Implement in this order:

1. Review this specification and raise conflicts before coding.
2. Add core representation types and checked encode/decode APIs in ternary/core/.
3. Add unit and property tests for defined boundaries and invalid cases.
4. Add a design note documenting the selected signed multi-trit integer encoding before implementing it.
5. Document translator classifications and current limitations in ternary/translator/.
6. Add tests proving inspection and translation are reported as distinct capabilities.
7. Run the narrowest relevant test suite, then broader repository checks where supported.
8. Report exact commands and results. Do not claim a full OS build, successful boot, or complete translation unless each was actually tested.

Any implementation choice not specified here MUST be documented and reviewed rather than silently invented.

## 10. Decisions requiring explicit review before Phase 2

- The canonical signed multi-trit integer encoding.
- Whether balanced trits are a first-class packed storage format or only an arithmetic/logical domain in the first release.
- The concrete compiler annotation syntax and compiler integration point.
- The translator's first supported instruction subset and output format.
- Whether future translation uses ahead-of-time native code, a VM, an interpreter, or multiple backends.
- The initial benchmark workloads and minimum acceptable wins for enabling a ternary path by default.
