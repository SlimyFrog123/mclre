//! World-size presets and dimension scaling (SPEC.md 5.6, PLAN.md M2).
//!
//! Values are verified reference data (`docs/reference/baseline.md` §2,
//! `Minecraft.World/ChunkSource.h:8-49`):
//!
//! | Preset | Overworld chunks | Blocks | Nether scale | Nether chunks | Edge moat |
//! |---|---|---|---|---|---|
//! | `classic` | 54 | 864 | 3 | 18 | yes |
//! | `small` | 64 | 1024 | 3 | 21 | yes |
//! | `medium` | 192 | 3072 | 6 | 32 | yes |
//! | `large` | 320 | 5120 | 8 | 40 | no (ocean edge) |
//! | `infinite` | unbounded | unbounded | 8 (Java-style) | unbounded | no |
//!
//! The End is fixed at 18×18 chunks for every preset
//! (`END_LEVEL_MAX_WIDTH 18`, "Fix the size of the end for all platforms").
//!
//! Finite worlds occupy chunk coordinates `0..width` in both axes (the
//! reference allocates one bounded coordinate space starting at zero and
//! offsets dimensions inside it). Negative coordinates are therefore invalid
//! in finite presets and perfectly valid in `infinite`.
//!
//! `infinite`'s Nether scale follows Java Edition (8); ADR 0004 tracks whether
//! that is what the reference does, and it does not affect any finite preset.

use std::fmt;
use std::str::FromStr;

use crate::pos::{BlockPos, CHUNK_SIZE, ChunkPos};

/// A console world-size preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WorldSizePreset {
    /// Smallest preset (PS3/360 default): 54×54 chunks = 864×864 blocks.
    Classic,
    /// 64×64 chunks = 1024×1024 blocks.
    Small,
    /// 192×192 chunks = 3072×3072 blocks.
    Medium,
    /// 320×320 chunks = 5120×5120 blocks.
    Large,
    /// Unbounded (modern-style) world.
    Infinite,
}

impl WorldSizePreset {
    /// Every preset, smallest finite to unbounded.
    pub const ALL: &'static [Self] = &[
        Self::Classic,
        Self::Small,
        Self::Medium,
        Self::Large,
        Self::Infinite,
    ];

    /// Stable lowercase key used in saves and configuration.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
            Self::Infinite => "infinite",
        }
    }

    /// Parses a preset key (case-sensitive, as stored in saves).
    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|p| p.as_str() == key)
    }

    /// Width in chunks for the Overworld, or `None` when unbounded.
    pub const fn chunk_width(self) -> Option<u32> {
        match self {
            Self::Classic => Some(54),
            Self::Small => Some(64),
            Self::Medium => Some(192),
            Self::Large => Some(320),
            Self::Infinite => None,
        }
    }

    /// Width in blocks for the Overworld, or `None` when unbounded.
    pub const fn block_width(self) -> Option<u32> {
        match self.chunk_width() {
            Some(chunks) => Some(chunks * CHUNK_SIZE as u32),
            None => None,
        }
    }

    /// Nether coordinate scale (1 Nether block = `scale` Overworld blocks).
    ///
    /// `infinite` uses the Java Edition scale of 8 (see module docs).
    pub const fn nether_scale(self) -> u32 {
        match self {
            Self::Classic | Self::Small => 3,
            Self::Medium => 6,
            Self::Large | Self::Infinite => 8,
        }
    }

    /// Nether width in chunks, or `None` when unbounded.
    ///
    /// Integer division matches the reference (`HELL_LEVEL_MAX_WIDTH =
    /// LEVEL_MAX_WIDTH / HELL_LEVEL_MAX_SCALE`), which makes `small` 21.
    pub const fn nether_chunk_width(self) -> Option<u32> {
        match self.chunk_width() {
            Some(width) => Some(width / self.nether_scale()),
            None => None,
        }
    }

    /// The End is fixed at 18×18 chunks for every preset.
    pub const fn end_chunk_width(self) -> u32 {
        18
    }

    /// Whether the world edge is a moat (classic/small/medium).
    ///
    /// The reference only declares moats for these three; large encounters the
    /// normal ocean edge. Exact visuals are pinned at M6/M13.
    pub const fn has_edge_moat(self) -> bool {
        matches!(self, Self::Classic | Self::Small | Self::Medium)
    }

    /// Whether this preset has finite bounds.
    pub const fn is_finite(self) -> bool {
        self.chunk_width().is_some()
    }

    /// Whether `chunk` lies inside the finite Overworld bounds.
    ///
    /// Always `true` for `infinite`.
    pub fn contains_overworld_chunk(self, chunk: ChunkPos) -> bool {
        Self::contains_chunk_in(self.chunk_width(), chunk)
    }

    /// Whether `pos` lies inside the finite Overworld bounds.
    ///
    /// Always `true` for `infinite`.
    pub fn contains_overworld_block(self, pos: BlockPos) -> bool {
        Self::contains_block_in(self.block_width(), pos)
    }

    fn contains_chunk_in(width: Option<u32>, chunk: ChunkPos) -> bool {
        match width {
            None => true,
            Some(width) => {
                let width = i32::try_from(width).unwrap_or(i32::MAX);
                (0..width).contains(&chunk.x) && (0..width).contains(&chunk.z)
            }
        }
    }

    fn contains_block_in(width: Option<u32>, pos: BlockPos) -> bool {
        match width {
            None => true,
            Some(width) => {
                let width = i32::try_from(width).unwrap_or(i32::MAX);
                (0..width).contains(&pos.x()) && (0..width).contains(&pos.z())
            }
        }
    }
}

