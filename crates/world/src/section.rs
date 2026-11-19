//! Palette-compressed chunk-section storage (SPEC.md 5.6, PLAN.md M2).
//!
//! A section is `16×16×16 = 4096` block states in YZX order
//! ([`LocalPos::index`]), stored in one of three forms chosen automatically:
//!
//! | Form | When | Cost |
//! |---|---|---|
//! | [`StorageKind::Single`] | every cell holds the same state (typically air) | one state id |
//! | [`StorageKind::Indirect`] | `<= 256` distinct states | 4–8 bits/entry + palette |
//! | [`StorageKind::Direct`] | more than 256 distinct states | 15+ bits/entry, no palette |
//!
//! Entries are packed into `u64` words **without straddling** (each entry stays
//! inside one word), so reads/writes are a single mask-and-shift. This is a
//! deliberate format choice: deterministic, portable, and simple to verify.
//!
//! Light is stored as two 4-bit nibble arrays per section (sky + block light),
//! always present (the reference keeps light for generated chunks too).
//!
//! All transitions preserve existing content; there is no silent data loss.

// Bit packing is the point of this module; casts stay inside the 0..=32 bit
// domains established by `bits` fields.
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_lossless)]

use mclre_core::BlockStateId;

use crate::pos::{AIR_STATE, LocalPos, SECTION_VOLUME};

/// Nibble count in one light array (`16³ / 2` bytes).
const NIBBLES: usize = SECTION_VOLUME;

/// Maximum palette length before a section switches to direct storage.
const MAX_INDIRECT_PALETTE: usize = 256;

/// Bits per entry for direct storage when first needed.
pub const DIRECT_MIN_BITS: u8 = 15;

/// Maximum bits per entry supported by direct storage (`u32` state ids).
const DIRECT_MAX_BITS: u8 = 32;

/// The storage form of a section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageKind {
    /// One state fills the whole section.
    Single,
    /// A small palette plus bit-packed palette indices.
    Indirect,
    /// Bit-packed global state ids, no palette.
    Direct,
}

/// Errors from reconstructing storage from decoded palette data.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StorageError {
    /// The palette is empty.
    #[error("empty palette")]
    EmptyPalette,
    /// The palette exceeds 256 entries (that requires direct storage).
    #[error("palette has {0} entries (maximum 256)")]
    PaletteTooLong(usize),
    /// Bits per entry outside the supported range for the storage form.
    #[error("invalid bits per entry {bits} for {kind:?} storage")]
    BitsPerEntry {
        /// The rejected width.
        bits: u8,
        /// The storage form being built.
        kind: StorageKind,
    },
    /// A packed palette index points past the palette.
    #[error("packed index {index} is outside a palette of {len} entries")]
    PaletteIndex {
        /// The rejected index.
        index: u64,
        /// Palette length.
        len: usize,
    },
    /// The word array is too short for 4096 entries at the given width.
    #[error("packed array has {actual} words (expected {expected})")]
    WordsTooShort {
        /// Supplied words.
        actual: usize,
        /// Required words.
        expected: usize,
    },
    /// A light array is not the exact size of a section.
    #[error("light array has {actual} bytes (expected {expected})")]
    LightLength {
        /// Supplied bytes.
        actual: usize,
        /// Required bytes.
        expected: usize,
    },
    /// A section declares a palette but omits the packed data.
    #[error("section has a {0}-entry palette but no packed data")]
    MissingData(usize),
}

/// A 4096-nibble (4-bit) array, used for light values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NibbleArray {
    /// Two nibbles per byte; low nibble = even index.
    data: Vec<u8>,
}

impl NibbleArray {
    /// An all-zero array.
    pub fn new() -> Self {
        Self {
            data: vec![0; NIBBLES / 2],
        }
    }

    /// An array filled with `value` (must be `0..=15`).
    ///
    /// # Panics
    /// If `value > 15` (a caller bug; light levels are 4-bit).
    pub fn filled(value: u8) -> Self {
        assert!(value <= 15, "light value out of range");
        Self {
            data: vec![value | (value << 4); NIBBLES / 2],
        }
    }

