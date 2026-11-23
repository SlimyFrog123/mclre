//! Biome definitions matching LCE reference

use crate::layer::{GeneratorType, Layer};
use mclre_core::GameRandom;

/// Biome IDs matching LCE reference
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BiomeId {
    Ocean = 0,
    Plains = 1,
    Desert = 2,
    ExtremeHills = 3,
    Forest = 4,
    Taiga = 5,
    Swampland = 6,
    River = 7,
    Hell = 8,
    Sky = 9,
    FrozenOcean = 10,
    FrozenRiver = 11,
    IcePlains = 12,
    IceMountains = 13,
    MushroomIsland = 14,
    MushroomIslandShore = 15,
    Beach = 16,
    DesertHills = 17,
    ForestHills = 18,
    TaigaHills = 19,
    SmallExtremeHills = 20,
    Jungle = 21,
    JungleHills = 22,
    #[doc(hidden)]
    BiomeCount = 23,
}

impl BiomeId {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
    pub fn from_u8(id: u8) -> Option<Self> {
        if id < BiomeId::BiomeCount as u8 {
            // Safe because we validate id < BiomeCount and BiomeId is repr(u8)
            Some(match id {
                0 => BiomeId::Ocean,
                1 => BiomeId::Plains,
                2 => BiomeId::Desert,
                3 => BiomeId::ExtremeHills,
                4 => BiomeId::Forest,
                5 => BiomeId::Taiga,
                6 => BiomeId::Swampland,
                7 => BiomeId::River,
                8 => BiomeId::Hell,
                9 => BiomeId::Sky,
                10 => BiomeId::FrozenOcean,
                11 => BiomeId::FrozenRiver,
                12 => BiomeId::IcePlains,
                13 => BiomeId::IceMountains,
                14 => BiomeId::MushroomIsland,
                15 => BiomeId::MushroomIslandShore,
                16 => BiomeId::Beach,
                17 => BiomeId::DesertHills,
                18 => BiomeId::ForestHills,
                19 => BiomeId::TaigaHills,
                20 => BiomeId::SmallExtremeHills,
                21 => BiomeId::Jungle,
                22 => BiomeId::JungleHills,
                _ => unreachable!(),
            })
        } else {
            None
        }
    }
    pub fn is_oceanic(self) -> bool {
        matches!(
            self,
            BiomeId::Ocean | BiomeId::FrozenOcean | BiomeId::River | BiomeId::FrozenRiver
        )
    }
}

impl std::fmt::Display for BiomeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Biome properties
#[derive(Debug, Clone, Copy)]
pub struct Biome {
    pub id: BiomeId,
    pub name: &'static str,
    pub temperature: f32,
    pub downfall: f32,
    pub depth: f32,
    pub scale: f32,
    pub top_material: u8,
    pub filler_material: u8,
    pub min_height: f32,
    pub max_height: f32,
    pub color: u32,
    // Decoration counts (matching LCE BiomeDecorator)
    pub tree_count: i32,
    pub flower_count: i32,
    pub grass_count: i32,
    pub dead_bush_count: i32,
    pub mushroom_count: i32,
    pub reeds_count: i32,
    pub cactus_count: i32,
    pub sand_count: i32,
    pub clay_count: i32,
    pub gravel_count: i32,
    pub waterlily_count: i32,
    pub huge_mushrooms: i32,
    pub liquids: bool,
}

impl Biome {
    pub fn new(id: BiomeId, name: &'static str) -> Self {
        Self {
            id,
            name,
            temperature: 0.5,
            downfall: 0.5,
            depth: 0.1,
            scale: 0.2,
            top_material: 2,
            filler_material: 3,
            min_height: 0.0,
            max_height: 0.0,
            color: 0xFFFFFF,
            // Default decoration counts (matching BiomeDecorator::_init)
            tree_count: 0,
            flower_count: 2,
            grass_count: 1,
            dead_bush_count: 0,
            mushroom_count: 0,
            reeds_count: 0,
            cactus_count: 0,
            sand_count: 3,
            clay_count: 1,
            gravel_count: 1,
            waterlily_count: 0,
            huge_mushrooms: 0,
            liquids: true,
        }
    }
    pub fn get_temperature(&self) -> f32 {
        self.temperature
    }
    pub fn get_downfall(&self) -> f32 {
        self.downfall
    }
    pub fn get_downfall_int(&self) -> i32 {
        (self.downfall * 65536.0) as i32
    }
    pub fn get_temperature_int(&self) -> i32 {
        (self.temperature * 65536.0) as i32
    }
}

