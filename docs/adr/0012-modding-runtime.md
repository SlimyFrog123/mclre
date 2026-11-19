# 0012. Mod Runtime (WASM) & Native Mods

- **Status:** Accepted (agent decision, owner confirmation pending) — confirmed 2026-11-18
- **Date:** 2026-11-17
- **Deciders:** Agent (owner confirmation pending)

## Context
SPEC 15 [DECIDE]. First mod milestone is the vanilla content crate itself + one example mod (block + recipe) as the API acceptance test.

## Options considered
See `PLAN.md` §3.2 (D-12).

## Decision (recommended default)
`wasmtime` with the component model (WIT interfaces) for sandboxed mods + `mclre-mod-api` guest SDK. Dev-only native Rust mods: include behind an explicit feature flag with user confirmation, no ABI stability promised.

## Consequences
`wasmtime` is heavy (compile time/disk — C: space is tight; consider keeping it out of default feature flags). Sandbox budgets: tick fuel + memory caps. Owner confirmation pending before M12.