    /// Reads the nibble at `index` (`0..4096`).
    ///
    /// # Panics
    /// If `index >= 4096` (a caller bug; section APIs validate positions).
    pub fn get(&self, index: usize) -> u8 {
        let byte = self.data[index >> 1];
        if index & 1 == 0 {
            byte & 0x0F
        } else {
            byte >> 4
        }
    }

    /// Writes `value` (`0..=15`) at `index` (`0..4096`).
    ///
    /// # Panics
    /// If `index >= 4096` or `value > 15` (caller bugs).
    pub fn set(&mut self, index: usize, value: u8) {
        assert!(value <= 15, "light value out of range");
        let byte = &mut self.data[index >> 1];
        if index & 1 == 0 {
            *byte = (*byte & 0xF0) | value;
        } else {
            *byte = (*byte & 0x0F) | (value << 4);
        }
    }

    /// Fills the whole array with `value`.
    ///
    /// # Panics
    /// If `value > 15` (light levels are 4-bit).
    pub fn fill(&mut self, value: u8) {
        assert!(value <= 15, "light value out of range");
        self.data.fill(value | (value << 4));
    }

    /// Raw bytes (used by save serialization).
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Builds an array from raw bytes (save deserialization).
    ///
    /// Exactly `2048` bytes are required; anything else is an error rather
    /// than a silent reshape.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, StorageError> {
        if bytes.len() != NIBBLES / 2 {
            return Err(StorageError::LightLength {
                actual: bytes.len(),
                expected: NIBBLES / 2,
            });
        }
        Ok(Self {
            data: bytes.to_vec(),
        })
    }
}

impl Default for NibbleArray {
    fn default() -> Self {
        Self::new()
    }
}

/// The block-state storage of one section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionStorage {
    /// Every cell holds `state`.
    Single {
        /// The repeated state.
        state: BlockStateId,
    },
    /// A palette plus bit-packed indices.
    Indirect {
        /// Distinct states present, in first-use order (deterministic).
        palette: Vec<BlockStateId>,
        /// Bits per packed index (4..=8).
        bits: u8,
        /// Packed indices, no straddling.
        words: Vec<u64>,
    },
    /// Bit-packed global state ids.
    Direct {
        /// Bits per packed state id (15..=32).
        bits: u8,
        /// Packed state ids, no straddling.
        words: Vec<u64>,
    },
}

/// Entries per `u64` for a given bit width (no straddling).
pub(crate) fn entries_per_word(bits: u8) -> usize {
    64 / bits as usize
}

/// Word count needed to hold 4096 entries at `bits` per entry.
pub(crate) fn word_count(bits: u8) -> usize {
    SECTION_VOLUME.div_ceil(entries_per_word(bits))
}

/// Minimum bits needed to represent `value`.
pub(crate) fn bits_for(value: u32) -> u8 {
    (32 - value.leading_zeros()) as u8
}

impl SectionStorage {
    /// A section filled with `state`.
    pub fn new_single(state: BlockStateId) -> Self {
        Self::Single { state }
    }

