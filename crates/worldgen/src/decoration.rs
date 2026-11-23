//! Biome decoration (trees, ores, flowers, tall grass, etc.) matching LCE reference

use crate::biome::{Biome, BiomeId};
use crate::mth::Mth;
use mclre_core::GameRandom;
use mclre_world::ChunkPos;

/// Decoration context for a chunk column
pub struct DecorationContext {
    pub biome: BiomeId,
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub random: GameRandom,
}

impl DecorationContext {
    pub fn new(biome: BiomeId, cx: i32, cz: i32, seed: i64) -> Self {
        let mut r = mclre_core::GameRandom::with_seed(seed);
        r.set_seed((cx as i64 * 341873128712 + cz as i64 * 132897987541) as i64);
        Self {
            biome,
            chunk_x: cx,
            chunk_z: cz,
            random: r,
        }
    }
}

/// Biome decorator matching LCE BiomeDecorator
pub struct BiomeDecorator<'a> {
    biome: &'a Biome,
    random: &'a mut GameRandom,
    chunk_x: i32,
    chunk_z: i32,
    blocks: &'a mut [u8],
    xo: i32,
    zo: i32,
}

impl<'a> BiomeDecorator<'a> {
    pub fn new(
        biome: &'a Biome,
        random: &'a mut GameRandom,
        chunk_x: i32,
        chunk_z: i32,
        blocks: &'a mut [u8],
    ) -> Self {
        Self {
            biome,
            random,
            chunk_x,
            chunk_z,
            blocks,
            xo: chunk_x * 16,
            zo: chunk_z * 16,
        }
    }

