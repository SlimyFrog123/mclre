//! Versioned world generators (per Game Profile): biomes, terrain, caves, structures, decoration.

// Core math and noise
pub mod noise;
pub mod mth;

// Biome generation
pub mod biome;
pub mod layer;

// World generation
pub mod terrain;
pub mod structure;
pub mod decoration;

// Dimension generators
pub mod overworld;
pub mod nether;
pub mod end;

// Profile-specific generators
pub mod v_tu19;

use mclre_core::GameRandom;
use mclre_world::{BlockPos, ChunkPos, WorldSizePreset};
use mclre_profiles::Profile;

/// World generator trait for versioned generators
pub trait WorldGenerator {
    /// Generate a chunk column at the given position
    fn generate_chunk(&mut self, chunk_pos: ChunkPos) -> ChunkData;
    
    /// Post-process a chunk (structures, decoration, etc.)
    fn post_process_chunk(&mut self, chunk_pos: ChunkPos, chunk_data: &mut ChunkData);
    
    /// Get the biome at a position
    fn get_biome(&self, pos: BlockPos) -> BiomeId;
}

/// Biome ID type - re-export from biome module
pub use biome::BiomeId;

/// Chunk data for generation
#[derive(Debug, Clone)]
pub struct ChunkData {
    /// Block states (16x256x16 = 65536 blocks)
    pub blocks: Vec<mclre_core::BlockStateId>,
    /// Biome IDs (16x16 = 256)
    pub biomes: Vec<BiomeId>,
    /// Heightmap
    pub heightmap: Vec<i32>,
}

impl ChunkData {
    pub fn new() -> Self {
        Self {
            blocks: vec![mclre_core::BlockStateId(0); 16 * 256 * 16],
            biomes: vec![BiomeId(0); 16 * 16],
            heightmap: vec![0; 16 * 16],
        }
    }
}

/// Create a world generator for the given profile
pub fn create_generator(profile: &Profile, seed: i64, preset: WorldSizePreset) -> Box<dyn WorldGenerator> {
    match profile.id.as_str() {
        "lce:141217" => Box::new(v_tu19::Tu19Generator::new(seed, preset)),
        _ => Box::new(v_tu19::Tu19Generator::new(seed, preset)), // fallback
    }
}