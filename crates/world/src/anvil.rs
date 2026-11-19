//! Anvil region files (`r.X.Z.mca`): 32×32 chunks per file, 4 KiB sectors.
//!
//! The on-disk layout follows Java Edition Anvil, which is what external
//! inspection tools expect (PLAN.md M2 acceptance criteria):
//!
//! ```text
//! bytes 0..4096      location table: 1024 × u32 BE (sector offset << 8 | sector count)
//! bytes 4096..8192   timestamp table: 1024 × u32 BE (unix seconds)
//! bytes 8192..       chunk payloads, each beginning at a sector boundary:
//!                      u32 BE  compressed payload length (type byte included)
//!                      u8      compression type (1 = gzip, 2 = zlib, 3 = uncompressed)
//!                      ...     payload
//! ```
//!
//! We always write zlib (type 2). LCE's own saves use LZX/RLE (compression type
//! 3 with an Xbox-specific codec, `RegionFile.cpp`); importing those is a save
//! converter concern, tracked in `docs/reference/discrepancies.md`. Everything
//! else here is the standard format so Java tools can inspect our output.
//!
//! Writes are crash-safe in the Anvil sense: the payload is written to free
//! sectors first, then a single 4-byte location entry is updated. A crash
//! before that update leaves the old chunk readable; a crash during it is
//! detected as a corrupt/truncated chunk on read instead of being silently
//! accepted. All writes are flushed with `sync_data` before returning.

// Region math works in fixed 4 KiB sectors and 1024-entry tables; all casts
// below are bounded by the on-disk format itself.
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_lossless)]

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::read::{GzDecoder, ZlibDecoder};
use flate2::write::ZlibEncoder;

use crate::pos::ChunkPos;
use crate::pos::region::{REGION_CHUNKS, REGION_SHIFT, RegionPos};

/// Size of one region sector.
pub const SECTOR_BYTES: u64 = 4096;

/// Header words per region file (`4096 / 4`), and entries in each table.
pub const SECTOR_INTS: usize = 1024;

/// Bytes occupied by the two header sectors.
pub const HEADER_BYTES: u64 = SECTOR_BYTES * 2;

/// Maximum accepted (decompressed) chunk payload.
///
/// Chunk NBT is a few hundred kilobytes at most; 64 MiB is a hard sanity bound
/// so corrupt or hostile length fields cannot exhaust memory.
pub const MAX_CHUNK_BYTES: u64 = 64 * 1024 * 1024;

/// Chunk compression types on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CompressionType {
    /// Gzip stream (`java.util.zip.GZIPOutputStream`).
    Gzip = 1,
    /// Zlib stream (`java.util.zip.DeflaterOutputStream`).
    Zlib = 2,
    /// No compression.
    None = 3,
}

impl CompressionType {
    /// Parses the on-disk type byte.
    pub fn from_byte(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Gzip),
            2 => Some(Self::Zlib),
            3 => Some(Self::None),
            _ => None,
        }
    }
}