    /// Rebuilds indirect storage from a decoded palette and packed indices.
    ///
    /// Accepts widths `1..=8` (Java writes as few as 1 bit for tiny palettes);
    /// widths below 4 are re-packed to 4 so the in-memory invariant holds.
    /// Every packed index is validated against the palette, so corrupt data is
    /// an error rather than a panic.
    pub fn from_indirect(
        palette: Vec<BlockStateId>,
        bits: u8,
        words: Vec<u64>,
    ) -> Result<Self, StorageError> {
        if palette.is_empty() {
            return Err(StorageError::EmptyPalette);
        }
        if palette.len() > MAX_INDIRECT_PALETTE {
            return Err(StorageError::PaletteTooLong(palette.len()));
        }
        if !(1..=8).contains(&bits) {
            return Err(StorageError::BitsPerEntry {
                bits,
                kind: StorageKind::Indirect,
            });
        }
        let expected = word_count(bits);
        if words.len() < expected {
            return Err(StorageError::WordsTooShort {
                actual: words.len(),
                expected,
            });
        }
        for index in 0..SECTION_VOLUME {
            let raw = unpack(&words, index, bits);
            if raw as usize >= palette.len() {
                return Err(StorageError::PaletteIndex {
                    index: raw,
                    len: palette.len(),
                });
            }
        }
        if bits < 4 {
            // Normalize tiny palettes to the 4-bit minimum we ever write.
            let new_bits = 4;
            let mut repacked = vec![0u64; word_count(new_bits)];
            for index in 0..SECTION_VOLUME {
                let raw = unpack(&words, index, bits);
                pack(&mut repacked, index, new_bits, raw);
            }
            return Ok(Self::Indirect {
                palette,
                bits: new_bits,
                words: repacked,
            });
        }
        Ok(Self::Indirect {
            palette,
            bits,
            words: words[..expected].to_vec(),
        })
    }

    /// Rebuilds direct storage from packed global state ids.
    ///
    /// Accepts widths `1..=32`; values are validated to fit in `u32`.
    pub fn from_direct(bits: u8, words: Vec<u64>) -> Result<Self, StorageError> {
        if !(1..=DIRECT_MAX_BITS).contains(&bits) {
            return Err(StorageError::BitsPerEntry {
                bits,
                kind: StorageKind::Direct,
            });
        }
        let expected = word_count(bits);
        if words.len() < expected {
            return Err(StorageError::WordsTooShort {
                actual: words.len(),
                expected,
            });
        }
        if bits < DIRECT_MIN_BITS {
            let new_bits = DIRECT_MIN_BITS;
            let mut repacked = vec![0u64; word_count(new_bits)];
            for index in 0..SECTION_VOLUME {
                let raw = unpack(&words, index, bits);
                pack(&mut repacked, index, new_bits, raw);
            }
            return Ok(Self::Direct {
                bits: new_bits,
                words: repacked,
            });
        }
        Ok(Self::Direct {
            bits,
            words: words[..expected].to_vec(),
        })
    }

    /// The storage form.
    pub fn kind(&self) -> StorageKind {
        match self {
            Self::Single { .. } => StorageKind::Single,
            Self::Indirect { .. } => StorageKind::Indirect,
            Self::Direct { .. } => StorageKind::Direct,
        }
    }

    /// Bits per entry (`0` for single-valued storage).
    pub fn bits_per_entry(&self) -> u8 {
        match self {
            Self::Single { .. } => 0,
            Self::Indirect { bits, .. } | Self::Direct { bits, .. } => *bits,
        }
    }

    /// The palette for indirect storage (`None` for single/direct).
    pub fn palette(&self) -> Option<&[BlockStateId]> {
        match self {
            Self::Indirect { palette, .. } => Some(palette),
            _ => None,
        }
    }

    /// Reads the state at `index` (`0..4096`).
    ///
    /// # Panics
    /// If `index >= 4096` (caller bug).
    pub fn get_index(&self, index: usize) -> BlockStateId {
        assert!(index < SECTION_VOLUME, "section index out of range");
        match self {
            Self::Single { state } => *state,
            Self::Indirect {
                palette,
                bits,
                words,
            } => {
                let raw = unpack(words, index, *bits);
                palette[raw as usize]
            }
            Self::Direct { bits, words } => BlockStateId(unpack(words, index, *bits) as u32),
        }
    }

    /// Reads the state at a section-local position.
    pub fn get(&self, pos: LocalPos) -> BlockStateId {
        self.get_index(pos.index())
    }

