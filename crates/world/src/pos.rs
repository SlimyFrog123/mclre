//! World coordinates: block positions, chunk positions, section positions.
//!
//! Three coordinate spaces exist (SPEC.md 5.6 / PLAN.md M2):
//!
//! | Type | Unit | Conversion |
//! |---|---|---|
//! | [`BlockPos`] | one block | — |
//! | [`ChunkPos`] | 16×16 column | `block >> 4` |
//! | [`SectionPos`] | 16×16×16 section | `block >> 4` on each axis |
//!
//! All conversions use arithmetic shifts, so negative coordinates floor
//! correctly (`-1 >> 4 == -1`, i.e. chunk `-1`), matching Java Edition /
//! Anvil behavior and the `infinite` preset.
//!
//! Local block coordinates inside a chunk/section are always `0..16` and are
//! obtained with `x & 15` (or [`BlockPos::local_x`] and friends).

// Coordinate math packs/unpacks 16-block indices; every cast is bounded by the
// masks above (0..=4095, 0..=15) and never sees untrusted input.
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

use std::fmt;

use mclre_core::BlockStateId;

/// Number of blocks along one horizontal chunk axis.
pub const CHUNK_SIZE: i32 = 16;

/// Number of blocks along one vertical section axis.
pub const SECTION_SIZE: i32 = 16;

/// Number of blocks in one section (`16³`).
pub const SECTION_VOLUME: usize = 4096;

/// Number of bits to shift a block coordinate into a chunk/section coordinate.
pub const CHUNK_SHIFT: u32 = 4;

/// Mask selecting the local coordinate inside a chunk/section (`16 - 1`).
pub const CHUNK_MASK: i32 = CHUNK_SIZE - 1;

/// A block position in world space.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockPos {
    /// World X (east).
    pub x: i32,
    /// World Y (up).
    pub y: i32,
    /// World Z (south).
    pub z: i32,
}

impl BlockPos {
    /// Origin position `(0, 0, 0)`.
    pub const ORIGIN: Self = Self { x: 0, y: 0, z: 0 };

    /// Creates a position.
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// This position shifted by the given offsets.
    #[must_use]
    pub const fn offset(self, dx: i32, dy: i32, dz: i32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            z: self.z + dz,
        }
    }

    /// The chunk column containing this position.
    pub const fn chunk(self) -> ChunkPos {
        ChunkPos {
            x: self.x >> CHUNK_SHIFT,
            z: self.z >> CHUNK_SHIFT,
        }
    }

    /// The section containing this position.
    pub const fn section(self) -> SectionPos {
        SectionPos {
            x: self.x >> CHUNK_SHIFT,
            y: self.y >> CHUNK_SHIFT,
            z: self.z >> CHUNK_SHIFT,
        }
    }

    /// X inside the chunk (`0..16`).
    pub const fn local_x(self) -> i32 {
        self.x & CHUNK_MASK
    }

    /// Y inside the section (`0..16`).
    pub const fn local_y(self) -> i32 {
        self.y & CHUNK_MASK
    }

    /// Z inside the chunk (`0..16`).
    pub const fn local_z(self) -> i32 {
        self.z & CHUNK_MASK
    }

    /// Linear index inside a section (`(y << 8) | (z << 4) | x`).
    ///
    /// This is the packing order used by chunk storage and the wire format:
    /// X varies fastest, then Z, then Y.
    pub const fn section_index(self) -> usize {
        ((self.local_y() << 8) | (self.local_z() << 4) | self.local_x()) as usize
    }

    /// Position of a block from its chunk and local coordinates.
    ///
    /// Local coordinates are masked to `0..16`, so callers cannot produce a
    /// position outside the named chunk.
    pub const fn from_chunk_local(chunk: ChunkPos, x: i32, y: i32, z: i32) -> Self {
        Self {
            x: (chunk.x << CHUNK_SHIFT) | (x & CHUNK_MASK),
            y,
            z: (chunk.z << CHUNK_SHIFT) | (z & CHUNK_MASK),
        }
    }

    /// Squared Euclidean distance to `other` (`i64` to stay exact).
    pub fn distance_squared(self, other: Self) -> i64 {
        let dx = i64::from(self.x) - i64::from(other.x);
        let dy = i64::from(self.y) - i64::from(other.y);
        let dz = i64::from(self.z) - i64::from(other.z);
        dx * dx + dy * dy + dz * dz
    }
}

