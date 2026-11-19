# 0004. Infinite Preset Behavior Inside Legacy Bounds

- **Status:** Accepted (agent decision, owner confirmation pending) — confirmed 2026-11-18
- **Date:** 2026-11-17
- **Deciders:** Agent (owner confirmation pending)

## Context
SPEC 3.6 [DECIDE]: whether an `infinite` world matches a finite preset in-bounds. Biome layout in LCE is scaled per world size; `infinite` has no original counterpart.

## Options considered
See `PLAN.md` §3.2 (D-03).

## Decision (recommended default)
`infinite` uses the SAME generator code with unbounded coordinates and un-scaled biome layout (Java-like), NOT a byte-match of a finite preset inside bounds. Parity region remains finite presets only.

## Consequences
One generator, two coordinate regimes — simplest path to M6 exact parity (finite seeds byte-match reference) while infinite stays spec-compliant. If the owner later wants an in-bounds match, add a preset-affine flag to the biome layer stack (small, data-level change).
