//! Overworld generator matching LCE reference (RandomLevelSource)

use crate::biome::{BiomeId, BiomeSource};
use crate::decoration::{DecorationContext, decorate_chunk};
use crate::layer::GeneratorType;
use crate::mth::Mth;
use crate::structure::{
    CanyonFeature, CaveFeature, LakeFeature, MineShaftFeature, MonsterRoomFeature,
    RandomScatteredLargeFeature, StrongholdFeature, StructureFeature, VillageFeature,
};
use crate::terrain::TerrainGenerator;
use mclre_core::GameRandom;
use mclre_profiles::Profile;
use mclre_world::{BlockPos, ChunkPos, WorldSizePreset};

pub struct OverworldGenerator {
    seed: i64,
    world_size: WorldSizePreset,
    generate_structures: bool,
    pub biome_source: BiomeSource,
    terrain: TerrainGenerator,
    cave_feature: CaveFeature,
    canyon_feature: CanyonFeature,
    village_feature: VillageFeature,
    stronghold_feature: StrongholdFeature,
    mine_shaft_feature: MineShaftFeature,
    scattered_feature: RandomScatteredLargeFeature,
}

impl OverworldGenerator {
    pub fn new(
        seed: i64,
        world_size: WorldSizePreset,
        generate_structures: bool,
        _profile: &Profile,
    ) -> Self {
        Self {
            seed,
            world_size,
            generate_structures,
            biome_source: BiomeSource::new(seed, GeneratorType::Normal),
            terrain: TerrainGenerator::new(seed, world_size),
            cave_feature: CaveFeature::new(seed),
            canyon_feature: CanyonFeature::new(seed),
            village_feature: VillageFeature::new(64),
            stronghold_feature: StrongholdFeature::new(),
            mine_shaft_feature: MineShaftFeature::new(),
            scattered_feature: RandomScatteredLargeFeature::new(),
        }
    }

    pub fn generate_chunk(&mut self, chunk_pos: ChunkPos) -> OverworldChunkData {
        let mut chunk = OverworldChunkData::new();
        let heights = self
            .terrain
            .prepare_heights(chunk_pos.x, chunk_pos.z, &self.biome_source);
        self.build_base_terrain(chunk_pos, &mut chunk.blocks, &heights);
        let biomes = self
            .biome_source
            .get_biome_block(chunk_pos.x * 16, chunk_pos.z * 16, 16, 16);
        chunk.biomes = biomes.clone();
        self.terrain
            .build_surfaces(chunk_pos.x, chunk_pos.z, &mut chunk.blocks, &biomes);
        self.cave_feature.place(
            &mut self.terrain,
            chunk_pos.x,
            chunk_pos.z,
            &mut chunk.blocks,
        );
        self.canyon_feature.place(
            &mut self.terrain,
            chunk_pos.x,
            chunk_pos.z,
            &mut chunk.blocks,
        );
        if self.generate_structures {
            self.mine_shaft_feature.place(
                &mut self.terrain,
                chunk_pos.x,
                chunk_pos.z,
                &mut chunk.blocks,
            );
            self.village_feature.place(
                &mut self.terrain,
                chunk_pos.x,
                chunk_pos.z,
                &mut chunk.blocks,
            );
            self.stronghold_feature.place(
                &mut self.terrain,
                chunk_pos.x,
                chunk_pos.z,
                &mut chunk.blocks,
            );
            self.scattered_feature.place(
                &mut self.terrain,
                chunk_pos.x,
                chunk_pos.z,
                &mut chunk.blocks,
            );
        }
        self.calculate_heightmap(&mut chunk);
        chunk
    }

