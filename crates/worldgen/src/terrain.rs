//! Terrain generation matching LCE reference (RandomLevelSource::prepareHeights, buildSurfaces)

use crate::noise::{PerlinNoise, ImprovedNoise};
use crate::biome::{BiomeSource, BiomeId};
use crate::mth::Mth;
use mclre_core::GameRandom;
use mclre_world::{BlockPos, ChunkPos, WorldSizePreset};

const CHUNK_WIDTH: i32 = 16;
const CHUNK_HEIGHT: i32 = 128;

pub struct TerrainGenerator {
    seed: i64, world_size: WorldSizePreset,
    lperlin_noise1: PerlinNoise, lperlin_noise2: PerlinNoise, perlin_noise1: PerlinNoise, perlin_noise3: PerlinNoise,
    scale_noise: PerlinNoise, depth_noise: PerlinNoise, forest_noise: PerlinNoise,
    pows: [f32; 25], pows_initialized: bool,
}

impl TerrainGenerator {
    pub fn new(seed: i64, world_size: WorldSizePreset) -> Self {
        let mut random = GameRandom::with_seed(seed);
        Self { seed, world_size,
            lperlin_noise1: PerlinNoise::new(&mut random, 16), lperlin_noise2: PerlinNoise::new(&mut random, 16),
            perlin_noise1: PerlinNoise::new(&mut random, 8), perlin_noise3: PerlinNoise::new(&mut random, 4),
            scale_noise: PerlinNoise::new(&mut random, 10), depth_noise: PerlinNoise::new(&mut random, 16),
            forest_noise: PerlinNoise::new(&mut random, 8),
            pows: [0.0; 25], pows_initialized: false,
        }
    }

    pub fn prepare_heights(&mut self, chunk_x: i32, chunk_z: i32, biome_source: &BiomeSource) -> Vec<f64> {
        let x_chunks = 1; let y_chunks = 1; let water_height = 63;
        let x_size = 2; let y_size = 2; let z_size = 2;
        let mut biomes = biome_source.get_raw_biome_block(chunk_x * 16 - 2, chunk_z * 16 - 2, 7, 7);
        if !self.pows_initialized { self.init_pows(); }
        let mut buffer = vec![0.0; 8];
        let s = 684.412; let hs = 684.412;
        let mut sr = vec![0.0; 4]; let mut dr = vec![0.0; 4];
        let mut pnr = vec![0.0; 8]; let mut ar = vec![0.0; 8]; let mut br = vec![0.0; 8];
        self.scale_noise.get_region_2d(&mut sr, chunk_x, chunk_z, 2, 2, 1.121, 1.121, 0.5);
        self.depth_noise.get_region_2d(&mut dr, chunk_x, chunk_z, 2, 2, 200.0, 200.0, 0.5);
        self.perlin_noise1.get_region(&mut pnr, chunk_x, 0, chunk_z, 2, 2, 2, s / 80.0, hs / 160.0, s / 80.0);
        self.lperlin_noise1.get_region(&mut ar, chunk_x, 0, chunk_z, 2, 2, 2, s, hs, s);
        self.lperlin_noise2.get_region(&mut br, chunk_x, 0, chunk_z, 2, 2, 2, s, hs, s);
        let mut buffer = vec![0.0; 8]; let mut p = 0;
        for xx in 0..2 { for zz in 0..2 {
            let mut sss = 0.0f32; let mut ddd = 0.0f32; let mut pow = 0.0f32;
            for xb in -2..=2 { for zb in -2..=2 {
                let b_idx = (xx + xb + 2) + (zz + zb + 2) * 7; // simplified
                let biome = BiomeSource::get_biome(BiomeId::Plains).unwrap();
                let ppp = self.pows[(xb + 2 + (zb + 2) * 5) as usize] / (biome.depth + 2.0);
                sss += biome.scale * ppp; ddd += biome.depth * ppp; pow += ppp;
            }}
            if pow > 0.0 { sss /= pow; ddd /= pow; }
            sss = sss * 0.9 + 0.1; ddd = (ddd * 4.0 - 1.0) / 8.0;
            let rdepth = (dr[p] / 8000.0) as f64;
            let rdepth = if rdepth < 0.0 { -rdepth * 0.3 } else { rdepth };
            let rdepth = rdepth * 3.0 - 2.0;
            let rdepth = if rdepth < 0.0 { let mut rd = rdepth / 2.0; if rd < -1.0 { rd = -1.0; } rd / 1.4 / 2.0 } else { let mut rd = if rdepth > 1.0 { 1.0 } else { rdepth }; rd / 8.0 };
            for yy in 0..2 {
                let mut depth = ddd as f64; let mut scale = sss as f64;
                depth += rdepth * 0.2; depth = depth * 2.0 / 16.0;
                let y_center = 1.0 + depth * 4.0;
                let mut val = 0.0; let y_offs = (yy as f64 - y_center) * 12.0 * 128.0 / 128.0 / scale;
                let y_offs = if y_offs < 0.0 { y_offs * 4.0 } else { y_offs };
                let bb = ar[p] / 512.0; let cc = br[p] / 512.0;
                let v = (pnr[p] / 10.0 + 1.0) / 2.0;
                if v < 0.0 { val = bb; } else if v > 1.0 { val = cc; } else { val = bb + (cc - bb) * v; }
                val -= y_offs;
                if yy > y_size - 4 { let slide = (yy - (y_size - 4)) as f64 / 3.0; val = val * (1.0 - slide) + -10.0 * slide; }
                buffer[p] = val; p += 1;
            }
        }}
        buffer
    }

