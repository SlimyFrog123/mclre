//! Structure generation matching LCE reference

use crate::mth::Mth;
use crate::terrain::TerrainGenerator;
use mclre_core::GameRandom;
use mclre_world::{BlockPos, ChunkPos};

pub trait StructureFeature {
    fn place(
        &mut self,
        _terrain: &mut TerrainGenerator,
        _chunk_x: i32,
        _chunk_z: i32,
        _blocks: &mut [u8],
    ) {
    }
    fn post_process(
        &mut self,
        _random: &mut mclre_core::GameRandom,
        _chunk_x: i32,
        _chunk_z: i32,
    ) -> bool {
        false
    }
}

/// Large cave feature (overworld) - matches LargeCaveFeature from reference
pub struct CaveFeature {
    seed: i64,
}

impl CaveFeature {
    pub fn new(seed: i64) -> Self {
        Self { seed }
    }

    fn add_tunnel(
        &self,
        random: &mut GameRandom,
        chunk_x: i32,
        chunk_z: i32,
        blocks: &mut [u8],
        mut x_cave: f64,
        mut y_cave: f64,
        mut z_cave: f64,
        thickness: f64,
        mut y_rot: f64,
        mut x_rot: f64,
        mut step: i32,
        mut dist: i32,
        y_scale: f64,
    ) {
        let x_mid = (chunk_x * 16 + 8) as f64;
        let z_mid = (chunk_z * 16 + 8) as f64;

        let mut y_rota = 0.0;
        let mut x_rota = 0.0;

        const GEN_DEPTH: i32 = 128;

        if dist <= 0 {
            let max = 16 * 16 - 16; // radius * 16 - 16
            dist = max - random.next_int_bound(max / 4) as i32;
        }
        let mut single_step = false;

        if step == -1 {
            step = dist / 2;
            single_step = true;
        }

        let split_point = random.next_int_bound(dist / 2) as i32 + dist / 4;
        let steep = random.next_int_bound(6) == 0;

        // Precompute radius squared per Y level (matching reference rs array)
        let mut rs = [0.0f64; 128];
        for i in 0..GEN_DEPTH {
            if i == 0 || random.next_int_bound(3) == 0 {
                let f = 1.0 + (random.next_float() as f64 * random.next_float() as f64) * 1.0;
                rs[i as usize] = f * f;
            } else {
                rs[i as usize] = rs[(i - 1) as usize];
            }
        }

        while step < dist {
            let step_f = step as f64;
            let dist_f = dist as f64;
            let rad = 1.5
                + (Mth::sin((step_f * std::f64::consts::PI / dist_f) as f32) as f64 * thickness)
                    * 1.0;
            let y_rad = rad * y_scale;

            let xc = Mth::cos(x_rot as f32) as f64;
            let xs = Mth::sin(x_rot as f32) as f64;
            x_cave += Mth::cos(y_rot as f32) as f64 * xc;
            y_cave += xs;
            z_cave += Mth::sin(y_rot as f32) as f64 * xc;

            if steep {
                x_rot *= 0.92;
            } else {
                x_rot *= 0.7;
            }
            x_rot += x_rota * 0.1;
            y_rot += y_rota * 0.1;

            x_rota *= 0.9;
            y_rota *= 0.75;
            x_rota += (random.next_float() as f64 - random.next_float() as f64)
                * random.next_float() as f64
                * 2.0;
            y_rota += (random.next_float() as f64 - random.next_float() as f64)
                * random.next_float() as f64
                * 4.0;

            if !single_step && step == split_point && thickness > 1.0 && dist > 0 {
                // Branch into two tunnels
                let new_thickness = random.next_float() as f64 * 0.5 + 0.5;
                self.add_tunnel(
                    random,
                    chunk_x,
                    chunk_z,
                    blocks,
                    x_cave,
                    y_cave,
                    z_cave,
                    new_thickness,
                    y_rot - std::f64::consts::PI / 2.0,
                    x_rot / 3.0,
                    step,
                    dist,
                    1.0,
                );
                self.add_tunnel(
                    random,
                    chunk_x,
                    chunk_z,
                    blocks,
                    x_cave,
                    y_cave,
                    z_cave,
                    new_thickness,
                    y_rot + std::f64::consts::PI / 2.0,
                    x_rot / 3.0,
                    step,
                    dist,
                    1.0,
                );
                return;
            }
            if !single_step && random.next_int_bound(4) == 0 {
                step += 1;
                continue;
            }

            // Check if tunnel is too far from chunk center
            let xd = x_cave - x_mid;
            let zd = z_cave - z_mid;
            let remaining = (dist - step) as f64;
            let rr = (thickness + 2.0) + 16.0;
            if xd * xd + zd * zd - remaining * remaining > rr * rr {
                return;
            }

            // Check chunk bounds
            if x_cave < x_mid - 16.0 - rad * 2.0
                || z_cave < z_mid - 16.0 - rad * 2.0
                || x_cave > x_mid + 16.0 + rad * 2.0
                || z_cave > z_mid + 16.0 + rad * 2.0
            {
                step += 1;
                continue;
            }

            let mut x0 = Mth::floor_f64(x_cave - rad) - chunk_x * 16 - 1;
            let mut x1 = Mth::floor_f64(x_cave + rad) - chunk_x * 16 + 1;
            let mut y0 = Mth::floor_f64(y_cave - y_rad) - 1;
            let mut y1 = Mth::floor_f64(y_cave + y_rad) + 1;
            let mut z0 = Mth::floor_f64(z_cave - rad) - chunk_z * 16 - 1;
            let mut z1 = Mth::floor_f64(z_cave + rad) - chunk_z * 16 + 1;

            x0 = x0.max(0);
            x1 = x1.min(16);
            y0 = y0.max(1);
            y1 = y1.min(GEN_DEPTH - 8);
            z0 = z0.max(0);
            z1 = z1.min(16);

            // Check for water
            let mut detected_water = false;
            for xx in x0..x1 {
                if detected_water {
                    break;
                }
                for zz in z0..z1 {
                    if detected_water {
                        break;
                    }
                    for yy in (y0 - 1..=y1 + 1).rev() {
                        if yy < 0 || yy >= GEN_DEPTH {
                            continue;
                        }
                        let p = (xx * 16 + zz) * GEN_DEPTH + yy;
                        if p >= 0 && p < blocks.len() as i32 {
                            let block = blocks[p as usize];
                            if block == 8 || block == 9 {
                                // water or calm water
                                detected_water = true;
                                break;
                            }
                        }
                        if yy != y0 - 1 && xx != x0 && xx != x1 - 1 && zz != z0 && zz != z1 - 1 {
                            break;
                        }
                    }
                }
            }
            if detected_water {
                step += 1;
                continue;
            }

            // Carve tunnel
            for xx in x0..x1 {
                let xd = ((xx + chunk_x * 16) as f64 + 0.5 - x_cave) / rad;
                for zz in z0..z1 {
                    let zd = ((zz + chunk_z * 16) as f64 + 0.5 - z_cave) / rad;
                    let mut p = (xx * 16 + zz) * GEN_DEPTH + y1;
                    let mut has_grass = false;
                    if xd * xd + zd * zd < 1.0 {
                        for yy in (y0..=y1 - 1).rev() {
                            let yd = (yy as f64 + 0.5 - y_cave) / y_rad;
                            if yd > -0.7 && xd * xd + yd * yd + zd * zd < 1.0 {
                                if p >= 0 && p < blocks.len() as i32 {
                                    let block = blocks[p as usize];
                                    if block == 2 {
                                        // grass
                                        has_grass = true;
                                    }
                                    if block == 1 || block == 3 || block == 2 {
                                        // stone, dirt, grass
                                        if yy < 10 {
                                            blocks[p as usize] = 10; // lava
                                        } else {
                                            blocks[p as usize] = 0; // air
                                            if has_grass && p > 0 && blocks[p as usize - 1] == 3 {
                                                // Would need biome to get top material - simplified
                                                blocks[p as usize - 1] = 2; // grass
                                            }
                                        }
                                    }
                                }
                            }
                            p -= 1;
                        }
                    }
                }
            }
            if single_step {
                break;
            }
            step += 1;
        }
    }
}

