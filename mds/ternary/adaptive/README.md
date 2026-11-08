# TRION adaptive representation policy

This crate provides the Stage A decision engine for choosing Binary, BalancedTernary, or Hybrid representations from caller-supplied target capabilities, compatibility requirements, semantic-validation status, and comparable costs.

It deliberately does not:
- detect or claim physical ternary hardware;
- generate benchmark values;
- translate machine instructions or executables;
- make a representation safe merely because it is ternary.

## Use

Add this crate as a path dependency:

```toml
trion-adaptive = { path = "../adaptive" }
```

Construct a `SelectionRequest` with a `Boundary` and a `CandidateSet`. Each candidate is optional; a present candidate must state whether it is supported, semantically validated, compatible, and its comparable total cost. The selector rejects candidates that lack required evidence.

For internal workloads, the lowest-cost supported and validated candidate wins. For conventional hardware, network-wire, and persistent-binary boundaries, a compatible Hybrid path is selected only when its cost is no greater than the compatible Binary path; otherwise the selector uses Binary. Cryptographic selection requires both semantic validation and compatibility.

The cost unit is defined by the caller. Compare candidates for the same workload and target, and include conversion, memory, storage, and compatibility overhead.
