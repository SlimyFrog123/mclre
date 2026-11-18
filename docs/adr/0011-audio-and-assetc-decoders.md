# 0011. Audio Engine & assetc `.binka` Decoding

- **Status:** Proposed (pending owner confirmation)
- **Date:** 2026-11-17
- **Deciders:** Agent draft; owner to confirm

## Context
SPEC 11.5/14 [DECIDE] + V-05: `.binka` audio confirmed in the reference (40+ music files). Multi-listener splitscreen policy documented in `docs/reference/audio.md` at M11 (V-06).

## Options considered
See `PLAN.md` §3.2 (D-11).

## Decision (recommended default)
`kira` as the audio engine (3D positioning + multiple listeners/soundtracks). `assetc` shells out to `vgmstream` for `.binka` → Ogg conversion (external tool, no bundled codec); if that proves fragile, revisit bundling.

## Consequences
`vgmstream` invocation adds an external dev dependency to `assetc` only (engine stays clean). Owner confirmation pending before M3/M11.