/// Errors from region file access.
#[derive(Debug, thiserror::Error)]
pub enum RegionError {
    /// Underlying file I/O failed.
    #[error("region I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// The file structure is invalid (bad header, bad length, ...).
    #[error("corrupt region file: {0}")]
    Corrupt(&'static str),
    /// The chunk payload is shorter than its header claims.
    #[error("chunk {x},{z} is truncated")]
    Truncated {
        /// Chunk X.
        x: i32,
        /// Chunk Z.
        z: i32,
    },
    /// The chunk uses an unsupported compression type.
    #[error("chunk {x},{z} uses unsupported compression type {compression}")]
    UnsupportedCompression {
        /// Chunk X.
        x: i32,
        /// Chunk Z.
        z: i32,
        /// The type byte that was read.
        compression: u8,
    },
    /// The chunk is larger than [`MAX_CHUNK_BYTES`].
    #[error("chunk {x},{z} exceeds {MAX_CHUNK_BYTES} bytes")]
    TooLarge {
        /// Chunk X.
        x: i32,
        /// Chunk Z.
        z: i32,
    },
    /// A chunk would need `sectors` sectors, more than the 255 an Anvil
    /// location entry can encode.
    #[error("chunk {x},{z} needs {sectors} sectors (maximum 255)")]
    ChunkTooLarge {
        /// Chunk X.
        x: i32,
        /// Chunk Z.
        z: i32,
        /// Required sectors.
        sectors: u64,
    },
}

/// The `r.X.Z.mca` file name for a region.
pub fn region_file_name(region: RegionPos) -> String {
    format!("r.{}.{}.mca", region.x, region.z)
}

/// The region position encoded by a path's file name, if it looks like a
/// region file (`r.<x>.<z>.mca`).
pub fn region_of_path(path: &Path) -> Option<RegionPos> {
    let name = path.file_name()?.to_str()?;
    let rest = name.strip_prefix("r.")?.strip_suffix(".mca")?;
    let (x, z) = rest.split_once('.')?;
    Some(RegionPos::new(x.parse().ok()?, z.parse().ok()?))
}

/// An open Anvil region file.
#[derive(Debug)]
pub struct RegionFile {
    path: PathBuf,
    file: File,
    region: RegionPos,
    locations: Vec<u32>,
    timestamps: Vec<u32>,
    /// Sector occupancy: `true` = in use (0 and 1 are the header).
    sectors: Vec<bool>,
}

impl RegionFile {
    /// Opens (creating if missing) the region file at `path`.
    ///
    /// A file shorter than the two header sectors but longer than zero bytes is
    /// rejected as corrupt; an empty file is treated as a new region and only
    /// gains its header on the first write (matching the reference, which
    /// avoids touching saves that are only read).
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, RegionError> {
        let path = path.into();
        let region =
            region_of_path(&path).ok_or(RegionError::Corrupt("file name is not r.<x>.<z>.mca"))?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)?;
        let len = file.metadata()?.len();
        let mut region_file = Self {
            path,
            file,
            region,
            locations: vec![0; SECTOR_INTS],
            timestamps: vec![0; SECTOR_INTS],
            sectors: Vec::new(),
        };
        if len == 0 {
            region_file.sectors = vec![true; 2];
            return Ok(region_file);
        }
        if len < HEADER_BYTES {
            return Err(RegionError::Corrupt("region header is truncated"));
        }

