//! Biome decoration (trees, ores, flowers, tall grass, etc.)

use crate::biome::BiomeId;
use mclre_core::GameRandom;

pub struct DecorationContext { pub biome: BiomeId, pub chunk_x: i32, pub chunk_z: i32, pub random: GameRandom }
impl DecorationContext { pub fn new(biome: BiomeId, cx: i32, cz: i32, seed: i64) -> Self { let mut r = mclre_core::GameRandom::with_seed(seed); r.set_seed(((cx as i64 * 341873128712 + cz as i64 * 132897987541) as i64)); Self { biome, chunk_x: cx, chunk_z: cz, random: r } } }

pub fn decorate_chunk(_ctx: &mut DecorationContext, _blocks: &mut [u8]) {}

fn decorate_plains(_ctx: &mut DecorationContext, _blocks: &mut [u8]) {}
fn decorate_forest(_ctx: &mut DecorationContext, _blocks: &mut [u8]) {}
fn decorate_taiga(_ctx: &mut DecorationContext, _blocks: &mut [u8]) {}
fn decorate_jungle(_ctx: &mut DecorationContext, _blocks: &mut [u8]) {}
fn decorate_desert(_ctx: &mut DecorationContext, _blocks: &mut [u8]) {}
fn decorate_swamp(_ctx: &mut DecorationContext, _blocks: &mut [u8]) {}
fn decorate_extreme_hills(_ctx: &mut DecorationContext, _blocks: &mut [u8]) {}
fn decorate_ice_plains(_ctx: &mut DecorationContext, _blocks: &mut [u8]) {}
fn decorate_mushroom_island(_ctx: &mut DecorationContext, _blocks: &mut [u8]) {}
fn decorate_ocean(_ctx: &mut DecorationContext, _blocks: &mut [u8]) {}

fn decorate_ores(_ctx: &mut DecorationContext, _blocks: &mut [u8]) {}
fn decorate_flowers(_ctx: &mut DecorationContext, _blocks: &mut [u8]) {}
fn decorate_tall_grass(_ctx: &mut DecorationContext, _blocks: &mut [u8]) {}
fn decorate_lakes(_ctx: &mut DecorationContext, _blocks: &mut [u8]) {}

fn find_top_block(_b: &[u8], _x: i32, _z: i32) -> i32 { 0 }
fn get_block(_b: &[u8], _x: i32, _y: i32, _z: i32) -> u8 { 0 }
fn set_block(_b: &mut [u8], _x: i32, _y: i32, _z: i32, _id: u8) {}
fn can_place_flower(_b: &[u8], _x: i32, _y: i32, _z: i32) -> bool { false }
fn can_place_tall_grass(_b: &[u8], _x: i32, _y: i32, _z: i32) -> bool { false }
fn generate_water_lake(_b: &mut [u8], _x: i32, _y: i32, _z: i32, _r: &mut mclre_core::GameRandom) {}
fn generate_lava_lake(_b: &mut [u8], _x: i32, _y: i32, _z: i32, _r: &mut mclre_core::GameRandom) {}
fn generate_cactus(_b: &mut [u8], _x: i32, _y: i32, _z: i32, _r: &mut mclre_core::GameRandom) {}
fn generate_oak_tree(_b: &mut [u8], _x: i32, _y: i32, _z: i32, _r: &mut mclre_core::GameRandom) {}
fn generate_birch_tree(_b: &mut [u8], _x: i32, _y: i32, _z: i32, _r: &mut mclre_core::GameRandom) {}
fn generate_spruce_tree(_b: &mut [u8], _x: i32, _y: i32, _z: i32, _r: &mut mclre_core::GameRandom) {}
fn generate_pine_tree(_b: &mut [u8], _x: i32, _y: i32, _z: i32, _r: &mut mclre_core::GameRandom) {}
fn generate_jungle_tree(_b: &mut [u8], _x: i32, _y: i32, _z: i32, _r: &mut mclre_core::GameRandom) {}
fn generate_mega_jungle_tree(_b: &mut [u8], _x: i32, _y: i32, _z: i32, _r: &mut mclre_core::GameRandom) {}
fn generate_swamp_oak(_b: &mut [u8], _x: i32, _y: i32, _z: i32, _r: &mut mclre_core::GameRandom) {}
fn generate_huge_red_mushroom(_b: &mut [u8], _x: i32, _y: i32, _z: i32, _r: &mut mclre_core::GameRandom) {}
fn generate_huge_brown_mushroom(_b: &mut [u8], _x: i32, _y: i32, _z: i32, _r: &mut mclre_core::GameRandom) {}
fn generate_ore_vein(_b: &mut [u8], _id: u8, _min_y: i32, _max_y: i32, _sz: i32, _r: &mut mclre_core::GameRandom) {}