    pub fn decorate(&mut self) {
        // Decorate ores
        self.decorate_ores();

        // Decorate sand/clay/gravel
        for _ in 0..self.biome.sand_count {
            let x = self.xo + self.random.next_int_bound(16) + 8;
            let z = self.zo + self.random.next_int_bound(16) + 8;
            let y = self.get_top_solid_block(x, z);
            self.place_sand(x, y, z);
        }

        for _ in 0..self.biome.clay_count {
            let x = self.xo + self.random.next_int_bound(16) + 8;
            let z = self.zo + self.random.next_int_bound(16) + 8;
            let y = self.get_top_solid_block(x, z);
            self.place_clay(x, y, z);
        }

        for _ in 0..self.biome.gravel_count {
            let x = self.xo + self.random.next_int_bound(16) + 8;
            let z = self.zo + self.random.next_int_bound(16) + 8;
            let y = self.get_top_solid_block(x, z);
            self.place_gravel(x, y, z);
        }

        // Decorate trees
        let mut forests = self.biome.tree_count;
        if self.random.next_int_bound(10) == 0 {
            forests += 1;
        }
        for _ in 0..forests.max(0) {
            let x = self.xo + self.random.next_int_bound(16) + 8;
            let z = self.zo + self.random.next_int_bound(16) + 8;
            let y = self.get_heightmap(x, z);
            self.place_tree(x, y, z);
        }

        // Decorate huge mushrooms
        for _ in 0..self.biome.huge_mushrooms {
            let x = self.xo + self.random.next_int_bound(16) + 8;
            let z = self.zo + self.random.next_int_bound(16) + 8;
            let y = self.get_heightmap(x, z);
            self.place_huge_mushroom(x, y, z);
        }

        // Decorate flowers
        for _ in 0..self.biome.flower_count.max(0) {
            let x = self.xo + self.random.next_int_bound(16) + 8;
            let y = self.random.next_int_bound(128);
            let z = self.zo + self.random.next_int_bound(16) + 8;
            self.place_yellow_flower(x, y, z);

            if self.random.next_int_bound(4) == 0 {
                let x = self.xo + self.random.next_int_bound(16) + 8;
                let y = self.random.next_int_bound(128);
                let z = self.zo + self.random.next_int_bound(16) + 8;
                self.place_rose_flower(x, y, z);
            }
        }

        // Decorate tall grass
        for _ in 0..self.biome.grass_count.max(0) {
            let x = self.xo + self.random.next_int_bound(16) + 8;
            let y = self.random.next_int_bound(128);
            let z = self.zo + self.random.next_int_bound(16) + 8;
            self.place_tall_grass(x, y, z);
        }

        // Decorate dead bush, waterlily, mushrooms, reeds, pumpkins, cacti
        if self.biome.dead_bush_count > 0 {
            for _ in 0..self.biome.dead_bush_count {
                let x = self.xo + self.random.next_int_bound(16) + 8;
                let y = self.random.next_int_bound(128);
                let z = self.zo + self.random.next_int_bound(16) + 8;
                self.place_dead_bush(x, y, z);
            }
        }

        for _ in 0..self.biome.waterlily_count {
            let x = self.xo + self.random.next_int_bound(16) + 8;
            let z = self.zo + self.random.next_int_bound(16) + 8;
            let mut y = self.random.next_int_bound(128);
            while y > 0 && self.get_block(x, y - 1, z) == 0 {
                y -= 1;
            }
            self.place_waterlily(x, y, z);
        }

        for _ in 0..self.biome.mushroom_count {
            if self.random.next_int_bound(4) == 0 {
                let x = self.xo + self.random.next_int_bound(16) + 8;
                let z = self.zo + self.random.next_int_bound(16) + 8;
                let y = self.get_heightmap(x, z);
                self.place_brown_mushroom(x, y, z);
            }

            if self.random.next_int_bound(8) == 0 {
                let x = self.xo + self.random.next_int_bound(16) + 8;
                let z = self.zo + self.random.next_int_bound(16) + 8;
                let y = self.random.next_int_bound(128);
                self.place_red_mushroom(x, y, z);
            }
        }

        if self.random.next_int_bound(4) == 0 {
            let x = self.xo + self.random.next_int_bound(16) + 8;
            let y = self.random.next_int_bound(128);
            let z = self.zo + self.random.next_int_bound(16) + 8;
            self.place_brown_mushroom(x, y, z);
        }

        if self.random.next_int_bound(8) == 0 {
            let x = self.xo + self.random.next_int_bound(16) + 8;
            let y = self.random.next_int_bound(128);
            let z = self.zo + self.random.next_int_bound(16) + 8;
            self.place_red_mushroom(x, y, z);
        }

        for _ in 0..self.biome.reeds_count {
            let x = self.xo + self.random.next_int_bound(16) + 8;
            let z = self.zo + self.random.next_int_bound(16) + 8;
            let y = self.random.next_int_bound(128);
            self.place_reeds(x, y, z);
        }

        for _ in 0..10 {
            let x = self.xo + self.random.next_int_bound(16) + 8;
            let y = self.random.next_int_bound(128);
            let z = self.zo + self.random.next_int_bound(16) + 8;
            self.place_reeds(x, y, z);
        }

        if self.random.next_int_bound(32) == 0 {
            let x = self.xo + self.random.next_int_bound(16) + 8;
            let y = self.random.next_int_bound(128);
            let z = self.zo + self.random.next_int_bound(16) + 8;
            self.place_pumpkin(x, y, z);
        }

        for _ in 0..self.biome.cactus_count {
            let x = self.xo + self.random.next_int_bound(16) + 8;
            let y = self.random.next_int_bound(128);
            let z = self.zo + self.random.next_int_bound(16) + 8;
            self.place_cactus(x, y, z);
        }

        // Decorate liquids (water/lava springs)
        if self.biome.liquids {
            // Water springs
            for _ in 0..50 {
                let x = self.xo + self.random.next_int_bound(16) + 8;
                let y1 = self.random.next_int_bound(128 - 8) + 8;
                let y = self.random.next_int_bound(y1);
                let z = self.zo + self.random.next_int_bound(16) + 8;
                self.place_water_spring(x, y, z);
            }

            // Lava springs
            for _ in 0..20 {
                let x = self.xo + self.random.next_int_bound(16) + 8;
                let y1 = self.random.next_int_bound(128 - 16) + 8;
                let y2 = self.random.next_int_bound(y1) + 8;
                let y = self.random.next_int_bound(y2);
                let z = self.zo + self.random.next_int_bound(16) + 8;
                self.place_lava_spring(x, y, z);
            }
        }
    }

    fn get_top_solid_block(&self, x: i32, z: i32) -> i32 {
        for y in (0..128).rev() {
            let idx = ((z - self.zo) * 16 + (x - self.xo)) * 128 + y;
            if idx >= 0 && idx < self.blocks.len() as i32 && self.blocks[idx as usize] != 0 {
                return y;
            }
        }
        0
    }

    fn get_heightmap(&self, x: i32, z: i32) -> i32 {
        // Find the highest non-air block
        for y in (0..128).rev() {
            let idx = ((z - self.zo) * 16 + (x - self.xo)) * 128 + y;
            if idx >= 0 && idx < self.blocks.len() as i32 && self.blocks[idx as usize] != 0 {
                return y + 1;
            }
        }
        64 // default sea level
    }