impl StructureFeature for CaveFeature {
    fn place(
        &mut self,
        _terrain: &mut TerrainGenerator,
        chunk_x: i32,
        chunk_z: i32,
        blocks: &mut [u8],
    ) {
        let mut random = GameRandom::with_seed(self.seed);
        let seed_val =
            ((chunk_x as i64 * 341873128712 + chunk_z as i64 * 132897987541) ^ self.seed) as i64;
        random.set_seed(seed_val);

        // Match reference: caves = random->nextInt(random->nextInt(random->nextInt(40) + 1) + 1)
        // if (random->nextInt(15) != 0) caves = 0;
        let r1 = random.next_int_bound(40) + 1;
        let r2 = random.next_int_bound(r1) + 1;
        let caves = random.next_int_bound(r2);
        if random.next_int_bound(15) != 0 {
            return;
        }

        for _ in 0..caves {
            let x_cave = chunk_x as f64 * 16.0 + random.next_int_bound(16) as f64;
            let inner = random.next_int_bound(128 - 8);
            let y_cave = random.next_int_bound(inner + 8) as f64;
            let z_cave = chunk_z as f64 * 16.0 + random.next_int_bound(16) as f64;

            let mut tunnels = 1;
            if random.next_int_bound(4) == 0 {
                // Add room
                let thickness = 1.0 + random.next_float() as f64 * 6.0;
                self.add_tunnel(
                    &mut random,
                    chunk_x,
                    chunk_z,
                    blocks,
                    x_cave,
                    y_cave,
                    z_cave,
                    thickness,
                    0.0,
                    0.0,
                    -1,
                    -1,
                    0.5,
                );
                tunnels += random.next_int_bound(4);
            }

            for _ in 0..tunnels {
                let y_rot = random.next_float() as f64 * std::f64::consts::PI * 2.0;
                let x_rot = ((random.next_float() as f64 - 0.5) * 2.0) / 8.0;
                let f1 = random.next_float() as f64;
                let f2 = random.next_float() as f64;
                let mut thickness = f1 * 2.0 + f2;
                if random.next_int_bound(10) == 0 {
                    let f3 = random.next_float() as f64;
                    let f4 = random.next_float() as f64;
                    thickness *= f3 * f4 * 3.0 + 1.0;
                }

                self.add_tunnel(
                    &mut random,
                    chunk_x,
                    chunk_z,
                    blocks,
                    x_cave,
                    y_cave,
                    z_cave,
                    thickness,
                    y_rot,
                    x_rot,
                    0,
                    0,
                    1.0,
                );
            }
        }
    }
    fn post_process(
        &mut self,
        _random: &mut mclre_core::GameRandom,
        _chunk_x: i32,
        _chunk_z: i32,
    ) -> bool {
        false
    }
}

