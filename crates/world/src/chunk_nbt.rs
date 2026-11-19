//! Chunk NBT encoding: chunk columns ⇄ Anvil chunk compounds.
//!
//! The schema is the modern Java Edition one (1.13–1.17 shape), because that is
//! what external Anvil tools read:
//!
//! ```text
//! { DataVersion: Int
//!   Level: {
//!     xPos: Int, zPos: Int, Status: "minecraft:full", isLightOn: Byte(1)
//!     Heightmaps: { WORLD_SURFACE: LongArray(9-bit packed) }
//!     Sections: [ { Y: Byte
//!                   block_states: { palette: [ {Name, Properties} ], data: LongArray }
//!                   SkyLight: ByteArray(2048), BlockLight: ByteArray(2048) } ]
//!   } }
//! ```
//!
//! Palettes are namespaced ([`StateKey`]) and never contain numeric state ids
//! (SPEC.md 5.2). A single-valued section writes a one-entry palette and no
//! `data` array; direct sections (more than 256 distinct states) write `data`
//! without a palette, like Java Edition.
//!
//! Decoding treats the sections as authoritative: heightmaps are recomputed, so
//! a corrupt heightmap cannot corrupt block data, and unknown palette entries
//! are errors rather than a guess.

use mclre_core::nbt::{Compound, Value};

use crate::chunk::{ChunkColumn, ChunkError, MAX_SECTIONS};
use crate::pos::{CHUNK_SIZE, ChunkPos, SECTION_VOLUME};
use crate::section::{ChunkSection, NibbleArray, SectionStorage, StorageError, StorageKind};
use crate::state_key::{StateCodec, StateKey};

/// Bit width of a heightmap entry (`0..=256`).
const HEIGHTMAP_BITS: u8 = 9;

/// Heightmap entries per packed long (`64 / 9`).
const HEIGHTMAP_PER_LONG: usize = 7;

/// Heightmap entries per chunk column.
const HEIGHTMAP_ENTRIES: usize = (CHUNK_SIZE * CHUNK_SIZE) as usize;

