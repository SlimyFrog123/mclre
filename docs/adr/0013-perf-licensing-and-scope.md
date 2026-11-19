# 0013. Perf Targets, `faithful_bugs` Defaults, Finite→Infinite Migration, Licenses

- **Status:** Accepted (agent decision, owner confirmation pending) — confirmed 2026-11-18
- **Date:** 2026-11-17
- **Deciders:** Agent (owner confirmation pending)

## Context
SPEC 3.3, 8.1, 12.5, 13, 16, 20 [DECIDE]s bundled as one pending-owner ADR.

## Options considered
See `PLAN.md` §3.2 (D-13).

## Decision (recommended default)
- **Perf:** low = iGPU / 8 GB / 30 FPS; mid = GTX 1060-class / 1080p / 60 FPS @ render distance 12–16 (to be written into `docs/perf.md`).
- **`faithful_bugs`:** default ON for gameplay-affecting quirks players rely on, OFF for crash/exploit bugs (per-bug list grows in `docs/reference/discrepancies.md`).
- **Finite→infinite migration:** defer; design chunk storage so it stays possible.
- **Own code license:** MIT / Apache-2.0 dual.
- **Shader pipeline:** clean-room reimplementation from format docs (no LGPL Iris code ported).

## Consequences
License choice is trivial to change pre-distribution (repo is private). Shader clean-room commitment constrains future Iris integration to format-compat only. Owner confirmation pending.
