//! Chunk columns: a stack of 16×16×16 sections covering the full world height.
//!
//! A column is addressed by [`ChunkPos`] and holds `section_count` sections
//! (16 for the baseline 256-block build height). Section `y` covers world Y
//! `y * 16 .. y * 16 + 16`; section 0 is the bottom of the world.
//!
//! Columns also keep a 16×16 [`Heightmap`] of the highest non-air block per
//! column cell. It is maintained incrementally on writes and can be rebuilt
//! from the sections at any time, so a corrupted heightmap never corrupts
//! block data (the sections are authoritative).

// Section/local indices are bounded by MAX_SECTIONS and LocalPos.
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

use mclre_core::BlockStateId;

use crate::pos::{AIR_STATE, BlockPos, CHUNK_SIZE, ChunkPos, LocalPos, SECTION_SIZE};
use crate::section::ChunkSection;

/// Sections in a baseline column (`256 / 16`).
pub const BASELINE_SECTIONS: u16 = 16;

/// Upper bound on sections per column (a 4096-block-tall world).
///
/// This is a sanity bound for untrusted save data, not a gameplay limit.
pub const MAX_SECTIONS: u16 = 256;

/// Errors from column construction and block access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ChunkError {
    /// The section count is zero or above [`MAX_SECTIONS`].
    #[error("invalid column section count {0}")]
    SectionCount(u16),
    /// A block position belongs to another chunk than the column.
    #[error("block {pos} is not in chunk {chunk}")]
    WrongChunk {
        /// The column's chunk position.
        chunk: ChunkPos,
        /// The rejected block position.
        pos: BlockPos,
    },
}

/// A 16×16 heightmap in block units: the number of blocks from the bottom of
/// the world to the highest non-air block, inclusive (`0` = empty column cell).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heightmap {
    /// `16 * 16` entries in X-then-Z order (`index = z * 16 + x`).
    values: Vec<u16>,
}

impl Heightmap {
    /// An empty heightmap (`0` everywhere).
    pub fn empty() -> Self {
        Self {
            values: vec![0; (CHUNK_SIZE * CHUNK_SIZE) as usize],
        }
    }

    /// The height at local `(x, z)` (`0..16`).
    ///
    /// # Panics
    /// If `x` or `z` is outside `0..16` (caller bug).
    pub fn get(&self, x: i32, z: i32) -> u16 {
        self.values[index(x, z)]
    }

    /// Sets the height at local `(x, z)`.
    ///
    /// # Panics
    /// If `x` or `z` is outside `0..16` (caller bug).
    pub fn set(&mut self, x: i32, z: i32, height: u16) {
        self.values[index(x, z)] = height;
    }

    /// The highest non-empty height across the map (`0` if fully empty).
    pub fn max(&self) -> u16 {
        self.values.iter().copied().max().unwrap_or(0)
    }

    /// Raw values in `z * 16 + x` order (save serialization).
    pub fn values(&self) -> &[u16] {
        &self.values
    }
}

impl Default for Heightmap {
    fn default() -> Self {
        Self::empty()
    }
}

fn index(x: i32, z: i32) -> usize {
    debug_assert!((0..CHUNK_SIZE).contains(&x) && (0..CHUNK_SIZE).contains(&z));
    (z * CHUNK_SIZE + x) as usize
}

/// One 16×16 column of the world, `section_count * 16` blocks tall.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkColumn {
    pos: ChunkPos,
    sections: Vec<ChunkSection>,
    heightmap: Heightmap,
}

impl ChunkColumn {
    /// An all-air column at `pos` with `section_count` sections.
    pub fn new(pos: ChunkPos, section_count: u16) -> Result<Self, ChunkError> {
        if section_count == 0 || section_count > MAX_SECTIONS {
            return Err(ChunkError::SectionCount(section_count));
        }
        let sections = vec![ChunkSection::air(); usize::from(section_count)];
        let column = Self {
            pos,
            sections,
            heightmap: Heightmap::empty(),
        };
        Ok(column)
    }

    /// An all-air baseline column (256 blocks tall).
    pub fn baseline(pos: ChunkPos) -> Self {
        // 16 sections is inside the validated range.
        Self::new(pos, BASELINE_SECTIONS).unwrap_or_else(|error| unreachable!("{error}"))
    }