        let mut header = [0u8; HEADER_BYTES as usize];
        region_file.file.seek(SeekFrom::Start(0))?;
        region_file.file.read_exact(&mut header)?;
        for (i, word) in header[..SECTOR_BYTES as usize].chunks_exact(4).enumerate() {
            region_file.locations[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for (i, word) in header[SECTOR_BYTES as usize..].chunks_exact(4).enumerate() {
            region_file.timestamps[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }

        let sector_count = usize::try_from(len / SECTOR_BYTES).unwrap_or(usize::MAX);
        let mut sectors = vec![true; sector_count.max(2)];
        for location in &region_file.locations {
            let start = usize::try_from(location >> 8).unwrap_or(usize::MAX);
            let count = (*location & 0xFF) as usize;
            // Ignore entries that point outside the file: reading them reports
            // truncation, and they must not panic the allocation map.
            if *location != 0 && start >= 2 && start.saturating_add(count) <= sectors.len() {
                for sector in &mut sectors[start..start + count] {
                    *sector = true;
                }
            }
        }
        region_file.sectors = sectors;
        Ok(region_file)
    }

    /// The region file path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Number of 4 KiB sectors currently allocated in the file.
    pub fn sector_count(&self) -> usize {
        self.sectors.len()
    }

    /// Whether the region contains a chunk.
    pub fn contains(&self, chunk: ChunkPos) -> bool {
        self.locations[slot(chunk)] != 0
    }

    /// The stored modification time (unix seconds) of a chunk, if present.
    pub fn timestamp(&self, chunk: ChunkPos) -> Option<u32> {
        let location = self.locations[slot(chunk)];
        (location != 0).then(|| self.timestamps[slot(chunk)])
    }

    /// Reads and decompresses a chunk payload.
    ///
    /// `Ok(None)` means the region has no data for that chunk. Structural
    /// problems (truncation, bad length, unknown compression) are errors so the
    /// caller can log, back up, or skip — never silently lose data.
    pub fn read_chunk(&mut self, chunk: ChunkPos) -> Result<Option<Vec<u8>>, RegionError> {
        if chunk.x >> REGION_SHIFT != self.region().x || chunk.z >> REGION_SHIFT != self.region().z
        {
            return Err(RegionError::Corrupt("chunk does not belong to this region"));
        }
        let location = self.locations[slot(chunk)];
        if location == 0 {
            return Ok(None);
        }
        let start = u64::from(location >> 8);
        let count = u64::from(location & 0xFF);
        if count == 0 || start < 2 || start + count > self.sectors.len() as u64 {
            return Err(RegionError::Truncated {
                x: chunk.x,
                z: chunk.z,
            });
        }

        self.file.seek(SeekFrom::Start(start * SECTOR_BYTES))?;
        let mut length_bytes = [0u8; 4];
        if self.file.read_exact(&mut length_bytes).is_err() {
            return Err(RegionError::Truncated {
                x: chunk.x,
                z: chunk.z,
            });
        }
        let length = u64::from(u32::from_be_bytes(length_bytes));
        if length == 0 || length + 4 > count * SECTOR_BYTES || length > MAX_CHUNK_BYTES + 1 {
            return Err(RegionError::Truncated {
                x: chunk.x,
                z: chunk.z,
            });
        }

        let mut payload = vec![0u8; usize::try_from(length).unwrap_or(0)];
        self.file
            .read_exact(&mut payload)
            .map_err(|_| RegionError::Truncated {
                x: chunk.x,
                z: chunk.z,
            })?;

        let (type_byte, data) = payload.split_first().ok_or(RegionError::Truncated {
            x: chunk.x,
            z: chunk.z,
        })?;
        let compression =
            CompressionType::from_byte(*type_byte).ok_or(RegionError::UnsupportedCompression {
                x: chunk.x,
                z: chunk.z,
                compression: *type_byte,
            })?;
        decompress(compression, data, chunk).map(Some)
    }

    /// Compresses and writes a chunk payload, returning the sector it landed in.
    ///
    /// Existing chunks are replaced atomically from a reader's perspective: the
    /// new payload is written first and the location entry updated last.
    pub fn write_chunk(&mut self, chunk: ChunkPos, nbt: &[u8]) -> Result<u32, RegionError> {
        self.write_chunk_at(chunk, nbt, unix_now())
    }

    /// Like [`RegionFile::write_chunk`], with an explicit timestamp (tests use
    /// this so region files are byte-reproducible).
    pub fn write_chunk_at(
        &mut self,
        chunk: ChunkPos,
        nbt: &[u8],
        timestamp: u32,
    ) -> Result<u32, RegionError> {
        if chunk.x >> REGION_SHIFT != self.region().x || chunk.z >> REGION_SHIFT != self.region().z
        {
            return Err(RegionError::Corrupt("chunk does not belong to this region"));
        }
        let payload = compress(nbt)?;
        let length = u32::try_from(payload.len()).map_err(|_| RegionError::TooLarge {
            x: chunk.x,
            z: chunk.z,
        })?;
        let total = u64::from(length) + 4;
        let needed = total.div_ceil(SECTOR_BYTES);
        if needed > 255 {
            return Err(RegionError::ChunkTooLarge {
                x: chunk.x,
                z: chunk.z,
                sectors: needed,
            });
        }

        let start = self.allocate(needed as usize)?;
        let offset = u64::try_from(start).unwrap_or(u64::MAX) * SECTOR_BYTES;
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(&length.to_be_bytes())?;
        self.file.write_all(&payload)?;
        // Zero the rest of the final sector so freed space never leaks old data.
        let written = total + offset;
        let padded = written.div_ceil(SECTOR_BYTES) * SECTOR_BYTES;
        if padded > written {
            let zeros = vec![0u8; usize::try_from(padded - written).unwrap_or(0)];
            self.file.write_all(&zeros)?;
        }
        self.file.flush()?;

        // Publish the new location only after the payload is on disk.
        let index = slot(chunk);
        let previous = self.locations[index];
        let location =
            (u32::try_from(start).unwrap_or(0) << 8) | u32::try_from(needed).unwrap_or(0);
        self.locations[index] = location;
        self.timestamps[index] = timestamp;
        self.file
            .seek(SeekFrom::Start(u64::try_from(index).unwrap_or(0) * 4))?;
        self.file.write_all(&location.to_be_bytes())?;
        self.file.seek(SeekFrom::Start(
            SECTOR_BYTES + u64::try_from(index).unwrap_or(0) * 4,
        ))?;
        self.file.write_all(&timestamp.to_be_bytes())?;
        self.file.sync_data()?;

        // Release the replaced chunk's sectors now that nothing references them.
        let old_start = usize::try_from(previous >> 8).unwrap_or(usize::MAX);
        let old_count = (previous & 0xFF) as usize;
        if previous != 0
            && old_start != start
            && old_start >= 2
            && old_start.saturating_add(old_count) <= self.sectors.len()
        {
            self.sectors[old_start..old_start + old_count].fill(false);
        }

        u32::try_from(start).map_err(|_| RegionError::Corrupt("region file exceeds 4 GiB"))
    }

    /// Removes a chunk (frees its sectors, clears its header entry).
    pub fn delete_chunk(&mut self, chunk: ChunkPos) -> Result<bool, RegionError> {
        let index = slot(chunk);
        let location = self.locations[index];
        if location == 0 {
            return Ok(false);
        }
        let start = usize::try_from(location >> 8).unwrap_or(usize::MAX);
        let count = (location & 0xFF) as usize;
        if start >= 2 && start.saturating_add(count) <= self.sectors.len() {
            self.sectors[start..start + count].fill(false);
        }
        self.locations[index] = 0;
        self.timestamps[index] = 0;
        self.file
            .seek(SeekFrom::Start(u64::try_from(index).unwrap_or(0) * 4))?;
        self.file.write_all(&0u32.to_be_bytes())?;
        self.file.seek(SeekFrom::Start(
            SECTOR_BYTES + u64::try_from(index).unwrap_or(0) * 4,
        ))?;
        self.file.write_all(&0u32.to_be_bytes())?;
        self.file.sync_data()?;
        Ok(true)
    }

    /// Flushes and syncs the file.
    pub fn flush(&mut self) -> Result<(), RegionError> {
        self.file.flush()?;
        self.file.sync_data()?;
        Ok(())
    }

    /// The region this file holds.
    pub fn region(&self) -> RegionPos {
        self.region
    }

    /// Iterates over all loaded chunk positions in this region.
    pub fn loaded_chunks(&self) -> impl Iterator<Item = ChunkPos> + '_ {
        self.locations.iter().enumerate().filter_map(|(i, &loc)| {
            if loc != 0 {
                let x = i as i32 & 31;
                let z = i as i32 >> 5;
                Some(ChunkPos::new(
                    (self.region.x << REGION_SHIFT) + x,
                    (self.region.z << REGION_SHIFT) + z,
                ))
            } else {
                None
            }
        })
    }

    /// Finds `needed` contiguous free sectors (appending if necessary).
    fn allocate(&mut self, needed: usize) -> Result<usize, RegionError> {
        if self.sectors.len() < 2 {
            self.sectors = vec![true; 2];
        }
        let mut run_start = None;
        let mut run = 0usize;
        for (index, used) in self.sectors.iter().enumerate().skip(2) {
            if *used {
                run = 0;
                run_start = None;
                continue;
            }
            if run == 0 {
                run_start = Some(index);
            }
            run += 1;
            if run == needed {
                let start = run_start.unwrap_or(index + 1 - needed);
                self.sectors[start..start + needed].fill(true);
                return Ok(start);
            }
        }
        // No run: append at the end of the file.
        let start = self.sectors.len();
        self.sectors.resize(start + needed, true);
        let wanted = u64::try_from(self.sectors.len()).unwrap_or(u64::MAX) * SECTOR_BYTES;
        if self.file.metadata()?.len() < wanted {
            self.file.set_len(wanted)?;
        }
        Ok(start)
    }
}

impl Drop for RegionFile {
    fn drop(&mut self) {
        // Best effort; explicit `flush` is the contract for durability.
        let _ = self.file.flush();
    }
}

/// Header slot of a chunk inside its region (`local_z * 32 + local_x`).
fn slot(chunk: ChunkPos) -> usize {
    let (x, z) = RegionPos::local(chunk);
    debug_assert!((0..REGION_CHUNKS).contains(&x) && (0..REGION_CHUNKS).contains(&z));
    (z * REGION_CHUNKS + x) as usize
}

/// Compresses NBT bytes into an Anvil payload (type byte + zlib stream).
fn compress(nbt: &[u8]) -> Result<Vec<u8>, RegionError> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(nbt)?;
    let compressed = encoder.finish()?;
    let mut payload = Vec::with_capacity(compressed.len() + 1);
    payload.push(CompressionType::Zlib as u8);
    payload.extend_from_slice(&compressed);
    Ok(payload)
}

/// Decompresses an Anvil payload, enforcing [`MAX_CHUNK_BYTES`].
fn decompress(
    compression: CompressionType,
    data: &[u8],
    chunk: ChunkPos,
) -> Result<Vec<u8>, RegionError> {
    let reader: Box<dyn Read> = match compression {
        CompressionType::Gzip => Box::new(GzDecoder::new(data)),
        CompressionType::Zlib => Box::new(ZlibDecoder::new(data)),
        CompressionType::None => Box::new(data),
    };
    let mut out = Vec::new();
    // One byte over the limit is enough to detect an oversized payload.
    let mut limited = reader.take(MAX_CHUNK_BYTES + 1);
    limited
        .read_to_end(&mut out)
        .map_err(|_| RegionError::Corrupt("chunk stream failed to decompress"))?;
    if out.len() as u64 > MAX_CHUNK_BYTES {
        return Err(RegionError::TooLarge {
            x: chunk.x,
            z: chunk.z,
        });
    }
    Ok(out)
}

/// Current unix time in seconds (saturating; saves never fail on clock errors).
fn unix_now() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            u32::try_from(duration.as_secs()).unwrap_or(u32::MAX)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_region() -> (tempfile::TempDir, RegionFile) {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(region_file_name(RegionPos::new(0, 0)));
        let region = RegionFile::open(path).expect("open region");
        (dir, region)
    }