/// Canyon/Ravine feature - matches CanyonFeature from reference
pub struct CanyonFeature {
    seed: i64,
}

impl CanyonFeature {
    pub fn new(seed: i64) -> Self {
        Self { seed }
    }

    fn add_tunnel(
        &self,
        random: &mut GameRandom,
        chunk_x: i32,
        chunk_z: i32,
        blocks: &mut [u8],
        mut x_cave: f64,
        mut y_cave: f64,
        mut z_cave: f64,
        thickness: f64,
        mut y_rot: f64,
        mut x_rot: f64,
        mut step: i32,
        mut dist: i32,
        y_scale: f64,
    ) {
        let x_mid = (chunk_x * 16 + 8) as f64;
        let z_mid = (chunk_z * 16 + 8) as f64;

        let mut y_rota = 0.0;
        let mut x_rota = 0.0;

        const GEN_DEPTH: i32 = 128;

        if dist <= 0 {
            let max = 16 * 16 - 16;
            dist = max - random.next_int_bound(max / 4) as i32;
        }
        let mut single_step = false;

        if step == -1 {
            step = dist / 2;
            single_step = true;
        }

        let mut rs = [0.0f64; 128];
        for i in 0..GEN_DEPTH {
            if i == 0 || random.next_int_bound(3) == 0 {
                let f = 1.0 + (random.next_float() as f64 * random.next_float() as f64) * 1.0;
                rs[i as usize] = f * f;
            } else {
                rs[i as usize] = rs[(i - 1) as usize];
            }
        }

        while step < dist {
            let step_f = step as f64;
            let dist_f = dist as f64;
            let rad = 1.5
                + (Mth::sin((step_f * std::f64::consts::PI / dist_f) as f32) as f64 * thickness)
                    * 1.0;
            let y_rad = rad * y_scale;

            let xc = Mth::cos(x_rot as f32) as f64;
            let xs = Mth::sin(x_rot as f32) as f64;
            x_cave += Mth::cos(y_rot as f32) as f64 * xc;
            y_cave += xs;
            z_cave += Mth::sin(y_rot as f32) as f64 * xc;

            x_rot *= 0.7;
            x_rot += x_rota * 0.05;
            y_rot += y_rota * 0.05;

            x_rota *= 0.8;
            y_rota *= 0.5;
            x_rota += (random.next_float() as f64 - random.next_float() as f64)
                * random.next_float() as f64
                * 2.0;
            y_rota += (random.next_float() as f64 - random.next_float() as f64)
                * random.next_float() as f64
                * 4.0;

            if !single_step && random.next_int_bound(4) == 0 {
                step += 1;
                continue;
            }

            // Check bounds
            let xd = x_cave - x_mid;
            let zd = z_cave - z_mid;
            let remaining = (dist - step) as f64;
            let rr = (thickness + 2.0) + 16.0;
            if xd * xd + zd * zd - remaining * remaining > rr * rr {
                return;
            }

            if x_cave < x_mid - 16.0 - rad * 2.0
                || z_cave < z_mid - 16.0 - rad * 2.0
                || x_cave > x_mid + 16.0 + rad * 2.0
                || z_cave > z_mid + 16.0 + rad * 2.0
            {
                step += 1;
                continue;
            }

            let mut x0 = Mth::floor_f64(x_cave - rad) - chunk_x * 16 - 1;
            let mut x1 = Mth::floor_f64(x_cave + rad) - chunk_x * 16 + 1;
            let mut y0 = Mth::floor_f64(y_cave - y_rad) - 1;
            let mut y1 = Mth::floor_f64(y_cave + y_rad) + 1;
            let mut z0 = Mth::floor_f64(z_cave - rad) - chunk_z * 16 - 1;
            let mut z1 = Mth::floor_f64(z_cave + rad) - chunk_z * 16 + 1;

            x0 = x0.max(0);
            x1 = x1.min(16);
            y0 = y0.max(1);
            y1 = y1.min(GEN_DEPTH - 8);
            z0 = z0.max(0);
            z1 = z1.min(16);

            // Check for water
            let mut detected_water = false;
            for xx in x0..x1 {
                if detected_water {
                    break;
                }
                for zz in z0..z1 {
                    if detected_water {
                        break;
                    }
                    for yy in (y0 - 1..=y1 + 1).rev() {
                        if yy < 0 || yy >= GEN_DEPTH {
                            continue;
                        }
                        let p = (xx * 16 + zz) * GEN_DEPTH + yy;
                        if p >= 0 && p < blocks.len() as i32 {
                            let block = blocks[p as usize];
                            if block == 8 || block == 9 {
                                // water or calm water
                                detected_water = true;
                                break;
                            }
                        }
                        if yy != y0 - 1 && xx != x0 && xx != x1 - 1 && zz != z0 && zz != z1 - 1 {
                            break;
                        }
                    }
                }
            }
            if detected_water {
                step += 1;
                continue;
            }

            // Carve canyon
            for xx in x0..x1 {
                let xd = ((xx + chunk_x * 16) as f64 + 0.5 - x_cave) / rad;
                for zz in z0..z1 {
                    let zd = ((zz + chunk_z * 16) as f64 + 0.5 - z_cave) / rad;
                    let mut p = (xx * 16 + zz) * GEN_DEPTH + y1;
                    let mut has_grass = false;
                    if xd * xd + zd * zd < 1.0 {
                        for yy in (y0..=y1 - 1).rev() {
                            let yd = (yy as f64 + 0.5 - y_cave) / y_rad;
                            if (xd * xd + zd * zd) * rs[yy as usize] + (yd * yd / 6.0) < 1.0 {
                                if p >= 0 && p < blocks.len() as i32 {
                                    let block = blocks[p as usize];
                                    if block == 2 {
                                        // grass
                                        has_grass = true;
                                    }
                                    if block == 1 || block == 3 || block == 2 {
                                        // stone, dirt, grass
                                        if yy < 10 {
                                            blocks[p as usize] = 10; // lava
                                        } else {
                                            blocks[p as usize] = 0; // air
                                            if has_grass && p > 0 && blocks[p as usize - 1] == 3 {
                                                blocks[p as usize - 1] = 2; // grass
                                            }
                                        }
                                    }
                                }
                            }
                            p -= 1;
                        }
                    }
                }
            }
            if single_step {
                break;
            }
            step += 1;
        }
    }
}