    fn get_block(&self, x: i32, y: i32, z: i32) -> u8 {
        let idx = ((z - self.zo) * 16 + (x - self.xo)) * 128 + y;
        if idx >= 0 && idx < self.blocks.len() as i32 {
            self.blocks[idx as usize]
        } else {
            0
        }
    }

    fn set_block(&mut self, x: i32, y: i32, z: i32, id: u8) {
        let idx = ((z - self.zo) * 16 + (x - self.xo)) * 128 + y;
        if idx >= 0 && idx < self.blocks.len() as i32 {
            self.blocks[idx as usize] = id;
        }
    }

    fn can_place_flower(&self, x: i32, y: i32, z: i32) -> bool {
        if y <= 0 || y >= 127 {
            return false;
        }
        let block_below = self.get_block(x, y - 1, z);
        // Grass block (2) or dirt (3)
        block_below == 2 || block_below == 3
    }

    fn can_place_tall_grass(&self, x: i32, y: i32, z: i32) -> bool {
        if y <= 0 || y >= 127 {
            return false;
        }
        let block_below = self.get_block(x, y - 1, z);
        // Grass block (2) or dirt (3)
        block_below == 2 || block_below == 3
    }

    // Ore generation
    fn decorate_ores(&mut self) {
        // Coal ore
        self.generate_ore_vein(16, 0, 128, 16, 16);
        // Iron ore
        self.generate_ore_vein(8, 0, 64, 8, 8);
        // Gold ore
        self.generate_ore_vein(8, 0, 32, 8, 8);
        // Redstone ore
        self.generate_ore_vein(7, 0, 16, 7, 7);
        // Diamond ore
        self.generate_ore_vein(7, 0, 16, 7, 7);
        // Lapis ore
        self.generate_ore_vein(6, 0, 32, 6, 6);
        // Dirt
        self.generate_ore_vein(32, 0, 128, 32, 32);
        // Gravel
        self.generate_ore_vein(32, 0, 128, 32, 32);
    }

    fn generate_ore_vein(&mut self, count: i32, min_y: i32, max_y: i32, size: i32, block_id: u8) {
        for _ in 0..count {
            let x = self.xo + self.random.next_int_bound(16);
            let y = self.random.next_int_bound(max_y - min_y) + min_y;
            let z = self.zo + self.random.next_int_bound(16);
            self.generate_ore_vein_at(x, y, z, size, block_id);
        }
    }

    fn generate_ore_vein_at(&mut self, x: i32, y: i32, z: i32, size: i32, block_id: u8) {
        let mut cx = x;
        let mut cy = y;
        let mut cz = z;
        for _ in 0..size {
            let idx = ((cz - self.zo) * 16 + (cx - self.xo)) * 128 + cy;
            if idx >= 0 && idx < self.blocks.len() as i32 {
                let current = self.blocks[idx as usize];
                if current == 1 {
                    // stone
                    self.blocks[idx as usize] = block_id;
                }
            }
            cx += self.random.next_int_bound(3) - 1;
            cy += self.random.next_int_bound(3) - 1;
            cz += self.random.next_int_bound(3) - 1;
        }
    }

    // Feature placement methods
    fn place_sand(&mut self, x: i32, y: i32, z: i32) {
        if y < 127 {
            self.set_block(x, y + 1, z, 12); // sand
        }
    }

    fn place_clay(&mut self, x: i32, y: i32, z: i32) {
        if y < 127 {
            self.set_block(x, y + 1, z, 82); // clay
        }
    }

    fn place_gravel(&mut self, x: i32, y: i32, z: i32) {
        if y < 127 {
            self.set_block(x, y + 1, z, 13); // gravel
        }
    }

    fn place_tree(&mut self, x: i32, y: i32, z: i32) {
        // Place a simple oak tree
        let trunk_height = 4 + self.random.next_int_bound(3);
        for i in 0..trunk_height {
            self.set_block(x, y + i, z, 17); // log
        }
        // Leaves
        for dy in 0..=2 {
            for dx in -2..=2 {
                for dz in -2..=2 {
                    let dist = dx * dx + dz * dz;
                    if dist <= 4 {
                        self.set_block(x + dx, y + trunk_height + dy - 2, z + dz, 18); // leaves
                    }
                }
            }
        }
    }

