# 0008. Chunk Meshing Strategy & Smooth Lighting

- **Status:** Proposed (pending owner confirmation)
- **Date:** 2026-11-17
- **Deciders:** Agent draft; owner to confirm

## Context
SPEC 12.2 [DECIDE]. LCE smooth lighting look is a fidelity target; greedy meshing can change vertex-level AO seams.

## Options considered
See `PLAN.md` §3.2 (D-08).

## Decision (recommended default)
Per-face meshing with per-vertex AO / LCE smooth lighting as the default; greedy meshing stays an optional low-end setting, OFF by default unless profiling shows fidelity-neutral wins.

## Consequences
More vertices than greedy (acceptable at LCE render distances); revisit after M3 profiling on the mid hardware profile.