/// Biome registry
struct BiomeRegistry {
    biomes: [Option<Biome>; BiomeId::BiomeCount as usize],
}

impl BiomeRegistry {
    fn new() -> Self {
        let mut r = Self {
            biomes: std::array::from_fn(|_| None),
        };
        r.register_all();
        r
    }
    fn register(&mut self, biome: Biome) {
        self.biomes[biome.id as usize] = Some(biome);
    }
    fn get(&self, id: BiomeId) -> Option<&Biome> {
        self.biomes[id as usize].as_ref()
    }
    fn register_all(&mut self) {
        // Register all 23 biomes from LCE reference
        let mut register = |id,
                            name,
                            temp,
                            downfall,
                            depth,
                            scale,
                            top,
                            filler,
                            color,
                            tree_count,
                            flower_count,
                            grass_count,
                            dead_bush_count,
                            mushroom_count,
                            reeds_count,
                            cactus_count,
                            sand_count,
                            clay_count,
                            gravel_count,
                            waterlily_count,
                            huge_mushrooms,
                            liquids| {
            let mut b = Biome::new(id, name);
            b.temperature = temp;
            b.downfall = downfall;
            b.depth = depth;
            b.scale = scale;
            b.top_material = top;
            b.filler_material = filler;
            b.color = color;
            b.tree_count = tree_count;
            b.flower_count = flower_count;
            b.grass_count = grass_count;
            b.dead_bush_count = dead_bush_count;
            b.mushroom_count = mushroom_count;
            b.reeds_count = reeds_count;
            b.cactus_count = cactus_count;
            b.sand_count = sand_count;
            b.clay_count = clay_count;
            b.gravel_count = gravel_count;
            b.waterlily_count = waterlily_count;
            b.huge_mushrooms = huge_mushrooms;
            b.liquids = liquids;
            self.register(b);
        };

        // Default counts (from BiomeDecorator::_init): tree=0, flower=2, grass=1, dead_bush=0, mushroom=0,
        // reeds=0, cactus=0, sand=3, clay=1, gravel=1, waterlily=0, huge_mushrooms=0, liquids=true
        let default = (0, 2, 1, 0, 0, 0, 0, 3, 1, 1, 0, 0, true);

        // Ocean - default
        register(
            BiomeId::Ocean,
            "Ocean",
            0.5,
            0.5,
            1.0,
            0.1,
            0,
            0,
            0x000070,
            default.0,
            default.1,
            default.2,
            default.3,
            default.4,
            default.5,
            default.6,
            default.7,
            default.8,
            default.9,
            default.10,
            default.11,
            default.12,
        );
        // Plains - tree=-999, flower=4, grass=10
        register(
            BiomeId::Plains,
            "Plains",
            0.8,
            0.4,
            0.125,
            0.05,
            2,
            3,
            0x80C040,
            -999,
            4,
            10,
            0,
            0,
            0,
            0,
            3,
            1,
            1,
            0,
            0,
            true,
        );
        // Desert - tree=-999, dead_bush=2, reeds=50, cactus=10
        register(
            BiomeId::Desert,
            "Desert",
            2.0,
            0.0,
            0.125,
            0.05,
            12,
            12,
            0xF0E050,
            -999,
            2,
            1,
            2,
            0,
            50,
            10,
            3,
            1,
            1,
            0,
            0,
            true,
        );
        // ExtremeHills - default
        register(
            BiomeId::ExtremeHills,
            "ExtremeHills",
            0.2,
            0.3,
            1.0,
            0.5,
            1,
            1,
            0x606060,
            default.0,
            default.1,
            default.2,
            default.3,
            default.4,
            default.5,
            default.6,
            default.7,
            default.8,
            default.9,
            default.10,
            default.11,
            default.12,
        );
        // Forest - tree=10, grass=2
        register(
            BiomeId::Forest,
            "Forest",
            0.7,
            0.8,
            0.125,
            0.05,
            2,
            3,
            0x208020,
            10,
            2,
            2,
            0,
            0,
            0,
            0,
            3,
            1,
            1,
            0,
            0,
            true,
        );
        // Taiga - tree=10, grass=1
        register(
            BiomeId::Taiga,
            "Taiga",
            0.05,
            0.4,
            0.125,
            0.05,
            2,
            3,
            0x306040,
            10,
            2,
            1,
            0,
            0,
            0,
            0,
            3,
            1,
            1,
            0,
            0,
            true,
        );
        // Swampland - tree=2, flower=-999, dead_bush=1, mushroom=8, reeds=10, clay=1, waterlily=4
        register(
            BiomeId::Swampland,
            "Swampland",
            0.8,
            0.9,
            -0.2,
            0.1,
            2,
            3,
            0x407040,
            2,
            -999,
            1,
            1,
            8,
            10,
            0,
            3,
            1,
            1,
            4,
            0,
            true,
        );
        // River - default
        register(
            BiomeId::River,
            "River",
            0.5,
            0.5,
            -0.5,
            0.0,
            2,
            12,
            0x3050A0,
            default.0,
            default.1,
            default.2,
            default.3,
            default.4,
            default.5,
            default.6,
            default.7,
            default.8,
            default.9,
            default.10,
            default.11,
            default.12,
        );
        // Hell - default (no surface decoration)
        register(
            BiomeId::Hell,
            "Hell",
            2.0,
            0.0,
            0.1,
            0.0,
            87,
            87,
            0xFF0000,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            false,
        );
        // Sky - default (no surface decoration)
        register(
            BiomeId::Sky,
            "Sky",
            0.5,
            0.0,
            0.0,
            0.0,
            0,
            0,
            0x8080FF,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            false,
        );
        // FrozenOcean - default
        register(
            BiomeId::FrozenOcean,
            "FrozenOcean",
            0.0,
            0.5,
            1.0,
            0.1,
            79,
            79,
            0x9090A0,
            default.0,
            default.1,
            default.2,
            default.3,
            default.4,
            default.5,
            default.6,
            default.7,
            default.8,
            default.9,
            default.10,
            default.11,
            default.12,
        );
        // FrozenRiver - default
        register(
            BiomeId::FrozenRiver,
            "FrozenRiver",
            0.0,
            0.5,
            -0.5,
            0.0,
            79,
            12,
            0xA0A0B0,
            default.0,
            default.1,
            default.2,
            default.3,
            default.4,
            default.5,
            default.6,
            default.7,
            default.8,
            default.9,
            default.10,
            default.11,
            default.12,
        );
        // IcePlains - default
        register(
            BiomeId::IcePlains,
            "IcePlains",
            0.0,
            0.5,
            0.125,
            0.05,
            78,
            3,
            0xFFFFFF,
            default.0,
            default.1,
            default.2,
            default.3,
            default.4,
            default.5,
            default.6,
            default.7,
            default.8,
            default.9,
            default.10,
            default.11,
            default.12,
        );
        // IceMountains - default
        register(
            BiomeId::IceMountains,
            "IceMountains",
            0.0,
            0.5,
            1.0,
            0.5,
            78,
            1,
            0xE0E0F0,
            default.0,
            default.1,
            default.2,
            default.3,
            default.4,
            default.5,
            default.6,
            default.7,
            default.8,
            default.9,
            default.10,
            default.11,
            default.12,
        );
        // MushroomIsland - tree=-100, flower=-100, grass=-100, mushroom=1, huge_mushrooms=1
        register(
            BiomeId::MushroomIsland,
            "MushroomIsland",
            0.9,
            1.0,
            0.2,
            0.3,
            110,
            3,
            0xFF00FF,
            -100,
            -100,
            -100,
            0,
            1,
            0,
            0,
            3,
            1,
            1,
            0,
            1,
            true,
        );
        // MushroomIslandShore - tree=-100, flower=-100, grass=-100, mushroom=1, huge_mushrooms=1
        register(
            BiomeId::MushroomIslandShore,
            "MushroomIslandShore",
            0.9,
            1.0,
            -0.1,
            0.1,
            110,
            3,
            0xCC00CC,
            -100,
            -100,
            -100,
            0,
            1,
            0,
            0,
            3,
            1,
            1,
            0,
            1,
            true,
        );
        // Beach - tree=-999
        register(
            BiomeId::Beach,
            "Beach",
            0.8,
            0.4,
            0.0,
            0.025,
            12,
            12,
            0xF0E050,
            -999,
            2,
            1,
            0,
            0,
            0,
            0,
            3,
            1,
            1,
            0,
            0,
            true,
        );
        // DesertHills - same as Desert
        register(
            BiomeId::DesertHills,
            "DesertHills",
            2.0,
            0.0,
            0.45,
            0.3,
            12,
            12,
            0xD0C030,
            -999,
            2,
            1,
            2,
            0,
            50,
            10,
            3,
            1,
            1,
            0,
            0,
            true,
        );
        // ForestHills - same as Forest
        register(
            BiomeId::ForestHills,
            "ForestHills",
            0.7,
            0.8,
            0.45,
            0.3,
            2,
            3,
            0x107010,
            10,
            2,
            2,
            0,
            0,
            0,
            0,
            3,
            1,
            1,
            0,
            0,
            true,
        );
        // TaigaHills - same as Taiga
        register(
            BiomeId::TaigaHills,
            "TaigaHills",
            0.05,
            0.4,
            0.45,
            0.3,
            2,
            3,
            0x205030,
            10,
            2,
            1,
            0,
            0,
            0,
            0,
            3,
            1,
            1,
            0,
            0,
            true,
        );
        // SmallExtremeHills - default
        register(
            BiomeId::SmallExtremeHills,
            "SmallExtremeHills",
            0.2,
            0.3,
            0.5,
            0.3,
            1,
            1,
            0x505050,
            default.0,
            default.1,
            default.2,
            default.3,
            default.4,
            default.5,
            default.6,
            default.7,
            default.8,
            default.9,
            default.10,
            default.11,
            default.12,
        );
        // Jungle - tree=50, grass=25, flower=4
        register(
            BiomeId::Jungle,
            "Jungle",
            0.95,
            0.9,
            0.125,
            0.05,
            2,
            3,
            0x008000,
            50,
            4,
            25,
            0,
            0,
            0,
            0,
            3,
            1,
            1,
            0,
            0,
            true,
        );
        // JungleHills - same as Jungle
        register(
            BiomeId::JungleHills,
            "JungleHills",
            0.95,
            0.9,
            0.45,
            0.3,
            2,
            3,
            0x006000,
            50,
            4,
            25,
            0,
            0,
            0,
            0,
            3,
            1,
            1,
            0,
            0,
            true,
        );
    }
}