impl StructureFeature for CanyonFeature {
    fn place(
        &mut self,
        _terrain: &mut TerrainGenerator,
        chunk_x: i32,
        chunk_z: i32,
        blocks: &mut [u8],
    ) {
        let mut random = GameRandom::with_seed(self.seed);
        let seed_val =
            ((chunk_x as i64 * 341873128712 + chunk_z as i64 * 132897987541) ^ self.seed) as i64;
        random.set_seed(seed_val);

        // Match reference: if (random->nextInt(50) != 0) return;
        if random.next_int_bound(50) != 0 {
            return;
        }

        let x_cave = chunk_x as f64 * 16.0 + random.next_int_bound(16) as f64;
        let inner = random.next_int_bound(40);
        let y_cave = random.next_int_bound(inner + 8) as f64 + 20.0;
        let z_cave = chunk_z as f64 * 16.0 + random.next_int_bound(16) as f64;

        let tunnels = 1;

        for _ in 0..tunnels {
            let y_rot = random.next_float() as f64 * std::f64::consts::PI * 2.0;
            let x_rot = ((random.next_float() as f64 - 0.5) * 2.0) / 8.0;
            let f1 = random.next_float() as f64;
            let f2 = random.next_float() as f64;
            let thickness = (f1 * 2.0 + f2) * 2.0;

            self.add_tunnel(
                &mut random,
                chunk_x,
                chunk_z,
                blocks,
                x_cave,
                y_cave,
                z_cave,
                thickness,
                y_rot,
                x_rot,
                0,
                0,
                3.0,
            );
        }
    }
    fn post_process(
        &mut self,
        _random: &mut mclre_core::GameRandom,
        _chunk_x: i32,
        _chunk_z: i32,
    ) -> bool {
        false
    }
}

