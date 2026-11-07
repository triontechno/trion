# TRION Phase 2 — Stage D: Translator (Retired)

**Status: Discontinued by architecture decision.**

The former Stage D implementation delivered a limited x86-64-to-TIR translator, a bounded TIR interpreter, and a TRIONEX v1 (.trx) container. It accepted only a restricted instruction subset and did not provide general Windows or macOS application compatibility.

**This is no longer TRION's application execution architecture.** It is superseded by the direct executable engine in [Stage G](Phase%202%20Stage%20G.md).

## Discontinued

- Translating PE, Mach-O, or ELF application code into TIR for execution.
- Requiring .tir or .trx artifacts to run applications.
- TRIONEX as an application executable format.
- The trion-translate application workflow: translate, import, package, or run translated programs.
- Any fallback from direct application launch to translation.

The old implementation may remain in repository history for reference during code/dependency cleanup. It must not be presented as supported application-launch functionality or expanded as a parallel execution path.

**Stage G is the sole planned engine for opening supported Windows .exe and macOS .app applications in TRION.**
