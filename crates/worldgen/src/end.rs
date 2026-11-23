//! The End generator matching LCE reference (TheEndLevelRandomLevelSource, TheEndDimension)

use crate::biome::BiomeId;
use crate::mth::Mth;
use crate::noise::{ImprovedNoise, PerlinNoise};
use crate::terrain::TerrainGenerator;
use mclre_core::GameRandom;
use mclre_world::ChunkPos;

pub struct EndChunkData {
    pub blocks: Vec<u8>,
    pub biomes: Vec<crate::biome::BiomeId>,
    pub heightmap: Vec<i32>,
}
impl EndChunkData {
    pub fn new() -> Self {
        Self {
            blocks: vec![0; 16 * 128 * 16],
            biomes: vec![crate::biome::BiomeId::Sky; 16 * 16],
            heightmap: vec![0; 16 * 16],
        }
    }
}

pub struct EndGenerator {
    seed: i64,
    lperlin_noise1: PerlinNoise,
    lperlin_noise2: PerlinNoise,
    perlin_noise1: PerlinNoise,
    perlin_noise3: PerlinNoise,
    scale_noise: PerlinNoise,
    depth_noise: PerlinNoise,
}

impl EndGenerator {
    pub fn new(seed: i64) -> Self {
        let mut random = mclre_core::GameRandom::with_seed(seed);
        Self {
            seed,
            lperlin_noise1: PerlinNoise::new(&mut random, 16),
            lperlin_noise2: PerlinNoise::new(&mut random, 16),
            perlin_noise1: PerlinNoise::new(&mut random, 8),
            perlin_noise3: PerlinNoise::new(&mut random, 4),
            scale_noise: PerlinNoise::new(&mut random, 10),
            depth_noise: PerlinNoise::new(&mut random, 16),
        }
    }

    pub fn generate_chunk(&mut self, chunk_pos: mclre_world::ChunkPos) -> EndChunkData {
        let mut chunk = EndChunkData::new();
        let max_coord = 9; // 18 chunks / 2 = 9
        if chunk_pos.x < -max_coord
            || chunk_pos.x >= max_coord
            || chunk_pos.z < -max_coord
            || chunk_pos.z >= max_coord
        {
            return chunk; // Outside end island
        }
        self.generate_island(chunk_pos, &mut chunk.blocks);
        self.generate_pillars(chunk_pos, &mut chunk.blocks);
        self.generate_exit_portal(chunk_pos, &mut chunk.blocks);
        self.calculate_heightmap(&mut chunk);
        chunk
    }

    pub fn post_process_chunk(
        &mut self,
        chunk_pos: mclre_world::ChunkPos,
        _chunk: &mut EndChunkData,
    ) {
        let mut random = mclre_core::GameRandom::with_seed(self.seed);
        random.set_seed(
            ((chunk_pos.x as i64 * 341873128712 + chunk_pos.z as i64 * 132897987541) ^ self.seed)
                as i64,
        );
    }

    fn generate_island(&self, chunk_pos: mclre_world::ChunkPos, blocks: &mut [u8]) {
        let mut random = mclre_core::GameRandom::with_seed(self.seed);
        random.set_seed(
            ((chunk_pos.x as i64 * 341873128712 + chunk_pos.z as i64 * 132897987541) as i64),
        );
        for x in 0..16 {
            for z in 0..16 {
                let wx = chunk_pos.x * 16 + x;
                let wz = chunk_pos.z * 16 + z;
                let dist_sq = wx * wx + wz * wz;
                let max_radius = 55;
                if dist_sq <= max_radius * max_radius {
                    let base_y = 45;
                    let height = if dist_sq < 20 * 20 {
                        20
                    } else {
                        10 + mclre_core::GameRandom::with_seed(self.seed).next_int_bound(10)
                    };
                    for y in base_y..base_y + height {
                        if y < 128 {
                            blocks[((z * 16 + x) * 128 + y) as usize] = 121;
                        }
                    }
                    let top_y = base_y + height - 1;
                    if top_y < 128 {
                        let offs = ((z * 16 + x) * 128 + top_y) as usize;
                        if mclre_core::GameRandom::with_seed(self.seed).next_int_bound(10) == 0 {
                            blocks[offs] = 206;
                        }
                    }
                }
            }
        }
    }

    fn generate_pillars(&self, chunk_pos: mclre_world::ChunkPos, blocks: &mut [u8]) {
        let pillar_positions = [
            (0, -40),
            (28, -28),
            (40, 0),
            (28, 28),
            (0, 40),
            (-28, 28),
            (-40, 0),
            (-28, -28),
            (20, -20),
            (-20, 20),
        ];
        for (px, pz) in pillar_positions {
            let cx = px - chunk_pos.x * 16;
            let cz = pz - chunk_pos.z * 16;
            if cx >= 0 && cx < 16 && cz >= 0 && cz < 16 {
                for y in 0..127 {
                    let offs = ((cz * 16 + cx) * 128 + y) as usize;
                    if y < 10 {
                        blocks[offs] = 7;
                    } else if y < 120 {
                        blocks[offs] = 49;
                    } else {
                        blocks[offs] = 0;
                    }
                }
            }
        }
    }

    fn generate_exit_portal(&self, chunk_pos: mclre_world::ChunkPos, blocks: &mut [u8]) {
        if chunk_pos.x == 0 && chunk_pos.z == 0 {
            for x in -2..=2 {
                for z in -2..=2 {
                    let cx = x + 8;
                    let cz = z + 8;
                    if cx >= 0 && cx < 16 && cz >= 0 && cz < 16 {
                        for y in 60..=65 {
                            let offs = ((cz * 16 + cx) * 128 + y) as usize;
                            if x == -2 || x == 2 || z == -2 || z == 2 {
                                blocks[offs] = 7;
                            } else if y == 65 {
                                blocks[offs] = 119;
                            }
                        }
                    }
                }
            }
        }
    }

    fn calculate_heightmap(&self, chunk: &mut EndChunkData) {
        for x in 0..16 {
            for z in 0..16 {
                let mut height = 0i32;
                for y in (0..128).rev() {
                    let offs = (z * 16 + x) * 128 + y as usize;
                    if chunk.blocks[offs] != 0 {
                        height = y;
                        break;
                    }
                }
                chunk.heightmap[(z * 16 + x) as usize] = height;
            }
        }
    }
}
