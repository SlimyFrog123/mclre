//! Terrain generation matching LCE reference (RandomLevelSource::prepareHeights, buildSurfaces)

use crate::biome::{BiomeId, BiomeSource};
use crate::mth::Mth;
use crate::noise::{ImprovedNoise, PerlinNoise};
use mclre_core::GameRandom;
use mclre_world::{ChunkPos, WorldSizePreset};

const CHUNK_WIDTH: i32 = 16;
const CHUNK_HEIGHT: i32 = 128; // genDepth = 128 for baseline

/// Terrain generator for overworld matching LCE reference (RandomLevelSource)
pub struct TerrainGenerator {
    seed: i64,
    world_size: WorldSizePreset,

    // Noise generators (matching reference exactly)
    lperlin_noise1: PerlinNoise,
    lperlin_noise2: PerlinNoise,
    perlin_noise1: PerlinNoise,
    perlin_noise3: PerlinNoise,
    scale_noise: PerlinNoise,
    depth_noise: PerlinNoise,
    forest_noise: PerlinNoise,

    // Pow cache for biome blending (5x5 = 25 entries)
    pows: [f32; 25],
    pows_initialized: bool,
}

impl TerrainGenerator {
    pub fn new(seed: i64, world_size: WorldSizePreset) -> Self {
        let mut random = GameRandom::with_seed(seed);

        Self {
            seed,
            world_size,
            lperlin_noise1: PerlinNoise::new(&mut random, 16),
            lperlin_noise2: PerlinNoise::new(&mut random, 16),
            perlin_noise1: PerlinNoise::new(&mut random, 8),
            perlin_noise3: PerlinNoise::new(&mut random, 4),
            scale_noise: PerlinNoise::new(&mut random, 10),
            depth_noise: PerlinNoise::new(&mut random, 16),
            forest_noise: PerlinNoise::new(&mut random, 8),
            pows: [0.0; 25],
            pows_initialized: false,
        }
    }

    /// Initialize pows cache for biome blending (matches reference exactly)
    fn init_pows(&mut self) {
        for xb in -2..=2 {
            for zb in -2..=2 {
                let dist_sq = (xb * xb + zb * zb) as f32 + 0.2;
                let ppp = 10.0 / Mth::sqrt_f32(dist_sq);
                self.pows[(xb + 2 + (zb + 2) * 5) as usize] = ppp;
            }
        }
        self.pows_initialized = true;
    }