// Use OnceLock for thread-safe lazy initialization (no unsafe needed)
static BIOME_REGISTRY: std::sync::OnceLock<BiomeRegistry> = std::sync::OnceLock::new();

fn get_registry() -> &'static BiomeRegistry {
    BIOME_REGISTRY.get_or_init(|| BiomeRegistry::new())
}

pub fn init_biomes() {
    get_registry();
}

pub fn get_biome(id: BiomeId) -> Option<&'static Biome> {
    get_registry().get(id)
}

/// Biome source for generating biome maps
pub struct BiomeSource {
    chunk_layer: crate::layer::LayerEnum,
    biome_layer: crate::layer::LayerEnum,
    zoomed_layer: crate::layer::LayerEnum,
}

impl BiomeSource {
    pub fn new(seed: i64, generator_type: GeneratorType) -> Self {
        let (chunk, biome, zoomed) = crate::layer::get_default_layers(seed, generator_type);
        Self {
            chunk_layer: chunk,
            biome_layer: biome,
            zoomed_layer: zoomed,
        }
    }
    /// Get biome at block coordinates (zoomed layer works at block resolution)
    pub fn get_biome(&self, x: i32, z: i32) -> BiomeId {
        let mut zoomed = self.zoomed_layer.clone();
        let result = zoomed.get_area(x, z, 1, 1);
        BiomeId::from_u8(result[0] as u8).unwrap_or(BiomeId::Ocean)
    }
    /// Get biome block at block coordinates (zoomed layer works at block resolution)
    pub fn get_biome_block(&self, x: i32, z: i32, w: i32, h: i32) -> Vec<BiomeId> {
        let mut zoomed = self.zoomed_layer.clone();
        let result = zoomed.get_area(x, z, w, h);
        result
            .iter()
            .map(|&v| BiomeId::from_u8(v as u8).unwrap_or(BiomeId::Ocean))
            .collect()
    }
    /// Get raw biome block at block coordinates (biome layer works at chunk resolution)
    pub fn get_raw_biome_block(&self, x: i32, z: i32, w: i32, h: i32) -> Vec<BiomeId> {
        let mut biome = self.biome_layer.clone();
        let result = biome.get_area(x >> 4, z >> 4, (w + 15) >> 4, (h + 15) >> 4);
        result
            .iter()
            .map(|&v| BiomeId::from_u8(v as u8).unwrap_or(BiomeId::Ocean))
            .collect()
    }
    /// Get raw biome chunk at chunk coordinates (chunk layer works at chunk resolution)
    pub fn get_raw_biome_chunk(&self, chunk_x: i32, chunk_z: i32, w: i32, h: i32) -> Vec<BiomeId> {
        let mut chunk = self.chunk_layer.clone();
        let result = chunk.get_area(chunk_x, chunk_z, w, h);
        result
            .iter()
            .map(|&v| BiomeId::from_u8(v as u8).unwrap_or(BiomeId::Ocean))
            .collect()
    }
}