    /// A column built from already-decoded sections (save deserialization).
    ///
    /// The heightmap is recomputed from the sections, which are authoritative.
    pub fn from_sections(pos: ChunkPos, sections: Vec<ChunkSection>) -> Result<Self, ChunkError> {
        let count =
            u16::try_from(sections.len()).map_err(|_| ChunkError::SectionCount(u16::MAX))?;
        if count == 0 || count > MAX_SECTIONS {
            return Err(ChunkError::SectionCount(count));
        }
        let mut column = Self {
            pos,
            sections,
            heightmap: Heightmap::empty(),
        };
        column.recompute_heightmap();
        Ok(column)
    }

    /// The column's chunk position.
    pub fn pos(&self) -> ChunkPos {
        self.pos
    }

    /// The number of sections.
    pub fn section_count(&self) -> u16 {
        self.sections.len() as u16
    }

    /// The column height in blocks.
    pub fn height(&self) -> i32 {
        i32::from(self.section_count()) * SECTION_SIZE
    }

    /// The column's heightmap.
    pub fn heightmap(&self) -> &Heightmap {
        &self.heightmap
    }

    /// Section `index` (`0..section_count`), or `None` when out of range.
    pub fn section(&self, index: usize) -> Option<&ChunkSection> {
        self.sections.get(index)
    }

    /// Mutable section `index`, or `None` when out of range.
    pub fn section_mut(&mut self, index: usize) -> Option<&mut ChunkSection> {
        self.sections.get_mut(index)
    }

    /// Every section, bottom to top.
    pub fn sections(&self) -> &[ChunkSection] {
        &self.sections
    }

    /// The block state at a world position.
    ///
    /// Returns `Ok(None)` for a position outside the column's vertical range.
    /// A position in another chunk is an error (caller bug).
    pub fn get(&self, pos: BlockPos) -> Result<Option<BlockStateId>, ChunkError> {
        let Some((section_y, local)) = self.local_of(pos)? else {
            return Ok(None);
        };
        Ok(Some(self.sections[usize::from(section_y)].get(local)))
    }

    /// Sets the block state at a world position.
    ///
    /// Returns `Ok(false)` for a position outside the column's vertical range
    /// (nothing is written).
    pub fn set(&mut self, pos: BlockPos, state: BlockStateId) -> Result<bool, ChunkError> {
        let Some((section_y, local)) = self.local_of(pos)? else {
            return Ok(false);
        };
        let x = pos.local_x();
        let z = pos.local_z();
        self.write_local(section_y, local, state);
        self.update_heightmap(x, z, pos.y(), state);
        Ok(true)
    }

    /// The block state at a section-local position in section `section_y`.
    ///
    /// Returns `None` when the section index is outside the column.
    pub fn get_local(&self, section_y: u16, local: LocalPos) -> Option<BlockStateId> {
        self.sections
            .get(usize::from(section_y))
            .map(|section| section.get(local))
    }

    /// Sets the block state at a section-local position.
    ///
    /// This does not touch the heightmap (the caller owns the world position);
    /// [`ChunkColumn::set`] is the heightmap-maintaining entry point.
    ///
    /// # Panics
    /// If `section_y` is outside the column (caller bug; use
    /// [`ChunkColumn::section_mut`] for untrusted input).
    pub fn write_local(&mut self, section_y: u16, local: LocalPos, state: BlockStateId) {
        let section = &mut self.sections[usize::from(section_y)];
        section.set(local, state);
    }

    /// The non-air block count of the whole column.
    pub fn non_air_count(&self) -> u32 {
        self.sections
            .iter()
            .map(|section| u32::from(section.non_air_count()))
            .sum()
    }

    /// Whether every section is air.
    pub fn is_empty(&self) -> bool {
        self.sections.iter().all(ChunkSection::is_empty)
    }