impl fmt::Display for WorldSizePreset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for WorldSizePreset {
    type Err = UnknownPreset;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| UnknownPreset(s.to_string()))
    }
}

/// A string that does not name a known world-size preset.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown world size preset: {0:?}")]
pub struct UnknownPreset(pub String);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verified_dimensions_match_the_reference() {
        let expected = [
            (WorldSizePreset::Classic, 54, 864, 3, 18, true),
            (WorldSizePreset::Small, 64, 1024, 3, 21, true),
            (WorldSizePreset::Medium, 192, 3072, 6, 32, true),
            (WorldSizePreset::Large, 320, 5120, 8, 40, false),
        ];
        for (preset, chunks, blocks, scale, nether_chunks, moat) in expected {
            assert_eq!(preset.chunk_width(), Some(chunks), "{preset}");
            assert_eq!(preset.block_width(), Some(blocks), "{preset}");
            assert_eq!(preset.nether_scale(), scale, "{preset}");
            assert_eq!(preset.nether_chunk_width(), Some(nether_chunks), "{preset}");
            assert_eq!(preset.has_edge_moat(), moat, "{preset}");
            assert_eq!(preset.end_chunk_width(), 18, "{preset}");
            assert!(preset.is_finite(), "{preset}");
        }
    }

    #[test]
    fn infinite_is_unbounded_with_java_nether_scale() {
        let preset = WorldSizePreset::Infinite;
        assert_eq!(preset.chunk_width(), None);
        assert_eq!(preset.block_width(), None);
        assert_eq!(preset.nether_chunk_width(), None);
        assert_eq!(preset.nether_scale(), 8);
        assert!(!preset.is_finite());
        assert!(!preset.has_edge_moat());
        assert_eq!(preset.end_chunk_width(), 18);
    }

    #[test]
    fn finite_bounds_are_zero_based_and_exclusive() {
        let preset = WorldSizePreset::Classic;
        assert!(preset.contains_overworld_chunk(ChunkPos::new(0, 0)));
        assert!(preset.contains_overworld_chunk(ChunkPos::new(53, 53)));
        assert!(!preset.contains_overworld_chunk(ChunkPos::new(54, 0)));
        assert!(!preset.contains_overworld_chunk(ChunkPos::new(0, 54)));
        assert!(!preset.contains_overworld_chunk(ChunkPos::new(-1, 0)));

        assert!(preset.contains_overworld_block(BlockPos::new(0, 0, 0)));
        assert!(preset.contains_overworld_block(BlockPos::new(863, 255, 863)));
        assert!(!preset.contains_overworld_block(BlockPos::new(864, 64, 0)));
        assert!(!preset.contains_overworld_block(BlockPos::new(0, 64, -1)));
    }

    #[test]
    fn infinite_accepts_any_coordinate() {
        let preset = WorldSizePreset::Infinite;
        for chunk in [
            ChunkPos::new(0, 0),
            ChunkPos::new(-1_000_000, 1_000_000),
            ChunkPos::new(i32::MAX, i32::MIN),
        ] {
            assert!(preset.contains_overworld_chunk(chunk));
        }
        assert!(preset.contains_overworld_block(BlockPos::new(-5, -64, 5)));
    }

    #[test]
    fn keys_round_trip() {
        for preset in WorldSizePreset::ALL {
            assert_eq!(WorldSizePreset::parse(preset.as_str()), Some(*preset));
            assert_eq!(preset.to_string(), preset.as_str());
        }
        assert_eq!(
            WorldSizePreset::parse("classic"),
            Some(WorldSizePreset::Classic)
        );
        assert_eq!(WorldSizePreset::parse("Classic"), None);
        assert!("huge".parse::<WorldSizePreset>().is_err());
        assert_eq!(
            "large".parse::<WorldSizePreset>(),
            Ok(WorldSizePreset::Large)
        );
    }
}