impl fmt::Display for BlockPos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

/// A chunk column position (16×16 blocks, full height).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChunkPos {
    /// Chunk X.
    pub x: i32,
    /// Chunk Z.
    pub z: i32,
}

impl ChunkPos {
    /// Chunk position `(0, 0)`.
    pub const ORIGIN: Self = Self { x: 0, z: 0 };

    /// Creates a chunk position.
    pub const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    /// The chunk containing the given block coordinates.
    pub const fn from_block(x: i32, z: i32) -> Self {
        Self {
            x: x >> CHUNK_SHIFT,
            z: z >> CHUNK_SHIFT,
        }
    }

    /// The block position of this chunk's `(0, y, 0)` corner.
    pub const fn origin(self, y: i32) -> BlockPos {
        BlockPos {
            x: self.x << CHUNK_SHIFT,
            y,
            z: self.z << CHUNK_SHIFT,
        }
    }

    /// The chunk at the given offset.
    #[must_use]
    pub const fn offset(self, dx: i32, dz: i32) -> Self {
        Self {
            x: self.x + dx,
            z: self.z + dz,
        }
    }

    /// Squared distance between chunk centers, in chunks.
    pub fn distance_squared(self, other: Self) -> i64 {
        let dx = i64::from(self.x) - i64::from(other.x);
        let dz = i64::from(self.z) - i64::from(other.z);
        dx * dx + dz * dz
    }

    /// The chunk key used by Anvil region files (`r.X.Z.mca`): region
    /// coordinates are `chunk >> 5`, local coordinates `chunk & 31`.
    pub const fn region(self) -> region::RegionPos {
        region::RegionPos::from_chunk(self)
    }
}

impl fmt::Display for ChunkPos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.z)
    }
}

/// Anvil region math: 32×32 chunks per `r.X.Z.mca` file.
pub mod region {
    /// Chunks per region axis.
    pub const REGION_CHUNKS: i32 = 32;
    /// Bits to shift a chunk coordinate into a region coordinate.
    pub const REGION_SHIFT: u32 = 5;
    /// Mask selecting the in-region chunk coordinate.
    pub const REGION_MASK: i32 = REGION_CHUNKS - 1;

    /// The region file coordinate (`r.X.Z.mca`).
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct RegionPos {
        /// Region X.
        pub x: i32,
        /// Region Z.
        pub z: i32,
    }

    impl RegionPos {
        /// Creates a region position.
        pub const fn new(x: i32, z: i32) -> Self {
            Self { x, z }
        }

        /// The region containing a chunk.
        pub const fn from_chunk(chunk: super::ChunkPos) -> Self {
            Self {
                x: chunk.x >> REGION_SHIFT,
                z: chunk.z >> REGION_SHIFT,
            }
        }

        /// The chunk's local coordinates inside the region (`0..32`).
        pub const fn local(chunk: super::ChunkPos) -> (i32, i32) {
            (chunk.x & REGION_MASK, chunk.z & REGION_MASK)
        }

        /// Slot index inside the region's location table (`local_z * 32 + local_x`).
        pub const fn slot(chunk: super::ChunkPos) -> usize {
            let (x, z) = Self::local(chunk);
            (z * REGION_CHUNKS + x) as usize
        }
    }
}

/// A 16×16×16 section position.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SectionPos {
    /// Section X.
    pub x: i32,
    /// Section Y (vertical section index, may be negative in future profiles).
    pub y: i32,
    /// Section Z.
    pub z: i32,
}

