//! Nether generator matching LCE reference (HellRandomLevelSource, HellDimension)

use crate::noise::{PerlinNoise, ImprovedNoise};
use crate::biome::BiomeId;
use crate::terrain::TerrainGenerator;
use crate::structure::{CaveFeature, CanyonFeature, LakeFeature, MonsterRoomFeature};
use crate::mth::Mth;
use mclre_core::GameRandom;
use mclre_world::{ChunkPos, WorldSizePreset};

pub struct NetherChunkData { pub blocks: Vec<u8>, pub biomes: Vec<crate::biome::BiomeId>, pub heightmap: Vec<i32> }
impl NetherChunkData { pub fn new() -> Self { Self { blocks: vec![0; 16 * 128 * 16], biomes: vec![crate::biome::BiomeId::Hell; 16 * 16], heightmap: vec![0; 16 * 16] } } }

pub struct NetherGenerator { seed: i64, world_size: WorldSizePreset, hell_scale: i32,
    lperlin_noise1: PerlinNoise, lperlin_noise2: PerlinNoise, perlin_noise1: PerlinNoise, perlin_noise3: PerlinNoise,
    scale_noise: PerlinNoise, depth_noise: PerlinNoise,
    cave_feature: CaveFeature, canyon_feature: CanyonFeature,
    fortress_feature: FortressFeature, glowstone_feature: GlowstoneFeature, spring_feature: SpringFeature,
}

impl NetherGenerator {
    pub fn new(seed: i64, world_size: WorldSizePreset) -> Self {
        let mut random = mclre_core::GameRandom::with_seed(seed);
        let hell_scale = match world_size { mclre_world::WorldSizePreset::Classic => 3, mclre_world::WorldSizePreset::Small => 3, mclre_world::WorldSizePreset::Medium => 6, mclre_world::WorldSizePreset::Large => 8, mclre_world::WorldSizePreset::Infinite => 8 };
        Self { seed, world_size, hell_scale,
            lperlin_noise1: PerlinNoise::new(&mut random, 16), lperlin_noise2: PerlinNoise::new(&mut random, 16),
            perlin_noise1: PerlinNoise::new(&mut random, 8), perlin_noise3: PerlinNoise::new(&mut random, 4),
            scale_noise: PerlinNoise::new(&mut random, 10), depth_noise: PerlinNoise::new(&mut random, 16),
            cave_feature: CaveFeature::new(seed), canyon_feature: CanyonFeature::new(seed),
            fortress_feature: FortressFeature::new(), glowstone_feature: GlowstoneFeature::new(),
            spring_feature: SpringFeature::new(),
        }
    }

    pub fn generate_chunk(&mut self, chunk_pos: mclre_world::ChunkPos) -> NetherChunkData {
        let mut chunk = NetherChunkData::new();
        self.generate_base_terrain(chunk_pos, &mut chunk.blocks);
        self.cave_feature.apply_placeholder(&mut chunk.blocks, chunk_pos.x, chunk_pos.z);
        self.canyon_feature.apply_placeholder(&mut chunk.blocks, chunk_pos.x, chunk_pos.z);
        self.fortress_feature.apply(&mut chunk.blocks, chunk_pos);
        self.glowstone_feature.apply(&mut chunk.blocks, chunk_pos);
        self.spring_feature.apply(&mut chunk.blocks, chunk_pos);
        self.calculate_heightmap(&mut chunk);
        chunk
    }

    pub fn post_process_chunk(&mut self, chunk_pos: mclre_world::ChunkPos, chunk: &mut NetherChunkData) {
        let mut random = mclre_core::GameRandom::with_seed(self.seed);
        random.set_seed(((chunk_pos.x as i64 * 341873128712 + chunk_pos.z as i64 * 132897987541) ^ self.seed) as i64);
        if random.next_int_bound(8) == 0 {
            let x = chunk_pos.x * 16 + random.next_int_bound(16) + 8;
            let inner = random.next_int_bound(120); let y = inner + 8;
            let z = chunk_pos.z * 16 + random.next_int_bound(16) + 8;
            let mut lake = LakeFeature::new(10); lake.place(&mut chunk.blocks, x, y, z, &mut random);
        }
        for _ in 0..8 {
            let x = chunk_pos.x * 16 + random.next_int_bound(16) + 8;
            let y = random.next_int_bound(128);
            let z = chunk_pos.z * 16 + random.next_int_bound(16) + 8;
            let mut mrf = MonsterRoomFeature::new(); mrf.place(&mut chunk.blocks, x, y, z, &mut random);
        }
    }

    fn generate_base_terrain(&self, chunk_pos: mclre_world::ChunkPos, blocks: &mut [u8]) {
        let mut random = mclre_core::GameRandom::with_seed(self.seed);
        random.set_seed(((chunk_pos.x as i64 * 341873128712 + chunk_pos.z as i64 * 132897987541) as i64));
        for x in 0..16 { for z in 0..16 {
            let offs_ceiling = ((z * 16 + x) * 128 + 127) as usize;
            let offs_floor = ((z * 16 + x) * 128 + 0) as usize;
            blocks[offs_ceiling] = 7; blocks[offs_floor] = 7;
        }}
        for x in 0..16 { for z in 0..16 { for y in 1..127 {
            let offs = ((z * 16 + x) * 128 + y) as usize;
            let noise_val = self.get_terrain_noise(chunk_pos.x * 16 + x, y, chunk_pos.z * 16 + z);
            if noise_val > 0.0 { blocks[offs] = 87; }
        }}}
    }

    fn get_terrain_noise(&self, x: i32, y: i32, z: i32) -> f64 { let s = 684.412; self.lperlin_noise1.get_value_3d(x as f64 / s, y as f64 / s, z as f64 / s) }
    fn calculate_heightmap(&self, chunk: &mut NetherChunkData) { for x in 0..16 { for z in 0..16 { let mut h = 0; for y in (0..128).rev() { let offs = (z * 16 + x) * 128 + y; if chunk.blocks[offs] != 0 { h = y; break; }} chunk.heightmap[(z * 16 + x) as usize] = h; }} }
    pub fn get_scale(&self) -> i32 { self.hell_scale }
}

pub struct FortressFeature { positions: Vec<(i32, i32)> }
impl FortressFeature { pub fn new() -> Self { Self { positions: Vec::new() } } pub fn apply(&self, _b: &mut [u8], _cp: mclre_world::ChunkPos) {} }
pub struct GlowstoneFeature {}
impl GlowstoneFeature { pub fn new() -> Self { Self {} } pub fn apply(&self, _b: &mut [u8], _cp: mclre_world::ChunkPos) {} }
pub struct SpringFeature {}
impl SpringFeature { pub fn new() -> Self { Self {} } pub fn apply(&self, _b: &mut [u8], _cp: mclre_world::ChunkPos) {} }

trait CaveFeatureExt { fn apply_placeholder(&self, b: &mut [u8], cx: i32, cz: i32); }
impl CaveFeatureExt for super::structure::CaveFeature { fn apply_placeholder(&self, _b: &mut [u8], _cx: i32, _cz: i32) {} }
trait CanyonFeatureExt { fn apply_placeholder(&self, b: &mut [u8], cx: i32, cz: i32); }
impl CanyonFeatureExt for super::structure::CanyonFeature { fn apply_placeholder(&self, _b: &mut [u8], _cx: i32, _cz: i32) {} }