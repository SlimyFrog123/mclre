//! Structure generation
use crate::terrain::TerrainGenerator;
use mclre_core::GameRandom;
use mclre_world::{BlockPos, ChunkPos};

pub trait StructureFeature {
    fn apply(&mut self, _terrain: &mut TerrainGenerator, _chunk_x: i32, _chunk_z: i32, _blocks: &mut [u8]) {}
    fn post_process(&mut self, _random: &mut mclre_core::GameRandom, _chunk_x: i32, _chunk_z: i32) {}
}

pub struct CaveFeature;
impl CaveFeature { pub fn new(_seed: i64) -> Self { Self } }
impl StructureFeature for CaveFeature { fn apply(&mut self, _t: &mut TerrainGenerator, _x: i32, _z: i32, _b: &mut [u8]) {} fn post_process(&mut self, _r: &mut mclre_core::GameRandom, _x: i32, _z: i32) {} }

pub struct CanyonFeature;
impl CanyonFeature { pub fn new(_seed: i64) -> Self { Self } }
impl StructureFeature for CanyonFeature { fn apply(&mut self, _t: &mut TerrainGenerator, _x: i32, _z: i32, _b: &mut [u8]) {} fn post_process(&mut self, _r: &mut mclre_core::GameRandom, _x: i32, _z: i32) {} }

pub struct VillageFeature { world_size: i32 }
impl VillageFeature { pub fn new(ws: i32) -> Self { Self { world_size: ws } } }
impl StructureFeature for VillageFeature { fn apply(&mut self, _t: &mut TerrainGenerator, _x: i32, _z: i32, _b: &mut [u8]) {} fn post_process(&mut self, _r: &mut mclre_core::GameRandom, _x: i32, _z: i32) -> bool { false } }

pub struct StrongholdFeature;
impl StrongholdFeature { pub fn new() -> Self { Self } }
impl StructureFeature for StrongholdFeature { fn apply(&mut self, _t: &mut TerrainGenerator, _x: i32, _z: i32, _b: &mut [u8]) {} fn post_process(&mut self, _r: &mut mclre_core::GameRandom, _x: i32, _z: i32) {} }
impl StrongholdFeature { pub fn get_nearest_generated_feature(&self, _x: i32, _y: i32, _z: i32) -> Option<mclre_world::BlockPos> { None } }

pub struct MineShaftFeature;
impl MineShaftFeature { pub fn new() -> Self { Self } }
impl StructureFeature for MineShaftFeature { fn apply(&mut self, _t: &mut TerrainGenerator, _x: i32, _z: i32, _b: &mut [u8]) {} fn post_process(&mut self, _r: &mut mclre_core::GameRandom, _x: i32, _z: i32) {} }

pub struct RandomScatteredLargeFeature;
impl RandomScatteredLargeFeature { pub fn new() -> Self { Self } }
impl StructureFeature for RandomScatteredLargeFeature { fn apply(&mut self, _t: &mut TerrainGenerator, _x: i32, _z: i32, _b: &mut [u8]) {} fn post_process(&mut self, _r: &mut mclre_core::GameRandom, _x: i32, _z: i32) {} }
impl RandomScatteredLargeFeature { fn generate_witch_hut(&self, _b: &mut [u8], _x: i32, _z: i32, _r: &mut mclre_core::GameRandom) {} pub fn is_swamphut(&self, _x: i32, _y: i32, _z: i32) -> bool { false } pub fn get_swamphut_enemies(&self) -> Vec<u32> { vec![] } }

pub struct LakeFeature { block_id: u8 }
impl LakeFeature { pub fn new(id: u8) -> Self { Self { block_id: id } } pub fn place(&self, _b: &mut [u8], _x: i32, _y: i32, _z: i32, _r: &mut mclre_core::GameRandom) {} }

pub struct MonsterRoomFeature;
impl MonsterRoomFeature { pub fn new() -> Self { Self } pub fn place(&self, _b: &mut [u8], _x: i32, _y: i32, _z: i32, _r: &mut mclre_core::GameRandom) {} }