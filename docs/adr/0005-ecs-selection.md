# 0005. Server ECS Framework

- **Status:** Proposed (pending owner confirmation)
- **Date:** 2026-11-17
- **Deciders:** Agent draft; owner to confirm

## Context
SPEC 7.2 [DECIDE]. Needs scheduling, deterministic iteration control, and mod-facing registration. Alternatives: `hecs` (simpler, less scheduling), custom (max control, more work).

## Options considered
See `PLAN.md` §3.2 (D-04).

## Decision (recommended default)
`bevy_ecs` standalone (no Bevy app), wrapped behind a thin `sim::entity` facade so it stays replaceable. Entity "classes" = archetype + behavior systems + data registered via public API.

## Consequences
Bigger dependency tree on the server path (license Apache/MIT — passes policy). Determinism: only iterate entity sets in explicit ordered structures where order affects RNG. Owner confirmation pending before M4/M7.