// Village feature - matches VillageFeature from reference
pub struct VillageFeature {
    world_size: i32,
    town_spacing: i32,
    min_town_separation: i32,
}

impl VillageFeature {
    pub fn new(ws: i32) -> Self {
        let town_spacing = if ws < 128 { 16 } else { 32 }; // Small worlds: 16, large: 32
        Self {
            world_size: ws,
            town_spacing,
            min_town_separation: 8,
        }
    }

    fn is_feature_chunk(&self, chunk_x: i32, chunk_z: i32, seed: i64) -> bool {
        let mut xx = chunk_x;
        let mut zz = chunk_z;
        let spacing = self.town_spacing;

        if chunk_x < 0 {
            xx -= spacing - 1;
        }
        if chunk_z < 0 {
            zz -= spacing - 1;
        }

        let x_center = xx / spacing;
        let z_center = zz / spacing;

        let mut random = GameRandom::with_seed(seed);
        random.set_seed(
            ((x_center as i64 * 341873128712 + z_center as i64 * 132897987541) ^ 10387312) as i64,
        );

        let mut x_center_chunk = x_center * spacing;
        let mut z_center_chunk = z_center * spacing;
        x_center_chunk += random.next_int_bound(spacing - self.min_town_separation);
        z_center_chunk += random.next_int_bound(spacing - self.min_town_separation);

        chunk_x == x_center_chunk && chunk_z == z_center_chunk
    }
}

impl StructureFeature for VillageFeature {
    fn place(&mut self, _t: &mut TerrainGenerator, _x: i32, _z: i32, _b: &mut [u8]) {}
    fn post_process(
        &mut self,
        random: &mut mclre_core::GameRandom,
        chunk_x: i32,
        chunk_z: i32,
    ) -> bool {
        // Check if this is a village chunk
        if !self.is_feature_chunk(chunk_x, chunk_z, random.seed()) {
            return false;
        }
        // Check biome (plains or desert)
        // Simplified: just return true for now, biome check would need biome source
        true
    }
}

/// Stronghold feature - matches StrongholdFeature from reference
pub struct StrongholdFeature {
    positions: Vec<(i32, i32)>,
}

impl StrongholdFeature {
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
        }
    }

    fn is_feature_chunk(&self, chunk_x: i32, chunk_z: i32) -> bool {
        for &(x, z) in &self.positions {
            if chunk_x == x && chunk_z == z {
                return true;
            }
        }
        false
    }

    pub fn generate_positions(&mut self, seed: i64) {
        if !self.positions.is_empty() {
            return;
        }
        let mut random = GameRandom::with_seed(seed);
        let angle = random.next_double() * std::f64::consts::PI * 2.0;
        let distance = 32;
        let spread = 3;

        for circle in 1..=3 {
            let circle_distance = distance * circle;
            let circle_spread = spread * circle;
            let count = 3 * circle;

            for i in 0..count {
                let theta = angle + (i as f64 / count as f64) * std::f64::consts::PI * 2.0;
                let mut r = GameRandom::with_seed(seed);
                r.set_seed(((circle as i64 * 1000 + i as i64) ^ seed) as i64);
                let offset_x = (r.next_double() * 2.0 - 1.0) * circle_spread as f64;
                let offset_z = (r.next_double() * 2.0 - 1.0) * circle_spread as f64;
                let x = (theta.cos() * circle_distance as f64 + offset_x) as i32;
                let z = (theta.sin() * circle_distance as f64 + offset_z) as i32;
                self.positions.push((x, z));
            }
        }
    }

    pub fn get_nearest_generated_feature(
        &self,
        x: i32,
        _y: i32,
        z: i32,
    ) -> Option<mclre_world::BlockPos> {
        let mut best = None;
        let mut best_dist = i64::MAX;
        for &(px, pz) in &self.positions {
            let dx = (px - x) as i64;
            let dz = (pz - z) as i64;
            let dist = dx * dx + dz * dz;
            if dist < best_dist {
                best_dist = dist;
                best = Some(mclre_world::BlockPos::new(px * 16, 64, pz * 16));
            }
        }
        best
    }
}

