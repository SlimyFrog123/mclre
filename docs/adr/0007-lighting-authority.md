# 0007. Light Computation Authority

- **Status:** Accepted (agent decision, owner confirmation pending) — confirmed 2026-11-18
- **Date:** 2026-11-17
- **Deciders:** Agent (owner confirmation pending)

## Context
SPEC 7.3 [DECIDE]. Server-authoritative keeps clients honest and deterministic; client recompute would duplicate the reference propagation algorithm on both sides.

## Options considered
See `PLAN.md` §3.2 (D-06).

## Decision (recommended default)
Server computes sky+block light and sends light data in chunk packets; client never simulates light propagation (only applies received light to meshing).

## Consequences
More network bytes per chunk (fine; LCE worlds are small). Client must still request relight on block updates (server response). Owner confirmation pending before M4 protocol design.
