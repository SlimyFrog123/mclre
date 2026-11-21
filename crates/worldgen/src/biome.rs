//! Biome definitions matching LCE reference

use crate::layer::{Layer, GeneratorType};
use mclre_core::GameRandom;

/// Biome IDs matching LCE reference
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BiomeId {
    Ocean = 0, Plains = 1, Desert = 2, ExtremeHills = 3, Forest = 4, Taiga = 5,
    Swampland = 6, River = 7, Hell = 8, Sky = 9, FrozenOcean = 10, FrozenRiver = 11,
    IcePlains = 12, IceMountains = 13, MushroomIsland = 14, MushroomIslandShore = 15,
    Beach = 16, DesertHills = 17, ForestHills = 18, TaigaHills = 19, SmallExtremeHills = 20,
    Jungle = 21, JungleHills = 22,
    #[doc(hidden)] BiomeCount = 23,
}

impl BiomeId {
    pub fn as_u8(self) -> u8 { self as u8 }
    pub fn from_u8(id: u8) -> Option<Self> { 
        if id < BiomeId::BiomeCount as u8 { 
            // Safe because we validate id < BiomeCount and BiomeId is repr(u8)
            Some(match id {
                0 => BiomeId::Ocean, 1 => BiomeId::Plains, 2 => BiomeId::Desert, 3 => BiomeId::ExtremeHills,
                4 => BiomeId::Forest, 5 => BiomeId::Taiga, 6 => BiomeId::Swampland, 7 => BiomeId::River,
                8 => BiomeId::Hell, 9 => BiomeId::Sky, 10 => BiomeId::FrozenOcean, 11 => BiomeId::FrozenRiver,
                12 => BiomeId::IcePlains, 13 => BiomeId::IceMountains, 14 => BiomeId::MushroomIsland, 15 => BiomeId::MushroomIslandShore,
                16 => BiomeId::Beach, 17 => BiomeId::DesertHills, 18 => BiomeId::ForestHills, 19 => BiomeId::TaigaHills,
                20 => BiomeId::SmallExtremeHills, 21 => BiomeId::Jungle, 22 => BiomeId::JungleHills,
                _ => unreachable!()
            })
        } else { None } 
    }
    pub fn is_oceanic(self) -> bool { matches!(self, BiomeId::Ocean | BiomeId::FrozenOcean | BiomeId::River | BiomeId::FrozenRiver) }
}