    /// Writes `state` at `index` (`0..4096`), growing the storage as needed.
    ///
    /// # Panics
    /// If `index >= 4096` (caller bug).
    pub fn set_index(&mut self, index: usize, state: BlockStateId) {
        match self {
            Self::Single { state: current } => {
                if *current == state {
                    return;
                }
                // Promote to indirect: palette index 0 is the old value, and
                // every existing cell already reads as index 0.
                let bits = 4;
                let mut words = vec![0u64; word_count(bits)];
                pack(&mut words, index, bits, 1);
                *self = Self::Indirect {
                    palette: vec![*current, state],
                    bits,
                    words,
                };
            }
            Self::Indirect {
                palette,
                bits,
                words,
            } => {
                if let Some(slot) = palette.iter().position(|s| *s == state) {
                    pack(words, index, *bits, slot as u64);
                    return;
                }
                if palette.len() < MAX_INDIRECT_PALETTE {
                    palette.push(state);
                    let needed = bits_for((palette.len() - 1) as u32).max(4);
                    if needed > *bits {
                        // Grow bit width, preserving existing indices.
                        let new_bits = needed;
                        let mut new_words = vec![0u64; word_count(new_bits)];
                        for i in 0..SECTION_VOLUME {
                            let old = unpack(words, i, *bits);
                            pack(&mut new_words, i, new_bits, old);
                        }
                        *words = new_words;
                        *bits = new_bits;
                    }
                    pack(words, index, *bits, (palette.len() - 1) as u64);
                } else {
                    // More than 256 distinct states: switch to direct storage.
                    // The bit width must cover the largest state id already in
                    // the palette as well as the incoming one.
                    let widest = palette.iter().map(|s| s.0).max().unwrap_or(0);
                    let new_bits = DIRECT_MIN_BITS.max(bits_for(widest.max(state.0)));
                    let mut direct_words = vec![0u64; word_count(new_bits)];
                    for i in 0..SECTION_VOLUME {
                        let raw = unpack(words, i, *bits);
                        pack(
                            &mut direct_words,
                            i,
                            new_bits,
                            palette[raw as usize].0 as u64,
                        );
                    }
                    pack(&mut direct_words, index, new_bits, state.0 as u64);
                    *self = Self::Direct {
                        bits: new_bits,
                        words: direct_words,
                    };
                }
            }
            Self::Direct { bits, words } => {
                let needed = bits_for(state.0);
                if needed > *bits {
                    let new_bits = needed.min(DIRECT_MAX_BITS);
                    debug_assert!(needed <= new_bits, "state id exceeds direct storage");
                    let mut new_words = vec![0u64; word_count(new_bits)];
                    for i in 0..SECTION_VOLUME {
                        let old = unpack(words, i, *bits);
                        pack(&mut new_words, i, new_bits, old);
                    }
                    *words = new_words;
                    *bits = new_bits;
                }
                pack(words, index, *bits, state.0 as u64);
            }
        }
    }

    /// Writes `state` at a section-local position.
    pub fn set(&mut self, pos: LocalPos, state: BlockStateId) {
        self.set_index(pos.index(), state);
    }

    /// Fills the whole section with `state` (single-valued form).
    pub fn fill(&mut self, state: BlockStateId) {
        *self = Self::Single { state };
    }

    /// Iterates every state in index order.
    pub fn iter(&self) -> impl Iterator<Item = BlockStateId> + '_ {
        (0..SECTION_VOLUME).map(|index| self.get_index(index))
    }
}

/// Reads one packed entry.
pub(crate) fn unpack(words: &[u64], index: usize, bits: u8) -> u64 {
    let per_word = entries_per_word(bits);
    let word = words[index / per_word];
    let shift = (index % per_word) * bits as usize;
    let mask = (1u64 << bits) - 1;
    (word >> shift) & mask
}

/// Writes one packed entry (`value` must fit in `bits`).
pub(crate) fn pack(words: &mut [u64], index: usize, bits: u8, value: u64) {
    let per_word = entries_per_word(bits);
    let shift = (index % per_word) * bits as usize;
    let mask = (1u64 << bits) - 1;
    let word = &mut words[index / per_word];
    *word = (*word & !(mask << shift)) | ((value & mask) << shift);
}