impl SectionPos {
    /// Section position `(0, 0, 0)`.
    pub const ORIGIN: Self = Self { x: 0, y: 0, z: 0 };

    /// Creates a section position.
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// The section containing the given block coordinates.
    pub const fn from_block(x: i32, y: i32, z: i32) -> Self {
        Self {
            x: x >> CHUNK_SHIFT,
            y: y >> CHUNK_SHIFT,
            z: z >> CHUNK_SHIFT,
        }
    }

    /// The chunk column this section belongs to.
    pub const fn chunk(self) -> ChunkPos {
        ChunkPos {
            x: self.x,
            z: self.z,
        }
    }

    /// The block position of this section's `(0, 0, 0)` corner.
    pub const fn origin(self) -> BlockPos {
        BlockPos {
            x: self.x << CHUNK_SHIFT,
            y: self.y << CHUNK_SHIFT,
            z: self.z << CHUNK_SHIFT,
        }
    }
}

impl fmt::Display for SectionPos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

/// A section-local block position (`0..16` on each axis).
///
/// Cheaper than [`BlockPos`] for palette/section APIs and for packed wire
/// payloads; conversion is checked so a bad index is an error, not a wrap.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalPos {
    x: u8,
    y: u8,
    z: u8,
}

/// A coordinate outside `0..16` was supplied for a section-local position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("section-local coordinate out of range: ({x}, {y}, {z})")]
pub struct LocalPosError {
    /// Offending X.
    pub x: i32,
    /// Offending Y.
    pub y: i32,
    /// Offending Z.
    pub z: i32,
}

impl LocalPos {
    /// Maximum valid coordinate on each axis.
    pub const MAX: u8 = 15;

    /// Creates a local position, validating each axis.
    pub fn new(x: i32, y: i32, z: i32) -> Result<Self, LocalPosError> {
        let valid = |v: i32| (0..super::SECTION_SIZE).contains(&v);
        if !valid(x) || !valid(y) || !valid(z) {
            return Err(LocalPosError { x, y, z });
        }
        Ok(Self {
            x: x as u8,
            y: y as u8,
            z: z as u8,
        })
    }

    /// Creates a local position from already-validated coordinates.
    ///
    /// # Panics
    /// If any coordinate is `>= 16` (caller bug; use [`LocalPos::new`] for
    /// untrusted input).
    pub const fn from_axes(x: u8, y: u8, z: u8) -> Self {
        assert!(x < 16 && y < 16 && z < 16, "local coordinate out of range");
        Self { x, y, z }
    }

    /// X coordinate.
    pub const fn x(self) -> u8 {
        self.x
    }

    /// Y coordinate.
    pub const fn y(self) -> u8 {
        self.y
    }

    /// Z coordinate.
    pub const fn z(self) -> u8 {
        self.z
    }

    /// Linear index in YZX order (`(y << 8) | (z << 4) | x`).
    pub const fn index(self) -> usize {
        ((self.y as usize) << 8) | ((self.z as usize) << 4) | self.x as usize
    }

    /// The inverse of [`LocalPos::index`].
    ///
    /// # Panics
    /// If `index >= 4096` (caller bug; section storage never exceeds it).
    pub const fn from_index(index: usize) -> Self {
        assert!(index < SECTION_VOLUME, "section index out of range");
        Self {
            x: (index & 15) as u8,
            y: ((index >> 8) & 15) as u8,
            z: ((index >> 4) & 15) as u8,
        }
    }
}