    pub fn post_process_chunk(&mut self, chunk_pos: ChunkPos, chunk: &mut OverworldChunkData) {
        let mut random = mclre_core::GameRandom::with_seed(self.seed);
        random.set_seed(
            ((chunk_pos.x as i64 * 341873128712 + chunk_pos.z as i64 * 132897987541) ^ self.seed)
                as i64,
        );
        self.calc_water_depths(chunk_pos, &mut chunk.blocks);
        let mut has_village = false;
        if self.generate_structures {
            self.mine_shaft_feature
                .post_process(&mut random, chunk_pos.x, chunk_pos.z);
            has_village = self
                .village_feature
                .post_process(&mut random, chunk_pos.x, chunk_pos.z);
            self.stronghold_feature
                .post_process(&mut random, chunk_pos.x, chunk_pos.z);
            // Check scattered feature (temples) with biome
            let biome = self
                .biome_source
                .get_biome(chunk_pos.x * 16 + 8, chunk_pos.z * 16 + 8);
            if self.scattered_feature.is_feature_chunk_with_biome(
                chunk_pos.x,
                chunk_pos.z,
                biome,
                self.seed,
            ) {
                // Temple generated here
            }
        }
        let biome = self
            .biome_source
            .get_biome(chunk_pos.x * 16 + 16, chunk_pos.z * 16 + 16);
        if biome != crate::biome::BiomeId::Desert && biome != crate::biome::BiomeId::DesertHills {
            if !has_village && random.next_int_bound(4) == 0 {
                let x = chunk_pos.x * 16 + random.next_int_bound(16) + 8;
                let y = random.next_int_bound(128);
                let z = chunk_pos.z * 16 + random.next_int_bound(16) + 8;
                let mut lake = LakeFeature::new(8);
                lake.place(&mut chunk.blocks, x, y, z, &mut random);
            }
        }
        if !has_village && random.next_int_bound(8) == 0 {
            let x = chunk_pos.x * 16 + random.next_int_bound(16) + 8;
            let inner = random.next_int_bound(120);
            let y = inner + 8;
            let z = chunk_pos.z * 16 + random.next_int_bound(16) + 8;
            if y < 63 || random.next_int_bound(10) == 0 {
                let mut lake = LakeFeature::new(10);
                lake.place(&mut chunk.blocks, x, y, z, &mut random);
            }
        }
        for _ in 0..8 {
            let x = chunk_pos.x * 16 + random.next_int_bound(16) + 8;
            let y = random.next_int_bound(128);
            let z = chunk_pos.z * 16 + random.next_int_bound(16) + 8;
            let mut mrf = MonsterRoomFeature::new();
            mrf.place(&mut chunk.blocks, x, y, z, &mut random);
        }
        for x in 0..16 {
            for z in 0..16 {
                let bx = chunk_pos.x * 16 + x;
                let bz = chunk_pos.z * 16 + z;
                let biome = self.biome_source.get_biome(bx, bz);
                let mut ctx = DecorationContext::new(biome, chunk_pos.x, chunk_pos.z, self.seed);
                crate::decoration::decorate_chunk(&mut ctx, &mut chunk.blocks);
            }
        }
        self.update_ice_and_snow(chunk_pos, &mut chunk.blocks);
    }

    fn build_base_terrain(&self, chunk_pos: ChunkPos, blocks: &mut [u8], heights: &[f64]) {
        for x in 0..16 {
            for z in 0..16 {
                let height = heights[(x + z * 17) as usize] as i32;
                for y in 0..128 {
                    let offs = ((z * 16 + x) * 128 + y) as usize;
                    if y <= height {
                        blocks[offs] = 1;
                    } else if y < 63 {
                        blocks[offs] = 8;
                    } else {
                        blocks[offs] = 0;
                    }
                }
                let mut rng = mclre_core::GameRandom::with_seed(self.seed);
                for y in 0..=1 + crate::mth::Mth::next_int_bounded(&mut rng, 0, 1) {
                    let offs = ((z * 16 + x) * 128 + y) as usize;
                    blocks[offs] = 7;
                }
            }
        }
    }

    fn calculate_heightmap(&self, chunk: &mut OverworldChunkData) {
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

    fn calc_water_depths(&self, _chunk_pos: ChunkPos, _blocks: &mut [u8]) {}
    fn update_ice_and_snow(&self, chunk_pos: ChunkPos, blocks: &mut [u8]) {
        let xo = chunk_pos.x * 16 + 8;
        let zo = chunk_pos.z * 16 + 8;
        for x in 0..16 {
            for z in 0..16 {
                let y = self.get_top_rain_block(xo + x, zo + z);
                if self.should_freeze_ignore_neighbors(x + xo, y - 1, z + zo) {
                    set_block(blocks, x + xo, y - 1, z + zo, 79);
                }
                if self.should_snow(x + xo, y, z + zo) {
                    set_block(blocks, x + xo, y, z + zo, 78);
                }
            }
        }
    }
    fn get_top_rain_block(&self, _x: i32, _z: i32) -> i32 {
        63
    }
    fn should_freeze_ignore_neighbors(&self, _x: i32, _y: i32, _z: i32) -> bool {
        false
    }
    fn should_snow(&self, _x: i32, _y: i32, _z: i32) -> bool {
        false
    }
    pub fn find_spawn(&self) -> mclre_world::BlockPos {
        mclre_world::BlockPos::new(0, 64, 0)
    }
}

pub struct OverworldChunkData {
    pub blocks: Vec<u8>,
    pub biomes: Vec<crate::biome::BiomeId>,
    pub heightmap: Vec<i32>,
    pub entities: Vec<EntityData>,
    pub tile_entities: Vec<TileEntityData>,
}
impl OverworldChunkData {
    pub fn new() -> Self {
        Self {
            blocks: vec![0; 16 * 128 * 16],
            biomes: vec![crate::biome::BiomeId::Ocean; 16 * 16],
            heightmap: vec![0; 16 * 16],
            entities: Vec::new(),
            tile_entities: Vec::new(),
        }
    }
}
pub struct EntityData {
    pub id: u32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
pub struct TileEntityData {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub data: Vec<u8>,
}

fn set_block(b: &mut [u8], x: i32, y: i32, z: i32, id: u8) {
    if x < 0 || x >= 256 || y < 0 || y >= 128 || z < 0 || z >= 256 {
        return;
    }
}
fn get_block(_b: &[u8], _x: i32, _y: i32, _z: i32) -> u8 {
    0
}