    /// Recomputes every heightmap entry from the block data.
    pub fn recompute_heightmap(&mut self) {
        let mut map = Heightmap::empty();
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let mut height = 0u16;
                for y in 0..self.height() {
                    let pos = BlockPos::from_chunk_local(self.pos, x, y, z);
                    // Every y here is inside the column.
                    if matches!(self.get(pos), Ok(Some(state)) if state != AIR_STATE) {
                        height = (y + 1) as u16;
                    }
                }
                map.set(x, z, height);
            }
        }
        self.heightmap = map;
    }

    /// Converts a world position to a section index plus local position.
    fn local_of(&self, pos: BlockPos) -> Result<Option<(u16, LocalPos)>, ChunkError> {
        if pos.chunk() != self.pos {
            return Err(ChunkError::WrongChunk {
                chunk: self.pos,
                pos,
            });
        }
        if pos.y() < 0 || pos.y() >= self.height() {
            return Ok(None);
        }
        let section_y = pos.y() >> 4;
        let local = LocalPos::from_axes(
            pos.local_x() as u8,
            (pos.y() & 15) as u8,
            pos.local_z() as u8,
        );
        Ok(Some((section_y as u16, local)))
    }

    /// Updates one heightmap cell after a write.
    fn update_heightmap(&mut self, x: i32, z: i32, y: i32, state: BlockStateId) {
        let current = self.heightmap.get(x, z);
        let y_next = (y + 1) as u16;
        if state != AIR_STATE {
            if y_next > current {
                self.heightmap.set(x, z, y_next);
            }
        } else if y_next == current {
            // The top block was removed: rescan downwards.
            let mut height = 0u16;
            for scan in (0..y).rev() {
                let pos = BlockPos::from_chunk_local(self.pos, x, scan, z);
                if matches!(self.get(pos), Ok(Some(s)) if s != AIR_STATE) {
                    height = (scan + 1) as u16;
                    break;
                }
            }
            self.heightmap.set(x, z, height);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn state(id: u32) -> BlockStateId {
        BlockStateId(id)
    }

    #[test]
    fn new_column_is_empty_with_16_sections() {
        let column = ChunkColumn::baseline(ChunkPos::new(3, -2));
        assert_eq!(column.pos(), ChunkPos::new(3, -2));
        assert_eq!(column.section_count(), BASELINE_SECTIONS);
        assert_eq!(column.height(), 256);
        assert!(column.is_empty());
        assert_eq!(column.non_air_count(), 0);
        assert_eq!(column.heightmap().max(), 0);
        for section in column.sections() {
            assert!(section.is_empty());
        }
    }

    #[test]
    fn rejects_impossible_section_counts() {
        assert!(ChunkColumn::new(ChunkPos::ORIGIN, 0).is_err());
        assert!(ChunkColumn::new(ChunkPos::ORIGIN, 1).is_ok());
        assert!(ChunkColumn::new(ChunkPos::ORIGIN, MAX_SECTIONS).is_ok());
        assert!(ChunkColumn::new(ChunkPos::ORIGIN, MAX_SECTIONS + 1).is_err());
    }

    #[test]
    fn get_and_set_map_world_positions_into_sections() {
        let mut column = ChunkColumn::baseline(ChunkPos::new(2, -1));
        let pos = BlockPos::from_chunk_local(ChunkPos::new(2, -1), 5, 70, 9);
        assert_eq!(column.get(pos), Ok(Some(AIR_STATE)));
        assert!(column.set(pos, state(42)).expect("in range"));
        assert_eq!(column.get(pos), Ok(Some(state(42))));
        assert_eq!(column.non_air_count(), 1);

        // Section 4 covers y 64..80.
        let section = column.section(4).expect("exists");
        assert_eq!(section.get(LocalPos::from_axes(5, 6, 9)), state(42));

        assert!(column.set(pos, AIR_STATE).expect("in range"));
        assert_eq!(column.get(pos), Ok(Some(AIR_STATE)));
        assert_eq!(column.non_air_count(), 0);
    }

    #[test]
    fn rejects_positions_from_other_chunks() {
        let mut column = ChunkColumn::baseline(ChunkPos::new(0, 0));
        let pos = BlockPos::new(16, 0, 0);
        assert!(matches!(
            column.get(pos),
            Err(ChunkError::WrongChunk { .. })
        ));
        assert!(matches!(
            column.set(pos, state(1)),
            Err(ChunkError::WrongChunk { .. })
        ));
        // Negative coordinates belong to chunk -1, not chunk 0.
        assert!(matches!(
            column.get(BlockPos::new(-1, 0, 0)),
            Err(ChunkError::WrongChunk { .. })
        ));
    }

    #[test]
    fn vertical_range_is_enforced() {
        let mut column = ChunkColumn::new(ChunkPos::ORIGIN, 1).expect("one section");
        assert_eq!(column.height(), 16);
        assert_eq!(column.get(BlockPos::new(0, 16, 0)), Ok(None));
        assert_eq!(column.set(BlockPos::new(0, 16, 0), state(1)), Ok(false));
        assert_eq!(column.get(BlockPos::new(0, -1, 0)), Ok(None));
        assert_eq!(column.get(BlockPos::new(0, 15, 0)), Ok(Some(AIR_STATE)));
        assert!(
            column
                .set(BlockPos::new(0, 15, 0), state(1))
                .expect("in range")
        );
    }

    #[test]
    fn heightmap_tracks_writes_and_removals() {
        let mut column = ChunkColumn::baseline(ChunkPos::ORIGIN);
        assert_eq!(column.set(BlockPos::new(1, 10, 2), state(1)), Ok(true));
        assert_eq!(column.heightmap().get(1, 2), 11);

        assert_eq!(column.set(BlockPos::new(1, 40, 2), state(2)), Ok(true));
        assert_eq!(column.heightmap().get(1, 2), 41);

        // Removing the top block rescans downwards to the next solid block.
        assert_eq!(column.set(BlockPos::new(1, 40, 2), AIR_STATE), Ok(true));
        assert_eq!(column.heightmap().get(1, 2), 11);
        assert_eq!(column.set(BlockPos::new(1, 10, 2), AIR_STATE), Ok(true));
        assert_eq!(column.heightmap().get(1, 2), 0);

        // Writing air into an empty cell is a no-op.
        assert_eq!(column.set(BlockPos::new(1, 40, 2), AIR_STATE), Ok(true));
        assert_eq!(column.heightmap().get(1, 2), 0);
        assert_eq!(column.non_air_count(), 0);
    }

    #[test]
    fn recomputed_heightmap_matches_incremental_updates() {
        let mut column = ChunkColumn::baseline(ChunkPos::new(-4, 7));
        let mut expected = Heightmap::empty();
        for (i, (x, y, z)) in [(0, 0, 0), (3, 5, 4), (3, 200, 4), (15, 255, 15), (7, 64, 8)]
            .into_iter()
            .enumerate()
        {
            let pos = BlockPos::from_chunk_local(column.pos(), x, y, z);
            assert_eq!(column.set(pos, state(100 + i as u32)), Ok(true));
            let height = (y + 1) as u16;
            if height > expected.get(x, z) {
                expected.set(x, z, height);
            }
        }
        assert_eq!(column.heightmap(), &expected);

        let incremental = column.heightmap().clone();
        column.recompute_heightmap();
        assert_eq!(column.heightmap(), &incremental);
    }

    #[test]
    fn local_accessors_are_bounds_checked() {
        let mut column = ChunkColumn::baseline(ChunkPos::ORIGIN);
        let local = LocalPos::from_axes(1, 2, 3);
        assert_eq!(column.get_local(0, local), Some(AIR_STATE));
        assert_eq!(column.get_local(BASELINE_SECTIONS, local), None);
        assert!(column.section(BASELINE_SECTIONS as usize).is_none());
        assert!(column.section_mut(BASELINE_SECTIONS as usize).is_none());
    }

    proptest! {
        /// Writes, reads and the heightmap stay consistent with a plain array
        /// model, regardless of order or duplication.
        #[test]
        fn arbitrary_writes_match_a_plain_model(
            writes in proptest::collection::vec(
                (0i32..16, 0i32..256, 0i32..16, 0u32..64u32),
                0..600,
            )
        ) {
            let mut column = ChunkColumn::baseline(ChunkPos::new(5, 5));
            let mut reference = vec![AIR_STATE; 16 * 256 * 16];
            let index = |x: i32, y: i32, z: i32| {
                ((y as usize) << 8) | ((z as usize) << 4) | x as usize
            };
            for (x, y, z, id) in writes {
                let state = state(id);
                let pos = BlockPos::from_chunk_local(column.pos(), x, y, z);
                prop_assert_eq!(column.set(pos, state), Ok(true));
                reference[index(x, y, z)] = state;
            }
            for x in 0..16 {
                for y in 0..256 {
                    for z in 0..16 {
                        let pos = BlockPos::from_chunk_local(column.pos(), x, y, z);
                        prop_assert_eq!(column.get(pos).ok().flatten(), Some(reference[index(x, y, z)]));
                    }
                }
            }
            for x in 0..16 {
                for z in 0..16 {
                    let mut height = 0u16;
                    for y in 0..256 {
                        if reference[index(x, y, z)] != AIR_STATE {
                            height = (y + 1) as u16;
                        }
                    }
                    prop_assert_eq!(column.heightmap().get(x, z), height);
                }
            }
        }
    }
}