    #[test]
    fn empty_region_has_no_chunks_and_no_file_yet() {
        let (dir, mut region) = temp_region();
        assert_eq!(region.sector_count(), 2);
        assert!(!region.contains(ChunkPos::new(0, 0)));
        assert!(
            region
                .read_chunk(ChunkPos::new(0, 0))
                .expect("read")
                .is_none()
        );
        // Opening a new region does not create a header (reference behavior).
        let path = dir.path().join(region_file_name(RegionPos::new(0, 0)));
        assert_eq!(std::fs::metadata(&path).expect("file exists").len(), 0);
        region.flush().expect("flush");
    }

    #[test]
    fn chunk_round_trips_through_zlib() {
        let (_dir, mut region) = temp_region();
        let payload = b"\x0a\x00\x00some chunk nbt bytes".to_vec();
        let chunk = ChunkPos::new(5, 7);
        let sector = region.write_chunk_at(chunk, &payload, 1234).expect("write");
        assert!(sector >= 2);
        assert_eq!(region.timestamp(chunk), Some(1234));
        assert!(region.contains(chunk));
        assert_eq!(region.read_chunk(chunk).expect("read"), Some(payload));
    }

    #[test]
    fn region_survives_reopen() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(region_file_name(RegionPos::new(-1, 2)));
        let chunk = ChunkPos::new(-17, 65); // region (-1, 2), local (15, 1)
        {
            let mut region = RegionFile::open(&path).expect("open");
            region
                .write_chunk_at(chunk, b"persisted", 99)
                .expect("write");
        }
        let mut region = RegionFile::open(&path).expect("reopen");
        assert_eq!(
            region.read_chunk(chunk).expect("read"),
            Some(b"persisted".to_vec())
        );
        assert_eq!(region.timestamp(chunk), Some(99));
        assert_eq!(region.region(), RegionPos::new(-1, 2));
    }

    #[test]
    fn overwriting_reuses_free_sectors() {
        let (_dir, mut region) = temp_region();
        let chunk = ChunkPos::new(1, 1);
        region
            .write_chunk_at(chunk, b"first payload", 1)
            .expect("write");
        // The first replacement allocates a second sector before releasing the
        // old one; from then on the freed sector is reused.
        region
            .write_chunk_at(chunk, b"second payload", 2)
            .expect("write");
        let sectors_after_first_replace = region.sector_count();
        for _ in 0..16 {
            region
                .write_chunk_at(chunk, b"second payload", 2)
                .expect("write");
        }
        // Repeatedly replacing the same chunk must not grow the file.
        assert_eq!(region.sector_count(), sectors_after_first_replace);
        assert_eq!(
            region.read_chunk(chunk).expect("read"),
            Some(b"second payload".to_vec())
        );
    }

    #[test]
    fn delete_frees_the_chunk() {
        let (_dir, mut region) = temp_region();
        let chunk = ChunkPos::new(3, 3);
        region.write_chunk_at(chunk, b"x", 5).expect("write");
        assert!(region.delete_chunk(chunk).expect("delete"));
        assert!(!region.delete_chunk(chunk).expect("already gone"));
        assert!(region.read_chunk(chunk).expect("read").is_none());
        assert!(!region.contains(chunk));
    }

    #[test]
    fn rejects_chunks_from_another_region() {
        let (_dir, mut region) = temp_region();
        assert!(matches!(
            region.write_chunk_at(ChunkPos::new(32, 0), b"x", 0),
            Err(RegionError::Corrupt(_))
        ));
        assert!(matches!(
            region.read_chunk(ChunkPos::new(0, 32)),
            Err(RegionError::Corrupt(_))
        ));
    }

    #[test]
    fn truncated_file_is_an_error_not_a_panic() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(region_file_name(RegionPos::new(0, 0)));
        let mut region = RegionFile::open(&path).expect("open");
        let chunk = ChunkPos::new(0, 0);
        region
            .write_chunk_at(chunk, b"payload that occupies a sector", 1)
            .expect("write");
        drop(region);

        // Cut the file in the middle of the chunk sector.
        let file = OpenOptions::new()
            .write(true)
            .open(&path)
            .expect("reopen file");
        file.set_len(HEADER_BYTES + 10).expect("truncate");
        drop(file);

        let mut region = RegionFile::open(&path).expect("open truncated region");
        assert!(matches!(
            region.read_chunk(chunk),
            Err(RegionError::Truncated { .. })
        ));
    }

    #[test]
    fn corrupt_compression_byte_is_reported() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(region_file_name(RegionPos::new(0, 0)));
        let mut region = RegionFile::open(&path).expect("open");
        let chunk = ChunkPos::new(2, 4);
        let sector = region.write_chunk_at(chunk, b"hello", 1).expect("write");
        drop(region);

        // Overwrite the compression type byte (after the length prefix).
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .expect("open file");
        file.seek(SeekFrom::Start(u64::from(sector) * SECTOR_BYTES + 4))
            .expect("seek");
        file.write_all(&[9u8]).expect("corrupt");
        drop(file);

        let mut region = RegionFile::open(&path).expect("reopen");
        assert!(matches!(
            region.read_chunk(chunk),
            Err(RegionError::UnsupportedCompression { compression: 9, .. })
        ));
    }

    #[test]
    fn accepts_java_style_gzip_and_uncompressed_chunks() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(region_file_name(RegionPos::new(0, 0)));
        let chunk = ChunkPos::new(1, 0);
        let nbt = b"java chunk".to_vec();

        // Hand-build a region with a gzip chunk, as Java Edition writes it.
        let mut gzip = flate2::write::GzEncoder::new(Vec::new(), Compression::default());
        gzip.write_all(&nbt).expect("gzip");
        let gzip = gzip.finish().expect("gzip finish");
        let mut payload = vec![CompressionType::Gzip as u8];
        payload.extend_from_slice(&gzip);

        let mut file = std::fs::File::create(&path).expect("create");
        file.write_all(&[0u8; HEADER_BYTES as usize])
            .expect("header");
        let length = u32::try_from(payload.len()).expect("fits");
        file.write_all(&length.to_be_bytes()).expect("length");
        file.write_all(&payload).expect("payload");
        file.set_len(HEADER_BYTES + SECTOR_BYTES).expect("pad");
        let mut location = [0u8; 4];
        location.copy_from_slice(&((2u32 << 8) | 1).to_be_bytes());
        file.seek(SeekFrom::Start(slot(chunk) as u64 * 4))
            .expect("seek");
        file.write_all(&location).expect("location");
        drop(file);

        let mut region = RegionFile::open(&path).expect("open");
        assert_eq!(region.read_chunk(chunk).expect("read"), Some(nbt));
    }

    #[test]
    fn region_names_round_trip() {
        for region in [RegionPos::new(0, 0), RegionPos::new(-3, 12)] {
            let name = region_file_name(region);
            let path = PathBuf::from(&name);
            assert_eq!(region_of_path(&path), Some(region));
        }
        assert_eq!(region_of_path(Path::new("level.dat")), None);
        assert_eq!(region_of_path(Path::new("r.a.0.mca")), None);
    }
}