impl StructureFeature for StrongholdFeature {
    fn place(&mut self, _t: &mut TerrainGenerator, _x: i32, _z: i32, _b: &mut [u8]) {}
    fn post_process(
        &mut self,
        random: &mut mclre_core::GameRandom,
        chunk_x: i32,
        chunk_z: i32,
    ) -> bool {
        self.generate_positions(random.seed());
        self.is_feature_chunk(chunk_x, chunk_z)
    }
}

/// Mine shaft feature - matches MineShaftFeature from reference
pub struct MineShaftFeature {
    chance: f64,
}

impl MineShaftFeature {
    pub fn new() -> Self {
        Self { chance: 0.01 }
    }
}

impl StructureFeature for MineShaftFeature {
    fn place(&mut self, _t: &mut TerrainGenerator, _x: i32, _z: i32, _b: &mut [u8]) {}
    fn post_process(
        &mut self,
        random: &mut mclre_core::GameRandom,
        chunk_x: i32,
        chunk_z: i32,
    ) -> bool {
        // Match reference: forcePlacement || (random->nextDouble() < chance && random->nextInt(80) < max(abs(x), abs(z)))
        let force = false; // No level gen options in our implementation
        let r1 = random.next_double();
        let r2 = random.next_int_bound(80);
        let max_dist = chunk_x.abs().max(chunk_z.abs());
        force || (r1 < self.chance && r2 < max_dist)
    }
}

/// Random scattered large feature (temples) - matches RandomScatteredLargeFeature from reference
pub struct RandomScatteredLargeFeature {
    spacing: i32,
    min_separation: i32,
}

impl RandomScatteredLargeFeature {
    pub fn new() -> Self {
        Self {
            spacing: 32,
            min_separation: 8,
        }
    }

    fn is_feature_chunk(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        biome: crate::biome::BiomeId,
        seed: i64,
    ) -> bool {
        let mut xx = chunk_x;
        let mut zz = chunk_z;
        let spacing = self.spacing;

        if chunk_x < 0 {
            xx -= spacing - 1;
        }
        if chunk_z < 0 {
            zz -= spacing - 1;
        }

        let x_center = xx / spacing;
        let z_center = zz / spacing;

        let mut random = GameRandom::with_seed(seed);
        random.set_seed(
            ((x_center as i64 * 341873128712 + z_center as i64 * 132897987541) ^ 14357617) as i64,
        );

        let mut x_center_chunk = x_center * spacing;
        let mut z_center_chunk = z_center * spacing;
        x_center_chunk += random.next_int_bound(spacing - self.min_separation);
        z_center_chunk += random.next_int_bound(spacing - self.min_separation);

        if !(chunk_x == x_center_chunk && chunk_z == z_center_chunk) {
            return false;
        }

        // Check biome (desert, desertHills, jungle, jungleHills, swampland)
        matches!(
            biome,
            crate::biome::BiomeId::Desert
                | crate::biome::BiomeId::DesertHills
                | crate::biome::BiomeId::Jungle
                | crate::biome::BiomeId::JungleHills
                | crate::biome::BiomeId::Swampland
        )
    }
}