/// The block-state id used as the "empty" filler in a section.
///
/// By convention `BlockStateId(0)` is `minecraft:air` (air registers first in
/// `content-vanilla`), which makes default-initialized storage all-air.
pub const AIR_STATE: BlockStateId = BlockStateId(0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_and_section_conversions_floor_for_negatives() {
        let pos = BlockPos::new(-1, -17, -16);
        assert_eq!(pos.chunk(), ChunkPos::new(-1, -1));
        assert_eq!(pos.section(), SectionPos::new(-1, -2, -1));
        assert_eq!(pos.local_x(), 15);
        assert_eq!(pos.local_y(), 15);
        assert_eq!(pos.local_z(), 0);

        let pos = BlockPos::new(15, 15, 15);
        assert_eq!(pos.chunk(), ChunkPos::new(0, 0));
        assert_eq!(pos.section(), SectionPos::new(0, 0, 0));
        assert_eq!(pos.section_index(), (15 << 8) | (15 << 4) | 15);

        let pos = BlockPos::new(16, 16, 16);
        assert_eq!(pos.chunk(), ChunkPos::new(1, 1));
        assert_eq!(pos.section(), SectionPos::new(1, 1, 1));
        assert_eq!(pos.section_index(), 0);
    }

    #[test]
    fn from_chunk_local_masks_coordinates() {
        let pos = BlockPos::from_chunk_local(ChunkPos::new(2, 3), 16 + 5, 70, 16 + 7);
        assert_eq!(pos, BlockPos::new(2 * 16 + 5, 70, 3 * 16 + 7));
        let pos = BlockPos::from_chunk_local(ChunkPos::new(-1, -1), -1, 0, -1);
        assert_eq!(pos, BlockPos::new(-1, 0, -1));
    }

    #[test]
    fn section_index_round_trips_for_every_index() {
        for index in 0..SECTION_VOLUME {
            let local = LocalPos::from_index(index);
            assert_eq!(local.index(), index, "index {index}");
            let block = BlockPos::new(
                i32::from(local.x()),
                i32::from(local.y()),
                i32::from(local.z()),
            );
            assert_eq!(block.section_index(), index);
        }
    }

    #[test]
    fn local_pos_rejects_out_of_range() {
        assert!(LocalPos::new(0, 0, 0).is_ok());
        assert!(LocalPos::new(15, 15, 15).is_ok());
        assert!(LocalPos::new(16, 0, 0).is_err());
        assert!(LocalPos::new(-1, 0, 0).is_err());
        assert!(LocalPos::new(0, 0, 16).is_err());
    }

    #[test]
    fn region_math_splits_into_32_chunk_files() {
        let chunk = ChunkPos::new(37, -1);
        let region = chunk.region();
        assert_eq!(region, region::RegionPos::new(1, -1));
        assert_eq!(region::RegionPos::local(chunk), (5, 31));
        assert_eq!(region::RegionPos::slot(chunk), 31 * 32 + 5);

        // Region slot is unique and in range across a full region.
        let mut seen = [false; 32 * 32];
        for x in 0..32 {
            for z in 0..32 {
                let slot = region::RegionPos::slot(ChunkPos::new(x, z));
                assert!(!seen[slot]);
                seen[slot] = true;
            }
        }
    }

    #[test]
    fn chunk_origin_is_corner_block() {
        let chunk = ChunkPos::new(-2, 5);
        assert_eq!(chunk.origin(64), BlockPos::new(-32, 64, 80));
        assert_eq!(chunk.origin(64).chunk(), chunk);
    }

    #[test]
    fn section_and_chunk_convert_both_ways() {
        let section = SectionPos::from_block(-5, 33, 17);
        assert_eq!(section, SectionPos::new(-1, 2, 1));
        assert_eq!(section.chunk(), ChunkPos::new(-1, 1));
        let origin = section.origin();
        assert_eq!(origin, BlockPos::new(-16, 32, 16));
        assert_eq!(origin.section(), section);
    }

    #[test]
    fn distance_squared_uses_block_units() {
        let a = BlockPos::new(0, 0, 0);
        let b = BlockPos::new(3, 4, 12);
        assert_eq!(a.distance_squared(b), 9 + 16 + 144);
        assert_eq!(
            ChunkPos::new(0, 0).distance_squared(ChunkPos::new(3, 4)),
            25
        );
    }
}
