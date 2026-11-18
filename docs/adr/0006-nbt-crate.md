# 0006. NBT Library

- **Status:** Proposed (pending owner confirmation)
- **Date:** 2026-11-17
- **Deciders:** Agent draft; owner to confirm

## Context
SPEC 14 [DECIDE]. Saves, registry snapshots, structures, and Java-tool interop all need Java-compatible NBT read/write. Confirm before M1/M2 saves.

## Options considered
See `PLAN.md` §3.2 (D-05).

## Decision (recommended default)
`simdnbt` for read-heavy paths, wrapped in a clean internal `core::nbt` API (owned Value type + streaming reader) so the crate is swappable; `fastnbt` as fallback candidate.

## Consequences
Wrapping keeps allocation strategy internal. Must fuzz the reader (SPEC 18). Owner confirmation pending.