    fn place_huge_mushroom(&mut self, x: i32, y: i32, z: i32) {
        let h = 5 + self.random.next_int_bound(3);
        for i in 0..h {
            self.set_block(x, y + i, z, 99); // mushroom stem
        }
        for dy in 0..=2 {
            for dx in -2..=2 {
                for dz in -2..=2 {
                    let dist = dx * dx + dz * dz;
                    if dist <= 4 {
                        self.set_block(x + dx, y + h + dy - 1, z + dz, 100); // mushroom cap
                    }
                }
            }
        }
    }

    fn place_yellow_flower(&mut self, x: i32, y: i32, z: i32) {
        if self.can_place_flower(x, y, z) {
            self.set_block(x, y, z, 37); // yellow flower
        }
    }

    fn place_rose_flower(&mut self, x: i32, y: i32, z: i32) {
        if self.can_place_flower(x, y, z) {
            self.set_block(x, y, z, 38); // rose
        }
    }

    fn place_tall_grass(&mut self, x: i32, y: i32, z: i32) {
        if self.can_place_tall_grass(x, y, z) {
            self.set_block(x, y, z, 31); // tall grass
        }
    }

    fn place_dead_bush(&mut self, x: i32, y: i32, z: i32) {
        if y > 0 && y < 127 {
            let below = self.get_block(x, y - 1, z);
            if below == 12 {
                // sand
                self.set_block(x, y, z, 32); // dead bush
            }
        }
    }

    fn place_waterlily(&mut self, x: i32, y: i32, z: i32) {
        if y > 0 && y < 127 {
            let below = self.get_block(x, y - 1, z);
            if below == 8 || below == 9 {
                // water
                self.set_block(x, y, z, 111); // waterlily
            }
        }
    }

    fn place_brown_mushroom(&mut self, x: i32, y: i32, z: i32) {
        if y > 0 && y < 127 {
            let light = self.get_block(x, y, z); // simplified light check
            if light == 0 {
                self.set_block(x, y, z, 40); // brown mushroom
            }
        }
    }

    fn place_red_mushroom(&mut self, x: i32, y: i32, z: i32) {
        if y > 0 && y < 127 {
            self.set_block(x, y, z, 40); // red mushroom (same ID for now)
        }
    }

    fn place_reeds(&mut self, x: i32, y: i32, z: i32) {
        // Reeds need water adjacent
        for dy in 0..3 {
            let ny = y + dy;
            if ny < 127 {
                let below = self.get_block(x, ny - 1, z);
                let adj_water = self.get_block(x + 1, ny, z) == 8
                    || self.get_block(x - 1, ny, z) == 8
                    || self.get_block(x, ny, z + 1) == 8
                    || self.get_block(x, ny, z - 1) == 8;
                if (below == 2 || below == 3) && adj_water {
                    self.set_block(x, ny, z, 83); // reeds
                }
            }
        }
    }

    fn place_pumpkin(&mut self, x: i32, y: i32, z: i32) {
        if self.can_place_flower(x, y, z) {
            self.set_block(x, y, z, 86); // pumpkin
        }
    }

    fn place_cactus(&mut self, x: i32, y: i32, z: i32) {
        if y < 126 {
            let below = self.get_block(x, y - 1, z);
            if below == 12 {
                // sand
                let h = 1 + self.random.next_int_bound(3);
                for i in 0..h {
                    self.set_block(x, y + i, z, 81); // cactus
                }
            }
        }
    }

    fn place_water_spring(&mut self, x: i32, y: i32, z: i32) {
        if y > 0 && y < 127 {
            let block = self.get_block(x, y, z);
            if block == 0 {
                let below = self.get_block(x, y - 1, z);
                if below != 0 {
                    self.set_block(x, y, z, 8); // water
                }
            }
        }
    }

    fn place_lava_spring(&mut self, x: i32, y: i32, z: i32) {
        if y > 0 && y < 127 {
            let block = self.get_block(x, y, z);
            if block == 0 {
                let below = self.get_block(x, y - 1, z);
                if below != 0 {
                    self.set_block(x, y, z, 10); // lava
                }
            }
        }
    }
}

/// Decorate a chunk with biome-specific features
pub fn decorate_chunk(ctx: &mut DecorationContext, blocks: &mut [u8]) {
    let biome = crate::biome::get_biome(ctx.biome).expect("biome exists");
    let mut decorator =
        BiomeDecorator::new(biome, &mut ctx.random, ctx.chunk_x, ctx.chunk_z, blocks);
    decorator.decorate();
}
