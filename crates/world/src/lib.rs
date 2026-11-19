//! World model: chunks, block states, palette-compressed sections, lighting,
//! Anvil region storage, dimensions, world size presets (SPEC.md 5.6, PLAN.md M2).
//!
//! Implemented so far:
//! - [`pos`]: block/chunk/section coordinates, Anvil region math, section-local
//!   positions and the YZX section index order.
//! - [`preset`]: [`WorldSizePreset`] with the verified console dimensions,
//!   Nether scales, fixed End size, and edge-moat flags.

pub mod pos;
pub mod preset;
pub mod section;

pub use pos::{
    AIR_STATE, BlockPos, CHUNK_MASK, CHUNK_SHIFT, CHUNK_SIZE, ChunkPos, LocalPos, LocalPosError,
    SECTION_SIZE, SECTION_VOLUME, SectionPos,
};
pub use preset::{UnknownPreset, WorldSizePreset};
pub use section::{ChunkSection, NibbleArray, SectionStorage, StorageKind};