/// Biome properties
#[derive(Debug, Clone, Copy)]
pub struct Biome { pub id: BiomeId, pub name: &'static str, pub temperature: f32, pub downfall: f32, pub depth: f32, pub scale: f32, pub top_material: u8, pub filler_material: u8, pub min_height: f32, pub max_height: f32, pub color: u32 }

impl Biome {
    pub fn new(id: BiomeId, name: &'static str) -> Self { Self { id, name, temperature: 0.5, downfall: 0.5, depth: 0.1, scale: 0.2, top_material: 2, filler_material: 3, min_height: 0.0, max_height: 0.0, color: 0xFFFFFF } }
    pub fn get_temperature(&self) -> f32 { self.temperature }
    pub fn get_downfall(&self) -> f32 { self.downfall }
    pub fn get_downfall_int(&self) -> i32 { (self.downfall * 65536.0) as i32 }
    pub fn get_temperature_int(&self) -> i32 { (self.temperature * 65536.0) as i32 }
}

/// Biome registry
struct BiomeRegistry { biomes: [Option<Biome>; BiomeId::BiomeCount as usize] }

impl BiomeRegistry {
    fn new() -> Self { let mut r = Self { biomes: std::array::from_fn(|_| None) }; r.register_all(); r }
    fn register(&mut self, biome: Biome) { self.biomes[biome.id as usize] = Some(biome); }
    fn get(&self, id: BiomeId) -> Option<&Biome> { self.biomes[id as usize].as_ref() }
    fn register_all(&mut self) {
        // Register all 23 biomes from LCE reference
        let mut register = |id, name, temp, downfall, depth, scale, top, filler, color| {
            let mut b = Biome::new(id, name);
            b.temperature = temp; b.downfall = downfall; b.depth = depth; b.scale = scale;
            b.top_material = top; b.filler_material = filler; b.color = color;
            self.register(b);
        };
        register(BiomeId::Ocean, "Ocean", 0.5, 0.5, 1.0, 0.1, 0, 0, 0x000070);
        register(BiomeId::Plains, "Plains", 0.8, 0.4, 0.125, 0.05, 2, 3, 0x80C040);
        register(BiomeId::Desert, "Desert", 2.0, 0.0, 0.125, 0.05, 12, 12, 0xF0E050);
        register(BiomeId::ExtremeHills, "ExtremeHills", 0.2, 0.3, 1.0, 0.5, 1, 1, 0x606060);
        register(BiomeId::Forest, "Forest", 0.7, 0.8, 0.125, 0.05, 2, 3, 0x208020);
        register(BiomeId::Taiga, "Taiga", 0.05, 0.4, 0.125, 0.05, 2, 3, 0x306040);
        register(BiomeId::Swampland, "Swampland", 0.8, 0.9, -0.2, 0.1, 2, 3, 0x407040);
        register(BiomeId::River, "River", 0.5, 0.5, -0.5, 0.0, 2, 12, 0x3050A0);
        register(BiomeId::Hell, "Hell", 2.0, 0.0, 0.1, 0.0, 87, 87, 0xFF0000);
        register(BiomeId::Sky, "Sky", 0.5, 0.0, 0.0, 0.0, 0, 0, 0x8080FF);
        register(BiomeId::FrozenOcean, "FrozenOcean", 0.0, 0.5, 1.0, 0.1, 79, 79, 0x9090A0);
        register(BiomeId::FrozenRiver, "FrozenRiver", 0.0, 0.5, -0.5, 0.0, 79, 12, 0xA0A0B0);
        register(BiomeId::IcePlains, "IcePlains", 0.0, 0.5, 0.125, 0.05, 78, 3, 0xFFFFFF);
        register(BiomeId::IceMountains, "IceMountains", 0.0, 0.5, 1.0, 0.5, 78, 1, 0xE0E0F0);
        register(BiomeId::MushroomIsland, "MushroomIsland", 0.9, 1.0, 0.2, 0.3, 110, 3, 0xFF00FF);
        register(BiomeId::MushroomIslandShore, "MushroomIslandShore", 0.9, 1.0, -0.1, 0.1, 110, 3, 0xCC00CC);
        register(BiomeId::Beach, "Beach", 0.8, 0.4, 0.0, 0.025, 12, 12, 0xF0E050);
        register(BiomeId::DesertHills, "DesertHills", 2.0, 0.0, 0.45, 0.3, 12, 12, 0xD0C030);
        register(BiomeId::ForestHills, "ForestHills", 0.7, 0.8, 0.45, 0.3, 2, 3, 0x107010);
        register(BiomeId::TaigaHills, "TaigaHills", 0.05, 0.4, 0.45, 0.3, 2, 3, 0x205030);
        register(BiomeId::SmallExtremeHills, "SmallExtremeHills", 0.2, 0.3, 0.5, 0.3, 1, 1, 0x505050);
        register(BiomeId::Jungle, "Jungle", 0.95, 0.9, 0.125, 0.05, 2, 3, 0x008000);
        register(BiomeId::JungleHills, "JungleHills", 0.95, 0.9, 0.45, 0.3, 2, 3, 0x006000);
    }
}

// Use OnceLock for thread-safe lazy initialization (no unsafe needed)
static BIOME_REGISTRY: std::sync::OnceLock<BiomeRegistry> = std::sync::OnceLock::new();

fn get_registry() -> &'static BiomeRegistry {
    BIOME_REGISTRY.get_or_init(|| BiomeRegistry::new())
}

pub fn init_biomes() { get_registry(); }

pub fn get_biome(id: BiomeId) -> Option<&'static Biome> {
    get_registry().get(id)
}

/// Biome source for generating biome maps
pub struct BiomeSource { base_layer: Box<dyn Layer>, zoomed_layer: Box<dyn Layer> }

impl BiomeSource {
    pub fn new(seed: i64, generator_type: GeneratorType) -> Self {
        let (base, zoomed) = crate::layer::get_default_layers(seed, generator_type);
        Self { base_layer: base, zoomed_layer: zoomed }
    }
    pub fn get_biome(&self, x: i32, z: i32) -> BiomeId {
        let mut zoomed = self.zoomed_layer.clone();
        let result = zoomed.get_area(x, z, 1, 1);
        BiomeId::from_u8(result[0] as u8).unwrap_or(BiomeId::Ocean)
    }
    pub fn get_biome_block(&self, x: i32, z: i32, w: i32, h: i32) -> Vec<BiomeId> {
        let mut zoomed = self.zoomed_layer.clone();
        let result = zoomed.get_area(x, z, w, h);
        result.iter().map(|&v| BiomeId::from_u8(v as u8).unwrap_or(BiomeId::Ocean)).collect()
    }
    pub fn get_raw_biome_block(&self, x: i32, z: i32, w: i32, h: i32) -> Vec<BiomeId> {
        let mut base = self.base_layer.clone();
        let result = base.get_area(x, z, w, h);
        result.iter().map(|&v| BiomeId::from_u8(v as u8).unwrap_or(BiomeId::Ocean)).collect()
    }
}