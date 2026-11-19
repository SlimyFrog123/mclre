//! Developer tools: reference-data extractors, registry dumpers, worldgen viewers, packet inspectors.
//!
//! Currently implemented: [`legacy_blocks`] — parses the reference `Tile.cpp`
//! registration table into `data/legacy/blocks.toml` (legacy `(id)` → block
//! metadata). The legacy mapping is import/parity data only; game logic never
//! reads numeric ids (SPEC.md 5.2/5.3).

pub mod legacy_blocks;