/// One chunk section: block states plus sky/block light.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkSection {
    storage: SectionStorage,
    sky_light: NibbleArray,
    block_light: NibbleArray,
    non_air: u16,
}

impl ChunkSection {
    /// An all-air, dark section (light arrays zeroed).
    pub fn air() -> Self {
        Self {
            storage: SectionStorage::new_single(AIR_STATE),
            sky_light: NibbleArray::new(),
            block_light: NibbleArray::new(),
            non_air: 0,
        }
    }

    /// A section filled with `state`.
    pub fn filled(state: BlockStateId) -> Self {
        let non_air = if state == AIR_STATE {
            0
        } else {
            SECTION_VOLUME as u16
        };
        Self {
            storage: SectionStorage::new_single(state),
            sky_light: NibbleArray::new(),
            block_light: NibbleArray::new(),
            non_air,
        }
    }

    /// Rebuilds a section from decoded storage and light arrays.
    ///
    /// The non-air count is recomputed from the storage, so a save can never
    /// make it disagree with the blocks.
    pub fn from_parts(
        storage: SectionStorage,
        sky_light: NibbleArray,
        block_light: NibbleArray,
    ) -> Self {
        let non_air = u16::try_from(storage.iter().filter(|state| *state != AIR_STATE).count())
            .unwrap_or(u16::MAX);
        Self {
            storage,
            sky_light,
            block_light,
            non_air,
        }
    }

    /// The block-state at a local position.
    pub fn get(&self, pos: LocalPos) -> BlockStateId {
        self.storage.get(pos)
    }

    /// The block-state at a linear index.
    pub fn get_index(&self, index: usize) -> BlockStateId {
        self.storage.get_index(index)
    }

    /// Sets the block-state at a local position.
    pub fn set(&mut self, pos: LocalPos, state: BlockStateId) {
        let index = pos.index();
        let previous = self.storage.get_index(index);
        if previous == state {
            return;
        }
        let was_air = previous == AIR_STATE;
        let is_air = state == AIR_STATE;
        self.storage.set_index(index, state);
        match (was_air, is_air) {
            (true, false) => self.non_air += 1,
            (false, true) => self.non_air -= 1,
            _ => {}
        }
    }

    /// Number of non-air blocks in the section.
    pub fn non_air_count(&self) -> u16 {
        self.non_air
    }

    /// Whether the section contains only air.
    pub fn is_empty(&self) -> bool {
        self.non_air == 0
    }

    /// The block-state storage (palette access for serialization).
    pub fn storage(&self) -> &SectionStorage {
        &self.storage
    }

    /// Sky light at a local position (`0..=15`).
    pub fn sky_light(&self, pos: LocalPos) -> u8 {
        self.sky_light.get(pos.index())
    }

    /// Sets sky light at a local position (`0..=15`).
    pub fn set_sky_light(&mut self, pos: LocalPos, value: u8) {
        self.sky_light.set(pos.index(), value);
    }

    /// Block light at a local position (`0..=15`).
    pub fn block_light(&self, pos: LocalPos) -> u8 {
        self.block_light.get(pos.index())
    }

    /// Sets block light at a local position (`0..=15`).
    pub fn set_block_light(&mut self, pos: LocalPos, value: u8) {
        self.block_light.set(pos.index(), value);
    }

    /// The raw sky-light array (save serialization).
    pub fn sky_light_bytes(&self) -> &[u8] {
        self.sky_light.as_bytes()
    }