    /// Prepare heightmap for a chunk column (matches RandomLevelSource::prepareHeights)
    /// Returns a 17x17 heightmap (289 elements) for the chunk with borders
    pub fn prepare_heights(
        &mut self,
        chunk_x: i32,
        chunk_z: i32,
        biome_source: &BiomeSource,
    ) -> Vec<f64> {
        // Noise sampling at 8-block intervals (matching LCE reference)
        // For a 16-block chunk: 3 samples horizontally (0, 8, 16), 17 vertically (0..16), 3 horizontally
        let noise_x_size = 3; // chunk_x * 2 + 1 with borders
        let noise_y_size = 17; // 128/8 + 1 = 17
        let noise_z_size = 3;

        // Get biomes for the area (with border for blending) - base layer works at chunk resolution
        // Need 2-chunk border around the noise sample area
        let mut biomes = biome_source.get_raw_biome_chunk(
            chunk_x - 2,
            chunk_z - 2,
            noise_x_size + 4, // 3 + 4 = 7 chunks
            noise_z_size + 4,
        );

        // Initialize pows if needed
        if !self.pows_initialized {
            self.init_pows();
        }

        let s = 684.412;
        let hs = 684.412;

        // Get noise regions at 8-block intervals (matching reference exactly)
        let mut sr = vec![0.0; (noise_x_size * noise_z_size) as usize];
        let mut dr = vec![0.0; (noise_x_size * noise_z_size) as usize];
        let mut pnr = vec![0.0; (noise_x_size * noise_y_size * noise_z_size) as usize];
        let mut ar = vec![0.0; (noise_x_size * noise_y_size * noise_z_size) as usize];
        let mut br = vec![0.0; (noise_x_size * noise_y_size * noise_z_size) as usize];

        // Noise coordinates are in 8-block units: chunk_x * 2, chunk_z * 2
        let noise_chunk_x = chunk_x * 2;
        let noise_chunk_z = chunk_z * 2;

        self.scale_noise.get_region_2d(
            &mut sr,
            noise_chunk_x,
            noise_chunk_z,
            noise_x_size,
            noise_z_size,
            1.121,
            1.121,
            0.5,
        );
        self.depth_noise.get_region_2d(
            &mut dr,
            noise_chunk_x,
            noise_chunk_z,
            noise_x_size,
            noise_z_size,
            200.0,
            200.0,
            0.5,
        );
        self.perlin_noise1.get_region(
            &mut pnr,
            noise_chunk_x,
            0,
            noise_chunk_z,
            noise_x_size,
            noise_y_size,
            noise_z_size,
            s / 80.0,
            hs / 160.0,
            s / 80.0,
        );
        self.lperlin_noise1.get_region(
            &mut ar,
            noise_chunk_x,
            0,
            noise_chunk_z,
            noise_x_size,
            noise_y_size,
            noise_z_size,
            s,
            hs,
            s,
        );
        self.lperlin_noise2.get_region(
            &mut br,
            noise_chunk_x,
            0,
            noise_chunk_z,
            noise_x_size,
            noise_y_size,
            noise_z_size,
            s,
            hs,
            s,
        );

        // Generate 3D density buffer (matching reference getHeights exactly)
        let mut density = vec![0.0; (noise_x_size * noise_y_size * noise_z_size) as usize];
        let mut p = 0;

        for xx in 0..noise_x_size {
            for zz in 0..noise_z_size {
                let mut sss = 0.0f32;
                let mut ddd = 0.0f32;
                let mut pow = 0.0f32;

                let rr = 2;
                // Biome index: noise grid is 3x3, biome grid is 7x7 (with 2-chunk border)
                // Noise xx=0..2 maps to biome indices 2..4
                let mb = biomes[((xx + 2) + (zz + 2) * (noise_x_size + 4)) as usize];
                let mb_id = crate::biome::BiomeId::from_u8(mb as u8)
                    .unwrap_or(crate::biome::BiomeId::Ocean);
                let mb_biome = crate::biome::get_biome(mb_id)
                    .unwrap_or_else(|| panic!("Invalid biome id: {}", mb));

                for xb in -rr..=rr {
                    for zb in -rr..=rr {
                        let b_idx = ((xx + xb + 2) + (zz + zb + 2) * (noise_x_size + 4)) as usize;
                        let b = biomes[b_idx];
                        let b_id = crate::biome::BiomeId::from_u8(b as u8)
                            .unwrap_or(crate::biome::BiomeId::Ocean);
                        let biome = crate::biome::get_biome(b_id)
                            .unwrap_or_else(|| panic!("Invalid biome id: {}", b));
                        let ppp = self.pows[(xb + 2 + (zb + 2) * 5) as usize] / (biome.depth + 2.0);
                        if biome.depth > mb_biome.depth {
                            // Reference: if (b->depth > mb->depth) ppp /= 2;
                        }
                        sss += biome.scale * ppp;
                        ddd += biome.depth * ppp;
                        pow += ppp;
                    }
                }

                if pow > 0.0 {
                    sss /= pow;
                    ddd /= pow;
                }

                sss = sss * 0.9 + 0.1;
                ddd = (ddd * 4.0 - 1.0) / 8.0;

                // dr is indexed by column (xx, zz)
                let dr_idx = (xx * noise_z_size + zz) as usize;
                let rdepth = (dr[dr_idx] / 8000.0) as f64;
                let rdepth = if rdepth < 0.0 { -rdepth * 0.3 } else { rdepth };
                let rdepth = rdepth * 3.0 - 2.0;
                let rdepth = if rdepth < 0.0 {
                    let mut rd = rdepth / 2.0;
                    if rd < -1.0 {
                        rd = -1.0;
                    }
                    rd / 1.4 / 2.0
                } else {
                    let mut rd = if rdepth > 1.0 { 1.0 } else { rdepth };
                    rd / 8.0
                };

                for yy in 0..noise_y_size {
                    let mut depth = ddd as f64;
                    let mut scale = sss as f64;

                    depth += rdepth * 0.2;
                    depth = depth * noise_y_size as f64 / 16.0;

                    let y_center = noise_y_size as f64 / 2.0 + depth * 4.0;

                    let mut val = 0.0;

                    let y_offs =
                        (yy as f64 - y_center) * 12.0 * 128.0 / CHUNK_HEIGHT as f64 / scale;

                    let y_offs = if y_offs < 0.0 { y_offs * 4.0 } else { y_offs };

                    let bb = ar[p] / 512.0;
                    let cc = br[p] / 512.0;

                    let v = (pnr[p] / 10.0 + 1.0) / 2.0;
                    if v < 0.0 {
                        val = bb;
                    } else if v > 1.0 {
                        val = cc;
                    } else {
                        val = bb + (cc - bb) * v;
                    }
                    val -= y_offs;

                    if yy > noise_y_size - 4 {
                        let slide = (yy - (noise_y_size - 4)) as f64 / 3.0;
                        val = val * (1.0 - slide) + -10.0 * slide;
                    }

                    density[p] = val;
                    p += 1;
                }
            }
        }

        // Extract 17x17 heightmap from 3D density by finding surface (density > 0)
        // Heightmap covers 16x16 chunk + 1 border on each side = 17x17
        let mut heightmap = vec![0.0; 17 * 17];
        let nx_size = noise_x_size as usize;
        let nz_size = noise_z_size as usize;
        let ny_size = noise_y_size as usize;
        for hx in 0..17 {
            for hz in 0..17 {
                // Map heightmap coordinates to noise grid coordinates
                // Noise grid is 3x17x3 at 8-block intervals
                // Heightmap is 17x17 at 1-block intervals
                let nx = hx as f64 / 8.0; // 0..2.0
                let nz = hz as f64 / 8.0; // 0..2.0

                // Find surface by scanning vertical noise columns
                let mut height = 63.0; // default sea level
                for yy in (0..ny_size).rev() {
                    // Trilinear interpolation of density at (nx, yy, nz)
                    let x0 = nx.floor() as usize;
                    let x1 = (x0 + 1).min(nx_size - 1);
                    let z0 = nz.floor() as usize;
                    let z1 = (z0 + 1).min(nz_size - 1);
                    let fx = nx - x0 as f64;
                    let fz = nz - z0 as f64;

                    let idx00 = yy * nx_size * nz_size + x0 * nz_size + z0;
                    let idx01 = yy * nx_size * nz_size + x0 * nz_size + z1;
                    let idx10 = yy * nx_size * nz_size + x1 * nz_size + z0;
                    let idx11 = yy * nx_size * nz_size + x1 * nz_size + z1;

                    let d00 = density[idx00];
                    let d01 = density[idx01];
                    let d10 = density[idx10];
                    let d11 = density[idx11];

                    let dx0 = d00 + (d10 - d00) * fx;
                    let dx1 = d01 + (d11 - d01) * fx;
                    let d = dx0 + (dx1 - dx0) * fz;

                    if d > 0.0 {
                        height = yy as f64 * 8.0; // Convert noise y to block y
                        break;
                    }
                }
                heightmap[hz * 17 + hx] = height;
            }
        }

        heightmap
    }