    fn init_pows(&mut self) { for xb in -2..=2 { for zb in -2..=2 { self.pows[(xb + 2 + (zb + 2) * 5) as usize] = 10.0 / Mth::sqrt_f64((xb * xb + zb * zb) as f64 + 0.2); }} self.pows_initialized = true; }

    pub fn build_surfaces(&mut self, chunk_x: i32, chunk_z: i32, blocks: &mut [u8], biomes: &[crate::biome::BiomeId]) {
        let water_height = 63; let mut depth_buffer = vec![0.0; 256];
        let s = 1.0 / 32.0;
        self.perlin_noise3.get_region_2d(&mut depth_buffer, chunk_x * 16, chunk_z * 16, 16, 16, s * 2.0, s * 2.0, s * 2.0);
        let mut random = GameRandom::with_seed(self.seed);
        random.set_seed(((chunk_x as i64 * 341873128712 + chunk_z as i64 * 132897987541) as i64));
        for x in 0..16 { for z in 0..16 {
            let biome = biomes[(z + x * 16) as usize]; let biome_data = crate::biome::get_biome(biome).unwrap();
            let temp = biome_data.temperature; let run_depth = (depth_buffer[(x + z * 16) as usize] / 3.0 + 3.0 + random.next_double() * 0.25) as i32;
            let mut run = -1; let mut top = biome_data.top_material; let mut material = biome_data.filler_material;
            for y in (0..128).rev() { let offs = ((z * 16 + x) * 128 + y) as usize;
                if y <= 1 + random.next_int_bound(2) { blocks[offs] = 7; }
                else { let old = blocks[offs];
                    if old == 0 { run = -1; }
                    else if old == 1 { if run == -1 { if run_depth <= 0 { top = 0; material = 1; } else if y >= water_height - 4 && y <= water_height + 1 { top = biome_data.top_material; material = biome_data.filler_material; } if y < water_height && top == 0 { if temp < 0.15 { top = 79; } else { top = 8; } } run = run_depth; if y >= water_height - 1 { blocks[offs] = top; } else { blocks[offs] = material; } } else if run > 0 { run -= 1; blocks[offs] = material; if run == 0 && material == 12 { run = random.next_int_bound(4); material = 24; } } }
                }
            }}
        }
    }
}