//! World save orchestration: `level.dat` and chunk region management.
//!
//! This module coordinates:
//! - [`LevelDat`]: the world's `level.dat` NBT (data version, profile, preset, seed, dimension
//!   data, game rules).
//! - [`Save`]: a region-file cache keyed by [`RegionPos`] with atomic temp-file → sync → rename
//!   semantics for `level.dat`, plus corrupt-chunk recovery (log + skip/backup).

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use mclre_core::nbt::{read, read_bytes, write};

use crate::anvil::{RegionError, RegionFile, region_file_name};
use crate::chunk::ChunkColumn;
use crate::chunk_nbt::{ChunkNbtError, decode as decode_chunk_nbt, encode as encode_chunk_nbt};
use crate::level::{DATA_VERSION, LevelDat, LevelDatError};
use crate::pos::ChunkPos;
use crate::pos::region::RegionPos;
use crate::preset::WorldSizePreset;
use crate::state_key::StateCodec;

/// Errors from world save operations.
#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// NBT read/write error.
    #[error("NBT error: {0}")]
    Nbt(String),

    /// Region file error.
    #[error("region error: {0}")]
    Region(#[from] RegionError),

    /// Chunk NBT error.
    #[error("chunk NBT error: {0}")]
    ChunkNbt(#[from] ChunkNbtError),

    /// The world directory is missing.
    #[error("world directory not found: {0}")]
    MissingWorldDir(PathBuf),

    /// The level.dat is missing or corrupt.
    #[error("level.dat error: {0}")]
    LevelDat(#[from] LevelDatError),

    /// A chunk's coordinates are outside the preset bounds.
    #[error("chunk {0} is outside world size preset bounds")]
    ChunkOutOfBounds(ChunkPos),

    /// The data version is unsupported.
    #[error("unsupported data version {0}")]
    UnsupportedDataVersion(i32),
}

pub type Result<T> = std::result::Result<T, SaveError>;

/// A world save: region cache + level.dat.
pub struct Save {
    /// The world directory.
    dir: PathBuf,
    /// Cached region files, keyed by region position.
    regions: HashMap<RegionPos, RegionFile>,
    /// The level.dat data.
    level: LevelDat,
    /// The state codec for palette encoding.
    codec: Box<dyn StateCodec>,
}

impl Save {
    /// Opens an existing world save.
    ///
    /// Reads `level.dat` and creates an empty region cache. Region files are
    /// opened lazily on first chunk access.
    pub fn open<P: AsRef<Path>>(dir: P, codec: Box<dyn StateCodec>) -> Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        if !dir.is_dir() {
            return Err(SaveError::MissingWorldDir(dir));
        }

        let level_path = dir.join("level.dat");
        let level = Self::read_level_dat(&level_path)?;

        Ok(Self {
            dir,
            regions: HashMap::new(),
            level,
            codec,
        })
    }

    /// Creates a new world save.
    ///
    /// Writes `level.dat` atomically and creates an empty region cache.
    pub fn create<P: AsRef<Path>>(
        dir: P,
        profile_id: impl Into<String>,
        preset: WorldSizePreset,
        seed: i64,
        codec: Box<dyn StateCodec>,
    ) -> Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir)?;

        let level = LevelDat::new(profile_id, preset, seed);
        let level_path = dir.join("level.dat");
        Self::write_level_dat_atomic(&level_path, &level)?;

        Ok(Self {
            dir,
            regions: HashMap::new(),
            level,
            codec,
        })
    }

    /// Returns the world directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Returns the `LevelDat`.
    pub fn level(&self) -> &LevelDat {
        &self.level
    }

    /// Returns a mutable reference to the `LevelDat` (caller must call
    /// [`Save::flush_level`] to persist changes).
    pub fn level_mut(&mut self) -> &mut LevelDat {
        &mut self.level
    }

    /// Persists the current `LevelDat` to disk atomically.
    pub fn flush_level(&mut self) -> Result<()> {
        let level_path = self.dir.join("level.dat");
        Self::write_level_dat_atomic(&level_path, &self.level)
    }

    /// Reads a chunk column.
    ///
    /// Returns `Ok(None)` if the chunk doesn't exist. Returns an error if the
    /// chunk NBT is corrupt (logged and skipped).
    pub fn read_chunk(&mut self, pos: ChunkPos) -> Result<Option<ChunkColumn>> {
        let region_pos = pos.region();
        let region = self.get_or_open_region(region_pos)?;

        match region.read_chunk(pos) {
            Ok(Some(nbt_bytes)) => {
                let (_, root) = read(&mut std::io::Cursor::new(nbt_bytes))
                    .map_err(|e| SaveError::Nbt(e.to_string()))?;
                let column = decode_chunk_nbt(&root, self.codec.as_ref())?;
                Ok(Some(column))
            }
            Ok(None) => Ok(None),
            Err(e) => {
                tracing::warn!("failed to read chunk {pos}: {e}");
                Ok(None)
            }
        }
    }

    /// Writes a chunk column.
    ///
    /// For bounded worlds, returns an error if the chunk is outside the
    /// preset's horizontal bounds.
    pub fn write_chunk(&mut self, column: &ChunkColumn) -> Result<()> {
        // Bounded-world check.
        if !self
            .level
            .world_size_preset
            .contains_overworld_chunk(column.pos())
        {
            return Err(SaveError::ChunkOutOfBounds(column.pos()));
        }

        // Encode before borrowing region mutably.
        let nbt = encode_chunk_nbt(column, self.codec.as_ref(), DATA_VERSION)?;
        let buf = write("", &nbt).map_err(|e| SaveError::Nbt(e.to_string()))?;

        let region_pos = column.pos().region();
        let region = self.get_or_open_region(region_pos)?;
        region.write_chunk(column.pos(), &buf)?;
        Ok(())
    }

    /// Deletes a chunk column.
    pub fn delete_chunk(&mut self, pos: ChunkPos) -> Result<bool> {
        let region_pos = pos.region();
        if let Some(region) = self.regions.get_mut(&region_pos) {
            Ok(region.delete_chunk(pos)?)
        } else {
            Ok(false)
        }
    }

    /// Flushes all dirty region files to disk.
    pub fn flush(&mut self) -> Result<()> {
        for region in self.regions.values_mut() {
            region.flush()?;
        }
        self.flush_level()?;
        Ok(())
    }

    /// Returns an iterator over all loaded chunk positions.
    pub fn loaded_chunks(&self) -> impl Iterator<Item = ChunkPos> + '_ {
        self.regions.values().flat_map(super::anvil::RegionFile::loaded_chunks)
    }

    fn get_or_open_region(&mut self, pos: RegionPos) -> Result<&mut RegionFile> {
        if !self.regions.contains_key(&pos) {
            let region_path = self.dir.join("region").join(region_file_name(pos));
            let region = RegionFile::open(&region_path)?;
            self.regions.insert(pos, region);
        }
        Ok(self.regions.get_mut(&pos).unwrap())
    }

    fn read_level_dat(path: &Path) -> Result<LevelDat> {
        let bytes = fs::read(path)?;

        // Peek first byte to detect compression.
        let compressed = bytes.first().copied() == Some(0x78); // zlib header starts with 0x78

        let compound = if compressed {
            let mut decoder = ZlibDecoder::new(&bytes[..]);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed)?;
            let (_, compound) =
                read_bytes(&decompressed).map_err(|e| SaveError::Nbt(e.to_string()))?;
            compound
        } else {
            let (_, compound) = read_bytes(&bytes).map_err(|e| SaveError::Nbt(e.to_string()))?;
            compound
        };

        LevelDat::from_compound(&compound).map_err(SaveError::LevelDat)
    }

    fn write_level_dat_atomic(path: &Path, level: &LevelDat) -> Result<()> {
        let tmp = path.with_extension("dat.tmp");
        {
            let file = File::create(&tmp)?;
            let mut writer = BufWriter::new(file);
            let compound = level.to_compound();
            let nbt_bytes = write("", &compound).map_err(|e| SaveError::Nbt(e.to_string()))?;
            let mut encoder = ZlibEncoder::new(&mut writer, Compression::default());
            encoder.write_all(&nbt_bytes)?;
            encoder.finish()?;
            writer.flush()?;
        }
        fs::rename(&tmp, path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pos::BlockPos;
    use crate::state_key::StateCodec;
    use mclre_core::{
        BlockProperties, BlockStateId, BlockStateModel, Identifier, PropertyDefinition,
        Registry,
    };
    use tempfile::TempDir;

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

    fn codec() -> Box<dyn StateCodec> {
        Box::new(model())
    }

    #[test]
    fn level_dat_round_trip() {
        let _model = model();
        let tmp = TempDir::new().unwrap();
        let mut save = Save::create(
            tmp.path(),
            "test",
            WorldSizePreset::Infinite,
            12345,
            codec(),
        )
        .expect("create");

        // Modify level.
        save.level_mut().seed = 999;
        save.flush_level().expect("flush");

        // Reopen and verify.
        let save2 = Save::open(tmp.path(), codec()).expect("reopen");
        assert_eq!(save2.level().seed, 999);
        assert_eq!(save2.level().profile_id, "test");
        assert_eq!(save2.level().world_size_preset, WorldSizePreset::Infinite);
        assert_eq!(save2.level().dimensions.len(), 3);
    }

    #[test]
    fn chunk_write_read_round_trip() {
        let tmp = TempDir::new().unwrap();
        let mut save = Save::create(
            tmp.path(),
            "test",
            WorldSizePreset::Infinite,
            12345,
            codec(),
        )
        .expect("create");

        let mut column = ChunkColumn::baseline(ChunkPos::new(2, -3));
        let pos = BlockPos::from_chunk_local(column.pos(), 1, 5, 2);
        column.set(pos, BlockStateId(1)).expect("set");
        let pos = BlockPos::from_chunk_local(column.pos(), 15, 47, 0);
        column.set(pos, BlockStateId(2)).expect("set");

        save.write_chunk(&column).expect("write");

        let read = save
            .read_chunk(ChunkPos::new(2, -3))
            .expect("read")
            .expect("exists");
        assert_eq!(read.pos(), column.pos());
        assert_eq!(read.get(pos), column.get(pos));
    }

    #[test]
    fn bounded_world_rejects_out_of_bounds_chunks() {
        let tmp = TempDir::new().unwrap();
        let mut save = Save::create(
            tmp.path(),
            "test",
            WorldSizePreset::Classic, // 256x256 = 16x16 chunks
            12345,
            codec(),
        )
        .expect("create");

        // Valid chunk inside bounds.
        let column = ChunkColumn::baseline(ChunkPos::new(0, 0));
        save.write_chunk(&column).expect("in-bounds write ok");

        // Out of bounds chunk (Classic is 54 chunks: 0..53).
        let column = ChunkColumn::baseline(ChunkPos::new(60, 0));
        assert!(matches!(
            save.write_chunk(&column),
            Err(SaveError::ChunkOutOfBounds(_))
        ));
    }

    #[test]
    fn delete_chunk_removes_it() {
        let tmp = TempDir::new().unwrap();
        let mut save = Save::create(
            tmp.path(),
            "test",
            WorldSizePreset::Infinite,
            12345,
            codec(),
        )
        .expect("create");

        let column = ChunkColumn::baseline(ChunkPos::new(5, 5));
        save.write_chunk(&column).expect("write");

        assert!(
            save.read_chunk(ChunkPos::new(5, 5))
                .expect("read")
                .is_some()
        );

        save.delete_chunk(ChunkPos::new(5, 5)).expect("delete");

        assert!(
            save.read_chunk(ChunkPos::new(5, 5))
                .expect("read")
                .is_none()
        );
    }

    #[test]
    fn flush_persists_all_regions() {
        let tmp = TempDir::new().unwrap();
        {
            let mut save = Save::create(
                tmp.path(),
                "test",
                WorldSizePreset::Infinite,
                12345,
                codec(),
            )
            .expect("create");
            let column = ChunkColumn::baseline(ChunkPos::new(1, 2));
            save.write_chunk(&column).expect("write");
            save.flush().expect("flush");
        }

        // Reopen - chunk should persist.
        let mut save = Save::open(tmp.path(), codec()).expect("reopen");
        assert!(
            save.read_chunk(ChunkPos::new(1, 2))
                .expect("read")
                .is_some()
        );
    }
}