impl StructureFeature for RandomScatteredLargeFeature {
    fn place(&mut self, _t: &mut TerrainGenerator, _x: i32, _z: i32, _b: &mut [u8]) {}
    fn post_process(
        &mut self,
        random: &mut mclre_core::GameRandom,
        chunk_x: i32,
        chunk_z: i32,
    ) -> bool {
        // This is called without biome, so we just do the chunk coordinate check
        // The biome check will be done by the caller
        let mut xx = chunk_x;
        let mut zz = chunk_z;
        let spacing = self.spacing;

        if chunk_x < 0 {
            xx -= spacing - 1;
        }
        if chunk_z < 0 {
            zz -= spacing - 1;
        }

        let x_center = xx / spacing;
        let z_center = zz / spacing;

        let mut rnd = GameRandom::with_seed(random.seed());
        rnd.set_seed(
            ((x_center as i64 * 341873128712 + z_center as i64 * 132897987541) ^ 14357617) as i64,
        );

        let mut x_center_chunk = x_center * spacing;
        let mut z_center_chunk = z_center * spacing;
        x_center_chunk += rnd.next_int_bound(spacing - self.min_separation);
        z_center_chunk += rnd.next_int_bound(spacing - self.min_separation);

        chunk_x == x_center_chunk && chunk_z == z_center_chunk
    }
}

impl RandomScatteredLargeFeature {
    /// Check if this chunk should have a scattered feature with biome check
    pub fn is_feature_chunk_with_biome(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        biome: crate::biome::BiomeId,
        seed: i64,
    ) -> bool {
        // First check chunk coordinates
        if !self.is_feature_chunk_only(chunk_x, chunk_z, seed) {
            return false;
        }
        // Then check biome
        matches!(
            biome,
            crate::biome::BiomeId::Desert
                | crate::biome::BiomeId::DesertHills
                | crate::biome::BiomeId::Jungle
                | crate::biome::BiomeId::JungleHills
                | crate::biome::BiomeId::Swampland
        )
    }

    fn is_feature_chunk_only(&self, chunk_x: i32, chunk_z: i32, seed: i64) -> bool {
        let mut xx = chunk_x;
        let mut zz = chunk_z;
        let spacing = self.spacing;

        if chunk_x < 0 {
            xx -= spacing - 1;
        }
        if chunk_z < 0 {
            zz -= spacing - 1;
        }

        let x_center = xx / spacing;
        let z_center = zz / spacing;

        let mut random = GameRandom::with_seed(seed);
        random.set_seed(
            ((x_center as i64 * 341873128712 + z_center as i64 * 132897987541) ^ 14357617) as i64,
        );

        let mut x_center_chunk = x_center * spacing;
        let mut z_center_chunk = z_center * spacing;
        x_center_chunk += random.next_int_bound(spacing - self.min_separation);
        z_center_chunk += random.next_int_bound(spacing - self.min_separation);

        chunk_x == x_center_chunk && chunk_z == z_center_chunk
    }

    fn generate_witch_hut(&self, _b: &mut [u8], _x: i32, _z: i32, _r: &mut mclre_core::GameRandom) {
    }
    pub fn is_swamphut(&self, _x: i32, _y: i32, _z: i32) -> bool {
        false
    }
    pub fn get_swamphut_enemies(&self) -> Vec<u32> {
        vec![]
    }
}

/// Lake feature - matches LakeFeature from reference
pub struct LakeFeature {
    block_id: u8,
}

impl LakeFeature {
    pub fn new(id: u8) -> Self {
        Self { block_id: id }
    }

