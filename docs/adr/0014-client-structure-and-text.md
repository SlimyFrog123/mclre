# 0014. Client Structure & Text Rendering

- **Status:** Proposed (pending owner confirmation)
- **Date:** 2026-11-17
- **Deciders:** Agent draft; owner to confirm

## Context
SPEC 12/14 [DECIDE]. Locking crate boundaries early prevents client/server bleed and keeps shipping UI data-driven.

## Options considered
See `PLAN.md` §3.2 (D-07 + client-structure note).

## Decision (recommended default)
Client = `mclre-client` lib (world mirror, prediction, interpolation) + `app` for loop/glue; text rendering via `glyphon` (cosmic-text) with a dedicated bitmap pixel-font path for the LCE look; `egui` only behind `dev-ui` for tooling.

## Consequences
Bitmap font path is extra work but required for fidelity; `glyphon` covers TTF. Owner confirmation pending before M3/M8.
