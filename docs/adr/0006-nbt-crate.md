# 0006. NBT Library

- **Status:** Accepted (agent decision, owner confirmation pending) — revised 2026-11-18
- **Date:** 2026-11-17 (revised 2026-11-18)
- **Deciders:** Agent draft; owner to confirm

## Context
SPEC 14 [DECIDE]. Saves, registry snapshots, structures, and Java-tool interop all need Java-compatible NBT read/write. Confirm before M1/M2 saves.

Constraints that turned out to be decisive:
- The workspace pins **stable Rust 1.88** (`rust-toolchain.toml`) and CI runs stable on Windows + Linux.
- SPEC 18 / CLAUDE.md require **deterministic** serialization: identical data must produce identical bytes.

## Options considered
See `PLAN.md` §3.2 (D-05).

1. **`simdnbt`** (planned default). Verified 2026-11-18: every published version we tried (0.4 → 0.10) carries `#![feature(portable_simd)]` / `#![feature(allocator_api)]` and therefore **cannot build on stable Rust**. Rejected for the stable-MSRV rule.
2. **`fastnbt`**. Builds on stable, but its `Value::Compound` is a `HashMap`, so key order on re-serialization is unspecified — a direct conflict with the determinism rule (and it would force us to re-sort or keep a parallel ordered structure anyway).
3. **In-house codec inside `core::nbt`** behind our own owned model.

## Decision
Implement the NBT codec in `crates/core/src/nbt.rs` (option 3), with no third-party NBT dependency:

- Owned `Value` / ordered `Compound` (entry order preserved, `insert` replaces in place).
- Big-endian Java-format reader/writer, all 12 tags, `u16`-length-prefixed names/strings in **Java modified UTF-8** (`U+0000` → `C0 80`, astral chars as surrogate pairs), preserving the root name.
- Hard depth limit (`MAX_DEPTH = 512`), bounds-checked reads, length-prefixed allocations capped at the remaining input, and no `unwrap`/`expect` on parsed data — corrupt input is an `NbtError`.
- Tests: hand-computed golden bytes, round-trip of every tag type, MUTF-8 edge cases, depth/short-input/allocation attacks, plus `proptest` round-trip and "arbitrary bytes never panic" properties.

## Consequences
- No nightly requirement and no dependency whose model we cannot control; dependency graph stays small (helps `cargo-deny`/audit).
- We own format correctness: region-file compression (M2), `level.dat`/chunk schemas, and data fixers must be fuzzed against real Java-produced files when M2 lands.
- Backend swap remains possible: only `core::nbt` exposes backend types, and there are none today.
- Revisit if a stable, order-preserving NBT crate appears; the public API would not change.
