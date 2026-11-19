//! World model: chunks, block states, palette-compressed sections, lighting,
//! Anvil region storage, dimensions, world size presets (SPEC.md 5.6, PLAN.md M2).
//!
//! Implemented so far:
//! - [`pos`]: block/chunk/section coordinates, Anvil region math, section-local
//!   positions and the YZX section index order.
//! - [`preset`]: [`WorldSizePreset`] with the verified console dimensions,
//!   Nether scales, fixed End size, and edge-moat flags.
//! - [`chunk_nbt`]: chunk column ↔ Anvil NBT encoding/decoding.
//! - [`level`]: `level.dat` NBT structure.
//! - [`save`]: world save orchestration (region cache + atomic level.dat writes).

pub mod anvil;
pub mod chunk;
pub mod chunk_nbt;
pub mod level;
pub mod pos;
pub mod preset;
pub mod save;
pub mod section;
pub mod state_key;

pub use anvil::{CompressionType, RegionError, RegionFile, region_file_name, region_of_path};
pub use chunk::{BASELINE_SECTIONS, ChunkColumn, ChunkError, Heightmap, MAX_SECTIONS};
pub use chunk_nbt::{ChunkNbtError, decode as decode_chunk_nbt, encode as encode_chunk_nbt};
pub use level::{DATA_VERSION, DimensionData, LevelDat, LevelDatError};
pub use pos::{
    AIR_STATE, BlockPos, CHUNK_MASK, CHUNK_SHIFT, CHUNK_SIZE, ChunkPos, LocalPos, LocalPosError,
    SECTION_SIZE, SECTION_VOLUME, SectionPos,
};
pub use preset::{UnknownPreset, WorldSizePreset};
pub use save::{Save, SaveError};
pub use section::{ChunkSection, NibbleArray, SectionStorage, StorageKind};
pub use state_key::{StateCodec, StateKey};