    pub fn place(
        &self,
        blocks: &mut [u8],
        x: i32,
        y: i32,
        z: i32,
        random: &mut mclre_core::GameRandom,
    ) {
        let mut wx = x - 8;
        let mut wz = z - 8;
        let mut wy = y;

        // Find ground level
        while wy > 5 {
            let idx = ((wz * 16 + wx) * 128 + wy) as usize;
            if idx < blocks.len() && blocks[idx] != 0 {
                break;
            }
            wy -= 1;
        }
        if wy <= 4 {
            return;
        }

        wy -= 4;

        // Check game rule options (simplified - always allow)

        let mut grid = [[[false; 8]; 16]; 16];

        let spots = random.next_int_bound(4) + 4;
        for _ in 0..spots {
            let xr = (random.next_double() * 6.0 + 3.0) as f64;
            let yr = (random.next_double() * 4.0 + 2.0) as f64;
            let zr = (random.next_double() * 6.0 + 3.0) as f64;

            let xp = random.next_double() * (16.0 - xr - 2.0) + 1.0 + xr / 2.0;
            let yp = random.next_double() * (8.0 - yr - 4.0) + 2.0 + yr / 2.0;
            let zp = random.next_double() * (16.0 - zr - 2.0) + 1.0 + zr / 2.0;

            for xx in 1..15 {
                for zz in 1..15 {
                    for yy in 1..7 {
                        let xd = ((xx as f64 - xp) / (xr / 2.0));
                        let yd = ((yy as f64 - yp) / (yr / 2.0));
                        let zd = ((zz as f64 - zp) / (zr / 2.0));
                        let d = xd * xd + yd * yd + zd * zd;
                        if d < 1.0 {
                            grid[xx][zz][yy] = true;
                        }
                    }
                }
            }
        }

        for xx in 0..16 {
            for zz in 0..16 {
                for yy in 0..8 {
                    if grid[xx][zz][yy] {
                        let gx = wx + xx as i32;
                        let gy = wy + yy as i32;
                        let gz = wz + zz as i32;
                        if gx >= 0 && gx < 16 && gy >= 0 && gy < 128 && gz >= 0 && gz < 16 {
                            let idx = ((gz * 16 + gx) * 128 + gy) as usize;
                            if idx < blocks.len() {
                                blocks[idx] = self.block_id;
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Monster room (dungeon) feature - matches MonsterRoomFeature from reference
pub struct MonsterRoomFeature;

impl MonsterRoomFeature {
    pub fn new() -> Self {
        Self
    }

    pub fn place(
        &self,
        blocks: &mut [u8],
        x: i32,
        y: i32,
        z: i32,
        random: &mut mclre_core::GameRandom,
    ) {
        let hr = 3;
        let xr = random.next_int_bound(2) + 2;
        let zr = random.next_int_bound(2) + 2;

        // Check if valid position (solid floor/ceiling, 1-5 wall openings)
        let mut hole_count = 0;
        for xx in (x - xr - 1)..=(x + xr + 1) {
            for yy in (y - 1)..=(y + hr + 1) {
                for zz in (z - zr - 1)..=(z + zr + 1) {
                    let idx = ((zz * 16 + xx) * 128 + yy) as usize;
                    if idx >= blocks.len() {
                        return;
                    }
                    let material = blocks[idx];

                    if yy == y - 1 && material == 0 {
                        return;
                    } // floor must be solid
                    if yy == y + hr + 1 && material == 0 {
                        return;
                    } // ceiling must be solid

                    if xx == x - xr - 1 || xx == x + xr + 1 || zz == z - zr - 1 || zz == z + zr + 1
                    {
                        if yy == y {
                            let above_idx = ((zz * 16 + xx) * 128 + yy + 1) as usize;
                            if material == 0 && above_idx < blocks.len() && blocks[above_idx] == 0 {
                                hole_count += 1;
                            }
                        }
                    }
                }
            }
        }

        if hole_count < 1 || hole_count > 5 {
            return;
        }

        // Build the room
        for xx in (x - xr - 1)..=(x + xr + 1) {
            for yy in (y - 1..=y + hr + 1).rev() {
                for zz in (z - zr - 1)..=(z + zr + 1) {
                    let idx = ((zz * 16 + xx) * 128 + yy) as usize;
                    if idx >= blocks.len() {
                        continue;
                    }

                    let is_wall = xx == x - xr - 1
                        || yy == y - 1
                        || zz == z - zr - 1
                        || xx == x + xr + 1
                        || yy == y + hr + 1
                        || zz == z + zr + 1;

                    if is_wall {
                        // Check if there's solid material below
                        let below_idx = ((zz * 16 + xx) * 128 + yy - 1) as usize;
                        let below_solid = below_idx < blocks.len() && blocks[below_idx] != 0;

                        if yy >= 0 && !below_solid {
                            blocks[idx] = 0; // remove
                        } else if blocks[idx] != 0 {
                            // solid material
                            if yy == y - 1 && random.next_int_bound(4) != 0 {
                                blocks[idx] = 48; // mossy cobblestone
                            } else {
                                blocks[idx] = 4; // cobblestone
                            }
                        }
                    } else {
                        blocks[idx] = 0; // air inside
                    }
                }
            }
        }

        // Place chests and spawner
        for _ in 0..2 {
            for _ in 0..3 {
                let xc = x + random.next_int_bound(xr * 2 + 1) - xr;
                let yc = y;
                let zc = z + random.next_int_bound(zr * 2 + 1) - zr;
                let idx = ((zc * 16 + xc) * 128 + yc) as usize;
                if idx >= blocks.len() || blocks[idx] != 0 {
                    continue;
                }

                // Place spawner in center
                if xc == x && zc == z {
                    blocks[idx] = 52; // monster spawner
                } else {
                    // Place chest with loot (simplified - just place chest)
                    blocks[idx] = 54; // chest
                }
            }
        }
    }
}