    /// Build surfaces for a chunk (matches RandomLevelSource::buildSurfaces)
    pub fn build_surfaces(
        &mut self,
        chunk_x: i32,
        chunk_z: i32,
        blocks: &mut [u8],
        biomes: &[BiomeId],
    ) {
        let water_height = 63;

        // Get depth buffer from perlin_noise3
        let mut depth_buffer = vec![0.0; 16 * 16];
        let s = 1.0 / 32.0;
        self.perlin_noise3.get_region_2d(
            &mut depth_buffer,
            chunk_x * 16,
            chunk_z * 16,
            16,
            16,
            s * 2.0,
            s * 2.0,
            s * 2.0,
        );

        let mut random = GameRandom::with_seed(self.seed);
        let seed_val = ((chunk_x as i64 * 341873128712 + chunk_z as i64 * 132897987541) as i64);
        random.set_seed(seed_val);

        for x in 0..16 {
            for z in 0..16 {
                let biome = biomes[(z + x * 16) as usize];
                let biome_data = crate::biome::get_biome(biome)
                    .unwrap_or_else(|| panic!("Invalid biome id: {:?}", biome));

                let temp = biome_data.temperature;
                let run_depth = (depth_buffer[(x + z * 16) as usize] / 3.0
                    + 3.0
                    + random.next_double() * 0.25) as i32;

                let mut run = -1;
                let mut top = biome_data.top_material;
                let mut material = biome_data.filler_material;

                for y in (0..CHUNK_HEIGHT).rev() {
                    let offs = ((z * 16 + x) * CHUNK_HEIGHT + y) as usize;

                    if y <= 1 + random.next_int_bound(2) {
                        // Bedrock at bottom
                        blocks[offs] = 7; // bedrock
                    } else {
                        let old = blocks[offs];

                        if old == 0 {
                            run = -1;
                        } else if old == 1 {
                            // stone
                            if run == -1 {
                                if run_depth <= 0 {
                                    top = 0;
                                    material = 1; // stone
                                } else if y >= water_height - 4 && y <= water_height + 1 {
                                    top = biome_data.top_material;
                                    material = biome_data.filler_material;
                                }

                                if y < water_height && top == 0 {
                                    if temp < 0.15 {
                                        top = 79; // ice
                                    } else {
                                        top = 8; // water
                                    }
                                }

                                run = run_depth;
                                if y >= water_height - 1 {
                                    blocks[offs] = top;
                                } else {
                                    blocks[offs] = material;
                                }
                            } else if run > 0 {
                                run -= 1;
                                blocks[offs] = material;

                                // Sandstone under sand
                                if run == 0 && material == 12 {
                                    // sand
                                    run = random.next_int_bound(4);
                                    material = 24; // sandstone
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