    /// The raw block-light array (save serialization).
    pub fn block_light_bytes(&self) -> &[u8] {
        self.block_light.as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn state(id: u32) -> BlockStateId {
        BlockStateId(id)
    }

    fn pos(x: usize, y: usize, z: usize) -> LocalPos {
        LocalPos::from_axes(x as u8, y as u8, z as u8)
    }

    #[test]
    fn single_storage_reads_everywhere() {
        let storage = SectionStorage::new_single(state(7));
        for index in [0, 1, 15, 16, 1000, SECTION_VOLUME - 1] {
            assert_eq!(storage.get_index(index), state(7));
        }
        assert_eq!(storage.kind(), StorageKind::Single);
        assert_eq!(storage.bits_per_entry(), 0);
    }

    #[test]
    fn single_to_indirect_preserves_existing_cells() {
        let mut storage = SectionStorage::new_single(state(3));
        storage.set_index(5, state(9));
        assert_eq!(storage.kind(), StorageKind::Indirect);
        assert_eq!(storage.bits_per_entry(), 4);
        assert_eq!(storage.get_index(5), state(9));
        assert_eq!(storage.get_index(0), state(3));
        assert_eq!(storage.get_index(SECTION_VOLUME - 1), state(3));
        assert_eq!(storage.palette(), Some(&[state(3), state(9)][..]));
    }

    #[test]
    fn indirect_grows_bits_at_seventeen_distinct_states() {
        let mut storage = SectionStorage::new_single(AIR_STATE);
        // Palette starts as [air]; 15 distinct non-air states fill it to 16
        // entries, which still fits in 4 bits.
        for i in 0..15u32 {
            storage.set_index(i as usize, state(100 + i));
        }
        assert_eq!(storage.kind(), StorageKind::Indirect);
        assert_eq!(storage.bits_per_entry(), 4);
        assert_eq!(storage.palette().expect("indirect").len(), 16);

        // The 17th distinct state forces a wider bit width.
        storage.set_index(15, state(200));
        assert_eq!(storage.kind(), StorageKind::Indirect);
        assert_eq!(storage.bits_per_entry(), 5);
        assert_eq!(storage.palette().expect("indirect").len(), 17);
        for i in 0..15u32 {
            assert_eq!(storage.get_index(i as usize), state(100 + i), "index {i}");
        }
        assert_eq!(storage.get_index(15), state(200));
        assert_eq!(storage.get_index(16), AIR_STATE);
    }

    #[test]
    fn indirect_switches_to_direct_above_256_states() {
        let mut storage = SectionStorage::new_single(AIR_STATE);
        let mut expected = vec![AIR_STATE; SECTION_VOLUME];
        // 300 distinct non-air states in the first 300 cells.
        for i in 0..300u32 {
            storage.set_index(i as usize, state(1000 + i));
            expected[i as usize] = state(1000 + i);
        }
        assert_eq!(storage.kind(), StorageKind::Direct);
        assert!(storage.bits_per_entry() >= DIRECT_MIN_BITS);
        for (index, want) in expected.iter().enumerate() {
            assert_eq!(storage.get_index(index), *want, "index {index}");
        }
    }

    #[test]
    fn direct_grows_for_large_state_ids() {
        let mut storage = SectionStorage::new_single(AIR_STATE);
        // A large id alone does not force direct storage: palettes hold any id.
        storage.set_index(0, state(70_000));
        assert_eq!(storage.kind(), StorageKind::Indirect);
        assert_eq!(storage.get_index(0), state(70_000));

        // More than 256 distinct states forces direct storage, whose width
        // must already cover the widest palette entry.
        for i in 0..260u32 {
            storage.set_index(1 + i as usize, state(1 + i));
        }
        assert_eq!(storage.kind(), StorageKind::Direct);
        assert_eq!(storage.bits_per_entry(), 17); // 70_000 needs 17 bits
        assert_eq!(storage.get_index(0), state(70_000));
        assert_eq!(storage.get_index(1), state(1));
        assert_eq!(storage.get_index(260), state(260));
        assert_eq!(storage.get_index(4095), AIR_STATE);

        storage.set_index(4095, state(u32::MAX));
        assert_eq!(storage.get_index(4095), state(u32::MAX));
        assert_eq!(storage.get_index(0), state(70_000));
        assert_eq!(storage.bits_per_entry(), 32);
    }

    #[test]
    fn chunk_section_tracks_non_air_and_light() {
        let mut section = ChunkSection::air();
        assert!(section.is_empty());
        section.set(pos(1, 2, 3), state(5));
        assert_eq!(section.non_air_count(), 1);
        assert!(!section.is_empty());
        section.set(pos(1, 2, 3), state(5));
        assert_eq!(section.non_air_count(), 1);
        section.set(pos(1, 2, 3), AIR_STATE);
        assert_eq!(section.non_air_count(), 0);
        assert!(section.is_empty());

        section.set(pos(0, 0, 0), state(9));
        section.set_sky_light(pos(0, 0, 0), 15);
        section.set_block_light(pos(0, 0, 0), 3);
        assert_eq!(section.sky_light(pos(0, 0, 0)), 15);
        assert_eq!(section.block_light(pos(0, 0, 0)), 3);
        assert_eq!(section.sky_light(pos(1, 0, 0)), 0);

        let filled = ChunkSection::filled(state(1));
        assert_eq!(filled.non_air_count(), SECTION_VOLUME as u16);
        assert!(!filled.is_empty());
    }

    #[test]
    fn nibble_array_round_trips_every_index() {
        let mut array = NibbleArray::new();
        for index in 0..NIBBLES {
            array.set(index, (index % 16) as u8);
        }
        for index in 0..NIBBLES {
            assert_eq!(array.get(index), (index % 16) as u8);
        }
        array.fill(7);
        assert!((0..NIBBLES).all(|i| array.get(i) == 7));
        assert_eq!(array.as_bytes().len(), NIBBLES / 2);
    }

    #[test]
    fn packing_never_straddles_words() {
        // Entries keep their value across a word boundary for every width we use.
        for bits in [4u8, 5, 6, 7, 8, 15, 17] {
            let per_word = entries_per_word(bits);
            let boundary = per_word - 1;
            let mut words = vec![0u64; word_count(bits)];
            pack(&mut words, boundary, bits, (1u64 << bits) - 1);
            pack(&mut words, per_word, bits, 1);
            assert_eq!(unpack(&words, boundary, bits), (1u64 << bits) - 1);
            assert_eq!(unpack(&words, per_word, bits), 1);
            assert_eq!(words.len(), word_count(bits));
        }
    }

    // --- property tests -----------------------------------------------------

    proptest! {
        /// Any sequence of writes behaves exactly like a plain array, and the
        /// storage always reports a consistent non-air count.
        #[test]
        fn arbitrary_writes_match_a_plain_array(
            writes in proptest::collection::vec((0usize..SECTION_VOLUME, 0u32..400u32), 0..400)
        ) {
            let mut storage = SectionStorage::new_single(AIR_STATE);
            let mut reference = vec![AIR_STATE; SECTION_VOLUME];
            for (index, value) in writes {
                let value = state(value);
                storage.set_index(index, value);
                reference[index] = value;
            }
            for (index, want) in reference.iter().enumerate() {
                prop_assert_eq!(storage.get_index(index), *want, "index {}", index);
            }
            prop_assert_eq!(
                storage.iter().filter(|s| *s != AIR_STATE).count(),
                reference.iter().filter(|s| **s != AIR_STATE).count()
            );
        }

        /// Storing many distinct states always ends in direct storage and
        /// remains readable.
        #[test]
        fn distinct_state_flood_reads_back(count in 257usize..=4096) {
            let mut storage = SectionStorage::new_single(AIR_STATE);
            let mut reference = vec![AIR_STATE; SECTION_VOLUME];
            for (index, slot) in reference.iter_mut().enumerate().take(count) {
                let value = state(1 + index as u32);
                storage.set_index(index, value);
                *slot = value;
            }
            prop_assert_eq!(storage.kind(), StorageKind::Direct);
            for (index, want) in reference.iter().enumerate() {
                prop_assert_eq!(storage.get_index(index), *want);
            }
        }
    }
}
