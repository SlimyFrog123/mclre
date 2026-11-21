//! Baseline profile world generator (lce:141217 / TU19)

use crate::overworld::OverworldGenerator;
use crate::nether::NetherGenerator;
use crate::end::EndGenerator;
use crate::WorldGenerator;
use crate::{ChunkData, BiomeId};
use mclre_core::BlockStateId;
use mclre_profiles::Profile;
use mclre_world::{ChunkPos, WorldSizePreset};

pub struct Tu19Generator {
    seed: i64, preset: WorldSizePreset,
    overworld: OverworldGenerator,
    nether: NetherGenerator,
    end: EndGenerator,
}

impl Tu19Generator {
    pub fn new(seed: i64, preset: WorldSizePreset) -> Self {
        let profile = Profile::new(mclre_profiles::ProfileConfig::new()).unwrap();
        Self { seed, preset,
            overworld: OverworldGenerator::new(seed, preset, true, &mclre_profiles::Profile::new(mclre_profiles::ProfileConfig::new()).unwrap()),
            nether: NetherGenerator::new(seed, preset),
            end: EndGenerator::new(seed),
        }
    }

    pub fn new_with_profile(seed: i64, preset: WorldSizePreset, generate_structures: bool) -> Self {
        let profile = mclre_profiles::Profile::new(mclre_profiles::ProfileConfig::new()).unwrap();
        Self { seed, preset,
            overworld: OverworldGenerator::new(seed, preset, generate_structures, &mclre_profiles::Profile::new(mclre_profiles::ProfileConfig::new()).unwrap()),
            nether: NetherGenerator::new(seed, preset),
            end: EndGenerator::new(seed),
        }
    }
}

impl super::WorldGenerator for Tu19Generator {
    fn generate_chunk(&mut self, chunk_pos: mclre_world::ChunkPos) -> ChunkData {
        let overworld_chunk = self.overworld.generate_chunk(chunk_pos);
        let mut chunk = ChunkData::new();
        for (i, &block_id) in overworld_chunk.blocks.iter().enumerate() { if block_id != 0 { chunk.blocks[i] = BlockStateId(block_id as u32); } }
        chunk.biomes = overworld_chunk.biomes; chunk.heightmap = overworld_chunk.heightmap; chunk
    }

    fn post_process_chunk(&mut self, chunk_pos: mclre_world::ChunkPos, chunk: &mut ChunkData) {
        let mut overworld_chunk = super::overworld::OverworldChunkData {
            blocks: chunk.blocks.iter().map(|b| b.0 as u8).collect(),
            biomes: chunk.biomes.clone(), heightmap: chunk.heightmap.clone(),
            entities: Vec::new(), tile_entities: Vec::new(),
        };
        self.overworld.post_process_chunk(chunk_pos, &mut overworld_chunk);
        chunk.blocks = overworld_chunk.blocks.iter().map(|&b| mclre_core::BlockStateId(b as u32)).collect();
        chunk.biomes = overworld_chunk.biomes; chunk.heightmap = overworld_chunk.heightmap;
    }

    fn get_biome(&self, pos: mclre_world::BlockPos) -> BiomeId {
        self.overworld.biome_source.get_biome(pos.x, pos.z)
    }
}

impl Tu19Generator {
    pub fn generate_nether_chunk(&mut self, chunk_pos: mclre_world::ChunkPos) -> ChunkData {
        let nether_chunk = self.nether.generate_chunk(chunk_pos);
        let mut chunk = ChunkData::new();
        for (i, &block_id) in nether_chunk.blocks.iter().enumerate() { if block_id != 0 { chunk.blocks[i] = mclre_core::BlockStateId(block_id as u32); } }
        chunk.biomes = nether_chunk.biomes; chunk.heightmap = nether_chunk.heightmap; chunk
    }

    pub fn generate_end_chunk(&mut self, chunk_pos: mclre_world::ChunkPos) -> ChunkData {
        let end_chunk = self.end.generate_chunk(chunk_pos);
        let mut chunk = ChunkData::new();
        for (i, &block_id) in end_chunk.blocks.iter().enumerate() { if block_id != 0 { chunk.blocks[i] = mclre_core::BlockStateId(block_id as u32); } }
        chunk.biomes = end_chunk.biomes; chunk.heightmap = end_chunk.heightmap; chunk
    }

    pub fn post_process_nether_chunk(&mut self, chunk_pos: mclre_world::ChunkPos, chunk: &mut ChunkData) {
        let mut nether_chunk = super::nether::NetherChunkData { blocks: chunk.blocks.iter().map(|b| b.0 as u8).collect(), biomes: chunk.biomes.clone(), heightmap: chunk.heightmap.clone() };
        self.nether.post_process_chunk(chunk_pos, &mut nether_chunk);
        chunk.blocks = nether_chunk.blocks.iter().map(|&b| mclre_core::BlockStateId(b as u32)).collect();
        chunk.biomes = nether_chunk.biomes; chunk.heightmap = nether_chunk.heightmap;
    }

    pub fn post_process_end_chunk(&mut self, chunk_pos: mclre_world::ChunkPos, chunk: &mut ChunkData) {
        let mut end_chunk = super::end::EndChunkData { blocks: chunk.blocks.iter().map(|b| b.0 as u8).collect(), biomes: chunk.biomes.clone(), heightmap: chunk.heightmap.clone() };
        self.end.post_process_chunk(chunk_pos, &mut end_chunk);
        chunk.blocks = end_chunk.blocks.iter().map(|&b| mclre_core::BlockStateId(b as u32)).collect();
        chunk.biomes = end_chunk.biomes; chunk.heightmap = end_chunk.heightmap;
    }
}