/// Errors from chunk NBT encoding/decoding.
#[derive(Debug, thiserror::Error)]
pub enum ChunkNbtError {
    /// A required field is missing.
    #[error("chunk NBT is missing field `{0}`")]
    Missing(&'static str),
    /// A field has the wrong NBT type.
    #[error("chunk NBT field `{0}` has the wrong type")]
    WrongType(&'static str),
    /// The section list is empty or has an invalid `Y`.
    #[error("invalid section Y value {0}")]
    BadSectionY(i32),
    /// Two sections claim the same `Y`.
    #[error("duplicate section Y value {0}")]
    DuplicateSection(i32),
    /// A palette entry does not name a known block state.
    #[error("palette entry `{0}` is not a known block state")]
    UnknownState(String),
    /// The column could not be constructed from the decoded sections.
    #[error(transparent)]
    Column(#[from] ChunkError),
    /// Packed/palette storage was malformed.
    #[error(transparent)]
    Storage(#[from] StorageError),
    /// A chunk ran out of sections when writing.
    #[error("column has {0} sections (maximum {MAX_SECTIONS})")]
    TooManySections(usize),
    /// A state id had no namespaced representation (content mismatch).
    #[error("block state {0:?} has no palette key in this block-state model")]
    UnmappedState(mclre_core::BlockStateId),
}

/// Ordinary `Result` alias for chunk NBT operations.
pub type Result<T> = std::result::Result<T, ChunkNbtError>;

/// Encodes a chunk column into a chunk NBT compound (root name `""`).
///
/// `data_version` is the profile's chunk data version; the caller owns it so
/// this crate stays independent of `mclre-profiles`.
pub fn encode(column: &ChunkColumn, codec: &dyn StateCodec, data_version: i32) -> Result<Compound> {
    let mut level = Compound::new();
    let pos = column.pos();
    level.insert("xPos", Value::Int(pos.x));
    level.insert("zPos", Value::Int(pos.z));
    level.insert("Status", Value::String("minecraft:full".to_string()));
    level.insert("isLightOn", Value::Byte(1));

    // Heightmaps are advisory (readers must recompute if unsure) but external
    // tools use them for chunk selection, so write the real one.
    let mut heightmaps = Compound::new();
    heightmaps.insert("WORLD_SURFACE", Value::LongArray(pack_heightmap(column)));
    level.insert("Heightmaps", Value::Compound(heightmaps));

    let mut sections = Vec::with_capacity(column.sections().len());
    for (index, section) in column.sections().iter().enumerate() {
        sections.push(Value::Compound(encode_section(index, section, codec)?));
    }
    level.insert("Sections", Value::List(sections));

    let mut root = Compound::new();
    root.insert("DataVersion", Value::Int(data_version));
    root.insert("Level", Value::Compound(level));
    Ok(root)
}

/// Encodes one section at vertical index `index`.
fn encode_section(
    index: usize,
    section: &ChunkSection,
    codec: &dyn StateCodec,
) -> Result<Compound> {
    let mut out = Compound::new();
    out.insert("Y", Value::Byte(i8::try_from(index).unwrap_or(i8::MAX)));

    let storage = section.storage();
    let mut block_states = Compound::new();
    match storage.kind() {
        StorageKind::Single => {
            let state = section.get_index(0);
            block_states.insert(
                "palette",
                Value::List(vec![Value::Compound(encode_key(codec, state)?)]),
            );
        }
        StorageKind::Indirect => {
            let palette = storage.palette().unwrap_or(&[]);
            let bits = needed_bits(palette.len())?;
            let mut words = vec![0u64; crate::section::word_count(bits)];
            for i in 0..SECTION_VOLUME {
                let state = section.get_index(i);
                let slot = palette
                    .iter()
                    .position(|candidate| *candidate == state)
                    .ok_or(ChunkNbtError::UnmappedState(state))?;
                crate::section::pack(&mut words, i, bits, slot as u64);
            }
            let mut entries = Vec::with_capacity(palette.len());
            for state in palette {
                entries.push(Value::Compound(encode_key(codec, *state)?));
            }
            block_states.insert("palette", Value::List(entries));
            block_states.insert(
                "data",
                // NBT LongArray expects i64; values are guaranteed < 2^63 since they're palette indices/packed state ids
                #[allow(clippy::cast_possible_wrap)]
                Value::LongArray(words.iter().map(|word| *word as i64).collect()),
            );
        }
        StorageKind::Direct => {
            // Direct sections have no palette, so the width must be implicit
            // but deterministic: it is derived from the codec's state count,
            // exactly like Java Edition derives it from the registry size.
            let bits = direct_bits_for(codec.state_count());
            let mut words = vec![0u64; crate::section::word_count(bits)];
            for i in 0..SECTION_VOLUME {
                crate::section::pack(&mut words, i, bits, u64::from(section.get_index(i).0));
            }
            block_states.insert(
                "data",
                // NBT LongArray expects i64; values are guaranteed < 2^63 since they're palette indices/packed state ids
                #[allow(clippy::cast_possible_wrap)]
                Value::LongArray(words.iter().map(|word| *word as i64).collect()),
            );
        }
    }
    out.insert("block_states", Value::Compound(block_states));
    // Light values are 0..=15, well within i8 range
    #[allow(clippy::cast_possible_wrap)]
    let to_i8 = |bytes: &[u8]| -> Vec<i8> { bytes.iter().map(|byte| *byte as i8).collect() };

    out.insert(
        "SkyLight",
        Value::ByteArray(to_i8(section.sky_light_bytes())),
    );
    out.insert(
        "BlockLight",
        Value::ByteArray(to_i8(section.block_light_bytes())),
    );
    Ok(out)
}

/// Encodes a palette entry for one block state.
fn encode_key(codec: &dyn StateCodec, state: mclre_core::BlockStateId) -> Result<Compound> {
    let key = codec
        .key_of(state)
        .ok_or(ChunkNbtError::UnmappedState(state))?;
    Ok(key_to_value(&key))
}

/// Encodes a [`StateKey`] as `{ Name, Properties? }`.
fn key_to_value(key: &StateKey) -> Compound {
    let mut out = Compound::new();
    out.insert("Name", Value::String(key.name.clone()));
    if !key.properties.is_empty() {
        let mut properties = Compound::new();
        for (name, value) in &key.properties {
            properties.insert(name.clone(), Value::String(value.clone()));
        }
        out.insert("Properties", Value::Compound(properties));
    }
    out
}

/// Packs the column heightmap into 9-bit values, 7 per long.
fn pack_heightmap(column: &ChunkColumn) -> Vec<i64> {
    let map = column.heightmap();
    let mut words = vec![0u64; HEIGHTMAP_ENTRIES.div_ceil(HEIGHTMAP_PER_LONG)];
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            // CHUNK_SIZE is 16, so index fits in usize and is non-negative
            #[allow(clippy::cast_sign_loss)]
            let index = (z * CHUNK_SIZE + x) as usize;
            let value = u64::from(map.get(x, z));
            let word = index / HEIGHTMAP_PER_LONG;
            let shift = (index % HEIGHTMAP_PER_LONG) * HEIGHTMAP_BITS as usize;
            words[word] |= value << shift;
        }
    }
    // NBT LongArray expects i64; heightmap values are <= 256, so well within i64 range
    #[allow(clippy::cast_possible_wrap)]
    words.into_iter().map(|word| word as i64).collect()
}

/// Decodes a chunk compound into a column, using `codec` for palette entries.
pub fn decode(root: &Compound, codec: &dyn StateCodec) -> Result<ChunkColumn> {
    let level = root
        .get_compound("Level")
        .ok_or(ChunkNbtError::Missing("Level"))?;
    let x = int_field(level, "xPos")?;
    let z = int_field(level, "zPos")?;
    let pos = ChunkPos::new(x, z);

    let section_list = level
        .get_list("Sections")
        .ok_or(ChunkNbtError::Missing("Sections"))?;
    if section_list.is_empty() {
        return Err(ChunkNbtError::BadSectionY(0));
    }

    let mut decoded: Vec<Option<ChunkSection>> = Vec::new();
    let mut highest = 0i32;
    for entry in section_list {
        let Value::Compound(section) = entry else {
            return Err(ChunkNbtError::WrongType("Sections[]"));
        };
        let y = match section.get("Y") {
            Some(Value::Byte(y)) => i32::from(*y),
            Some(Value::Short(y)) => i32::from(*y),
            Some(Value::Int(y)) => *y,
            Some(_) => return Err(ChunkNbtError::WrongType("Sections[].Y")),
            None => return Err(ChunkNbtError::Missing("Sections[].Y")),
        };
        if y < 0 || y >= i32::from(MAX_SECTIONS) {
            return Err(ChunkNbtError::BadSectionY(y));
        }
        let index = usize::try_from(y).unwrap_or(usize::MAX);
        if index >= decoded.len() {
            decoded.resize_with(index + 1, || None);
        }
        if decoded[index].is_some() {
            return Err(ChunkNbtError::DuplicateSection(y));
        }
        decoded[index] = Some(decode_section(section, codec)?);
        highest = highest.max(y);
    }

    let mut sections = Vec::with_capacity(usize::try_from(highest + 1).unwrap_or(0));
    for entry in decoded
        .into_iter()
        .take(usize::try_from(highest + 1).unwrap_or(0))
    {
        // Missing section indices are all-air (Java omits empty sections).
        sections.push(entry.unwrap_or_else(ChunkSection::air));
    }
    ChunkColumn::from_sections(pos, sections).map_err(Into::into)
}

/// Decodes one section compound.
fn decode_section(section: &Compound, codec: &dyn StateCodec) -> Result<ChunkSection> {
    let block_states = section
        .get_compound("block_states")
        .ok_or(ChunkNbtError::Missing("Sections[].block_states"))?;
    let palette = match block_states.get_list("palette") {
        Some(entries) => {
            let mut palette = Vec::with_capacity(entries.len());
            for entry in entries {
                let Value::Compound(entry) = entry else {
                    return Err(ChunkNbtError::WrongType("block_states.palette[]"));
                };
                let name = entry
                    .get_str("Name")
                    .ok_or(ChunkNbtError::Missing("block_states.palette[].Name"))?;
                let mut properties = Vec::new();
                if let Some(Value::Compound(map)) = entry.get("Properties") {
                    for (key, value) in map.iter() {
                        let Value::String(value) = value else {
                            return Err(ChunkNbtError::WrongType(
                                "block_states.palette[].Properties",
                            ));
                        };
                        properties.push((key.to_string(), value.clone()));
                    }
                }
                let key = StateKey::new(name, properties);
                let state = codec
                    .state_of(&key)
                    .ok_or_else(|| ChunkNbtError::UnknownState(key.to_string()))?;
                palette.push(state);
            }
            Some(palette)
        }
        None => None,
    };

    let data = match block_states.get("data") {
        Some(Value::LongArray(words)) => Some(words),
        Some(_) => return Err(ChunkNbtError::WrongType("block_states.data")),
        None => None,
    };

    let storage = match (palette, data) {
        (Some(palette), None) => {
            if palette.len() != 1 {
                return Err(StorageError::MissingData(palette.len()).into());
            }
            SectionStorage::new_single(palette[0])
        }
        (Some(palette), Some(words)) => {
            let bits = needed_bits(palette.len())?;
            SectionStorage::from_indirect(palette, bits, words_to_u64(words))?
        }
        (None, Some(words)) => {
            let bits = direct_bits_for(codec.state_count());
            SectionStorage::from_direct(bits, words_to_u64(words))?
        }
        (None, None) => {
            // An all-air section may be written with neither palette nor data.
            SectionStorage::new_single(crate::pos::AIR_STATE)
        }
    };

    let sky_light = match section.get("SkyLight") {
        Some(Value::ByteArray(bytes)) => NibbleArray::from_bytes(&to_u8(bytes))?,
        Some(_) => return Err(ChunkNbtError::WrongType("Sections[].SkyLight")),
        None => NibbleArray::new(),
    };
    let block_light = match section.get("BlockLight") {
        Some(Value::ByteArray(bytes)) => NibbleArray::from_bytes(&to_u8(bytes))?,
        Some(_) => return Err(ChunkNbtError::WrongType("Sections[].BlockLight")),
        None => NibbleArray::new(),
    };
    Ok(ChunkSection::from_parts(storage, sky_light, block_light))
}

/// Minimum bits that can index a palette of `len` entries.
fn needed_bits(len: usize) -> Result<u8> {
    if len <= 1 {
        return Ok(4);
    }
    let needed = usize::BITS - (len - 1).leading_zeros();
    u8::try_from(needed.max(4)).map_err(|_| StorageError::PaletteTooLong(len).into())
}

/// Bits per entry for direct storage, derived from the number of registered
/// states (Java derives it from registry size the same way).
fn direct_bits_for(state_count: u32) -> u8 {
    if state_count == 0 {
        return crate::section::DIRECT_MIN_BITS;
    }
    let needed = crate::section::bits_for(state_count - 1);
    needed.max(crate::section::DIRECT_MIN_BITS)
}

/// Reinterprets stored longs as `u64` words.
/// NBT LongArray uses signed i64 but we need the bit pattern as u64.
#[allow(clippy::cast_sign_loss)]
fn words_to_u64(words: &[i64]) -> Vec<u64> {
    words.iter().map(|word| *word as u64).collect()
}

/// Reinterprets `TAG_Byte_Array` data as unsigned bytes.
/// NBT ByteArray uses signed i8 but we need the bit pattern as u8.
#[allow(clippy::cast_sign_loss)]
fn to_u8(bytes: &[i8]) -> Vec<u8> {
    bytes.iter().map(|byte| *byte as u8).collect()
}

fn int_field(compound: &Compound, key: &'static str) -> Result<i32> {
    match compound.get(key) {
        Some(Value::Int(value)) => Ok(*value),
        Some(_) => Err(ChunkNbtError::WrongType(key)),
        None => Err(ChunkNbtError::Missing(key)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pos::BlockPos;
    use crate::state_key::StateCodec;
    use mclre_core::{
        BlockProperties, BlockStateId, BlockStateModel, Identifier, PropertyDefinition, Registry,
    };

    fn model() -> BlockStateModel {
        let mut registry = Registry::new("minecraft:block");
        for (i, id) in ["minecraft:air", "minecraft:stone", "minecraft:oak_log"]
            .into_iter()
            .enumerate()
        {
            let properties = if i == 2 {
                BlockProperties::new(vec![PropertyDefinition::enumeration(
                    "axis",
                    ["x", "y", "z"],
                )])
                .expect("valid")
            } else {
                BlockProperties::none()
            };
            registry
                .register(Identifier::parse(id).expect("valid"), properties)
                .expect("register");
        }
        BlockStateModel::freeze(registry).expect("freeze")
    }

    fn stone() -> BlockStateId {
        BlockStateId(1)
    }

    fn build_column(model: &BlockStateModel) -> ChunkColumn {
        let mut column = ChunkColumn::new(ChunkPos::new(2, -3), 3).expect("column");
        let pos = BlockPos::from_chunk_local(column.pos(), 1, 5, 2);
        assert!(column.set(pos, stone()).expect("set"));
        let pos = BlockPos::from_chunk_local(column.pos(), 15, 47, 0);
        assert!(column.set(pos, BlockStateId(2)).expect("set"));
        let pos = BlockPos::from_chunk_local(column.pos(), 0, 0, 0);
        assert!(column.set(pos, BlockStateId(3)).expect("set"));
        // Exercise light persistence too.
        if let Some(section) = column.section_mut(0) {
            section.set_sky_light(crate::pos::LocalPos::from_axes(5, 6, 7), 15);
            section.set_block_light(crate::pos::LocalPos::from_axes(5, 6, 7), 9);
        }
        column
            .set(
                BlockPos::from_chunk_local(column.pos(), 3, 3, 3),
                model.default_state(mclre_core::BlockId(1)).expect("stone"),
            )
            .expect("set");
        column
    }

    #[test]
    fn column_round_trips_through_nbt() {
        let model = model();
        let codec: &dyn StateCodec = &model;
        let column = build_column(&model);
        let nbt = encode(&column, codec, 1).expect("encode");
        let decoded = decode(&nbt, codec).expect("decode");
        assert_eq!(decoded.pos(), column.pos());
        assert_eq!(decoded.section_count(), column.section_count());
        assert_eq!(decoded.heightmap(), column.heightmap());
        for y in 0..column.height() {
            for x in 0..16 {
                for z in 0..16 {
                    let pos = BlockPos::from_chunk_local(column.pos(), x, y, z);
                    assert_eq!(decoded.get(pos), column.get(pos), "at {pos}");
                }
            }
        }
        assert_eq!(
            decoded
                .section(0)
                .expect("section")
                .sky_light(crate::pos::LocalPos::from_axes(5, 6, 7)),
            15
        );
        assert_eq!(
            decoded
                .section(0)
                .expect("section")
                .block_light(crate::pos::LocalPos::from_axes(5, 6, 7)),
            9
        );
    }

    #[test]
    fn single_indirect_and_direct_palettes_round_trip() {
        let model = model();
        let codec: &dyn StateCodec = &model;
        // Single-value section (all stone): palette of one, no data array.
        // Use a pre-filled section to get true Single storage.
        let section = ChunkSection::filled(stone());
        let column = ChunkColumn::from_sections(ChunkPos::ORIGIN, vec![section]).expect("column");
        let nbt = encode(&column, codec, 1).expect("encode");
        let sections = nbt
            .get_compound("Level")
            .expect("level")
            .get_list("Sections")
            .expect("sections");
        let Value::Compound(section) = &sections[0] else {
            panic!("section is a compound");
        };
        let states = section.get_compound("block_states").expect("block_states");
        assert_eq!(states.get_list("palette").expect("palette").len(), 1);
        assert!(
            states.get("data").is_none(),
            "single-value sections omit data"
        );

        // Indirect: 200 distinct states.
        let mut column = ChunkColumn::new(ChunkPos::ORIGIN, 1).expect("column");
        for i in 0..200u32 {
            let pos = BlockPos::from_chunk_local(column.pos(), i as i32 % 16, 0, (i / 16) as i32);
            assert!(
                column
                    .set(pos, if i % 2 == 0 { stone() } else { BlockStateId(2) })
                    .expect("set")
            );
        }
        let nbt = encode(&column, codec, 1).expect("encode");
        let decoded = decode(&nbt, codec).expect("decode");
        for y in 0..16 {
            for x in 0..16 {
                for z in 0..16 {
                    let pos = BlockPos::from_chunk_local(ChunkPos::ORIGIN, x, y, z);
                    assert_eq!(decoded.get(pos), column.get(pos));
                }
            }
        }
    }

    #[test]
    fn unknown_palette_entries_are_rejected() {
        let model = model();
        let codec: &dyn StateCodec = &model;
        let root = Compound::from_entries(vec![(
            "Level".into(),
            Value::Compound(Compound::from_entries(vec![
                ("xPos".into(), Value::Int(0)),
                ("zPos".into(), Value::Int(0)),
                (
                    "Sections".into(),
                    Value::List(vec![Value::Compound(Compound::from_entries(vec![
                        ("Y".into(), Value::Byte(0)),
                        (
                            "block_states".into(),
                            Value::Compound(Compound::from_entries(vec![(
                                "palette".into(),
                                Value::List(vec![Value::Compound(Compound::from_entries(vec![(
                                    "Name".into(),
                                    Value::String("minecraft:not_real".into()),
                                )]))]),
                            )])),
                        ),
                    ]))]),
                ),
            ])),
        )]);
        assert!(matches!(
            decode(&root, codec),
            Err(ChunkNbtError::UnknownState(_))
        ));
    }

    #[test]
    fn missing_level_is_an_error() {
        assert!(matches!(
            decode(&Compound::new(), &model()),
            Err(ChunkNbtError::Missing("Level"))
        ));
    }

    #[test]
    fn duplicate_and_negative_sections_are_rejected() {
        let model = model();
        let codec: &dyn StateCodec = &model;
        let make = |ys: &[i8]| {
            Compound::from_entries(vec![(
                "Level".into(),
                Value::Compound(Compound::from_entries(vec![
                    ("xPos".into(), Value::Int(0)),
                    ("zPos".into(), Value::Int(0)),
                    (
                        "Sections".into(),
                        Value::List(
                            ys.iter()
                                .map(|y| {
                                    Value::Compound(Compound::from_entries(vec![
                                        ("Y".into(), Value::Byte(*y)),
                                        ("block_states".into(), Value::Compound(Compound::new())),
                                    ]))
                                })
                                .collect(),
                        ),
                    ),
                ])),
            )])
        };
        assert!(matches!(
            decode(&make(&[0, 0]), codec),
            Err(ChunkNbtError::DuplicateSection(0))
        ));
        assert!(matches!(
            decode(&make(&[-1]), codec),
            Err(ChunkNbtError::BadSectionY(-1))
        ));
    }
}
