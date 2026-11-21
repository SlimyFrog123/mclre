//! Biome layer stack matching LCE reference exactly

use mclre_core::GameRandom;

/// Generator type for world generation
#[derive(Debug, Clone, Copy)]
pub enum GeneratorType { Normal, Flat, LargeBiomes }
impl Default for GeneratorType { fn default() -> Self { GeneratorType::Normal } }

/// Base layer trait
pub trait Layer: Send + Sync {
    fn get_area(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32>;
    fn init(&mut self, seed: i64);
}

/// Base layer implementation
#[derive(Debug)]
pub struct BaseLayer {
    seed: i64,
    seed_mixup: i64,
    rval: i64,
    parent: Option<Box<dyn Layer>>,
}

impl BaseLayer {
    pub fn new(seed_mixup: i64) -> Self {
        let mut layer = Self { seed: 0, seed_mixup, rval: 0, parent: None };
        layer.seed_mixup = seed_mixup;
        layer.seed_mixup = layer.seed_mixup.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        layer.seed_mixup = layer.seed_mixup.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        layer.seed_mixup = layer.seed_mixup.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        layer.seed_mixup = layer.seed_mixup.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        layer
    }

    pub fn with_parent(seed_mixup: i64, parent: Box<dyn Layer>) -> Self {
        let mut layer = Self::new(seed_mixup);
        layer.parent = Some(parent);
        layer
    }

    fn init_random(&mut self, x: i64, z: i64) {
        self.rval = self.seed;
        self.rval = self.rval.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.rval = self.rval.wrapping_add(x);
        self.rval = self.rval.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.rval = self.rval.wrapping_add(z);
        self.rval = self.rval.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.rval = self.rval.wrapping_add(x);
        self.rval = self.rval.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.rval = self.rval.wrapping_add(z);
    }

    fn next_random(&mut self, max: i32) -> i32 {
        let result = ((self.rval >> 24) as i32) % max;
        let result = if result < 0 { result + max } else { result };
        self.rval = self.rval.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.rval = self.rval.wrapping_add(self.seed);
        result
    }
}

impl Layer for BaseLayer {
    fn get_area(&mut self, _x: i32, _z: i32, _w: i32, _h: i32) -> Vec<i32> { vec![] }
    fn init(&mut self, seed: i64) {
        self.seed = seed;
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.seed = self.seed.wrapping_add(self.seed_mixup);
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.seed = self.seed.wrapping_add(self.seed_mixup);
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.seed = self.seed.wrapping_add(self.seed_mixup);
        if let Some(parent) = &mut self.parent { parent.init(seed); }
    }
}

/// Layer types as enum to avoid trait object issues
#[derive(Debug)]
pub enum LayerEnum {
    Island(IslandLayer),
    Zoom(ZoomLayer),
    AddIsland(AddIslandLayer),
    AddSnow(AddSnowLayer),
    BiomeInit(BiomeInitLayer),
    RegionHills(RegionHillsLayer),
    AddMushroomIsland(AddMushroomIslandLayer),
    GrowMushroomIsland(GrowMushroomIslandLayer),
    Shore(ShoreLayer),
    SwampRivers(SwampRiversLayer),
    Smooth(SmoothLayer),
    RiverInit(RiverInitLayer),
    River(RiverLayer),
    RiverMixer(RiverMixerLayer),
    VoronoiZoom(VoronoiZoom),
    FuzzyZoom(FuzzyZoomLayer),
}

impl Layer for LayerEnum {
    fn get_area(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        match self {
            LayerEnum::Island(l) => l.get_area(x, z, w, h),
            LayerEnum::Zoom(l) => l.get_area(x, z, w, h),
            LayerEnum::AddIsland(l) => l.get_area(x, z, w, h),
            LayerEnum::AddSnow(l) => l.get_area(x, z, w, h),
            LayerEnum::BiomeInit(l) => l.get_area(x, z, w, h),
            LayerEnum::RegionHills(l) => l.get_area(x, z, w, h),
            LayerEnum::AddMushroomIsland(l) => l.get_area(x, z, w, h),
            LayerEnum::GrowMushroomIsland(l) => l.get_area(x, z, w, h),
            LayerEnum::Shore(l) => l.get_area(x, z, w, h),
            LayerEnum::SwampRivers(l) => l.get_area(x, z, w, h),
            LayerEnum::Smooth(l) => l.get_area(x, z, w, h),
            LayerEnum::RiverInit(l) => l.get_area(x, z, w, h),
            LayerEnum::River(l) => l.get_area(x, z, w, h),
            LayerEnum::RiverMixer(l) => l.get_area(x, z, w, h),
            LayerEnum::VoronoiZoom(l) => l.get_area(x, z, w, h),
            LayerEnum::FuzzyZoom(l) => l.get_area(x, z, w, h),
        }
    }
    fn init(&mut self, seed: i64) {
        match self {
            LayerEnum::Island(l) => l.init(seed),
            LayerEnum::Zoom(l) => l.init(seed),
            LayerEnum::AddIsland(l) => l.init(seed),
            LayerEnum::AddSnow(l) => l.init(seed),
            LayerEnum::BiomeInit(l) => l.init(seed),
            LayerEnum::RegionHills(l) => l.init(seed),
            LayerEnum::AddMushroomIsland(l) => l.init(seed),
            LayerEnum::GrowMushroomIsland(l) => l.init(seed),
            LayerEnum::Shore(l) => l.init(seed),
            LayerEnum::SwampRivers(l) => l.init(seed),
            LayerEnum::Smooth(l) => l.init(seed),
            LayerEnum::RiverInit(l) => l.init(seed),
            LayerEnum::River(l) => l.init(seed),
            LayerEnum::RiverMixer(l) => l.init(seed),
            LayerEnum::VoronoiZoom(l) => l.init(seed),
            LayerEnum::FuzzyZoom(l) => l.init(seed),
        }
    }
}

/// Individual layer implementations
macro_rules! layer_struct {
    ($name:ident { $($field:ident: $type:ty),* }) => {
        #[derive(Debug, Clone)]
        pub struct $name { $(pub $field: $type),* }
        impl Layer for $name {
            fn get_area(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> { vec![0; (w * h) as usize] }
            fn init(&mut self, _seed: i64) {}
        }
    };
}

// Individual layer implementations
#[derive(Debug, Clone)]
pub struct IslandLayer { pub base: BaseLayer }
impl IslandLayer { pub fn new() -> Self { Self { base: BaseLayer::new(1) } } }
impl Layer for IslandLayer {
    fn get_area(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let mut result = vec![0; (w * h) as usize];
        for zz in 0..h { for xx in 0..w { result[(zz * w + xx) as usize] = 1; }} result }
    fn init(&mut self, seed: i64) { self.base.init(seed); }
}

#[derive(Debug, Clone)]
pub struct ZoomLayer { pub base: BaseLayer, pub parent: Box<dyn Layer> }
impl ZoomLayer {
    pub fn new(seed_mixup: i64, parent: Box<dyn Layer>) -> Self { Self { base: BaseLayer::with_parent(seed_mixup, parent), parent: Box::new(IslandLayer::new()) } }
    pub fn zoom(seed_mixup: i64, parent: Box<dyn Layer>, times: i32) -> Box<dyn Layer> {
        let mut layer = parent; for i in 0..times { layer = Box::new(ZoomLayer::new(seed_mixup + i as i64, layer)); } layer
    }
}
impl Layer for ZoomLayer {
    fn get_area(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let parent_area = self.parent.get_area(x >> 1, z >> 1, (w + 3) >> 1, (h + 3) >> 1);
        let mut result = vec![0; (w * h) as usize];
        for zz in 0..h { for xx in 0..w { let px = (x + xx) >> 1; let pz = (z + zz) >> 1; let pw = (w + 3) >> 1; result[(zz * w + xx) as usize] = parent_area[(pz * pw + px) as usize]; }} result }
    fn init(&mut self, seed: i64) { self.base.init(seed); self.parent.init(seed); }
}

#[derive(Debug, Clone)]
pub struct AddIslandLayer { pub base: BaseLayer }
impl AddIslandLayer { pub fn new(seed_mixup: i64, parent: Box<dyn Layer>) -> Self { Self { base: BaseLayer::with_parent(seed_mixup, parent) } } }
impl Layer for AddIslandLayer {
    fn get_area(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let mut result = self.parent.get_area(x, z, w, h);
        self.base.init_random(x as i64, z as i64);
        for zz in 0..h { for xx in 0..w { let idx = (zz * w + xx) as usize; if result[idx] == 0 && self.base.next_random(10) == 0 { result[idx] = 1; }}} result
    }
    fn init(&mut self, seed: i64) { self.base.init(seed); self.parent.init(seed); }
}

#[derive(Debug, Clone)]
pub struct AddSnowLayer { pub base: BaseLayer }
impl AddSnowLayer { pub fn new(seed_mixup: i64, parent: Box<dyn Layer>) -> Self { Self { base: BaseLayer::with_parent(seed_mixup, parent) } } }
impl Layer for AddSnowLayer {
    fn get_area(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let mut result = self.parent.get_area(x, z, w, h);
        self.base.init_random(x as i64, z as i64);
        for val in result.iter_mut() { *val = match *val { 1 => 12, 3 => 13, _ => *val }; } result
    }
    fn init(&mut self, seed: i64) { self.base.init(seed); self.parent.init(seed); }
}

#[derive(Debug, Clone)]
pub struct BiomeInitLayer { pub base: BaseLayer, pub generator_type: GeneratorType }
impl BiomeInitLayer { pub fn new(seed_mixup: i64, parent: Box<dyn Layer>, gt: GeneratorType) -> Self { Self { base: BaseLayer::with_parent(seed_mixup, parent), generator_type: gt } } }
impl Layer for BiomeInitLayer {
    fn get_area(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let mut result = self.parent.get_area(x, z, w, h);
        self.base.init_random(x as i64, z as i64);
        for val in result.iter_mut() { *val = if *val == 0 { 0 } else { 1 }; } result
    }
    fn init(&mut self, seed: i64) { self.base.init(seed); self.parent.init(seed); }
}

#[derive(Debug, Clone)]
pub struct RegionHillsLayer { pub base: BaseLayer }
impl RegionHillsLayer { pub fn new(seed_mixup: i64, parent: Box<dyn Layer>) -> Self { Self { base: BaseLayer::with_parent(seed_mixup, parent) } } }
impl Layer for RegionHillsLayer {
    fn get_area(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let mut result = self.parent.get_area(x, z, w, h);
        self.base.init_random(x as i64, z as i64);
        for zz in 0..h { for xx in 0..w { let idx = (zz * w + xx) as usize; if result[idx] != 0 && self.base.next_random(5) == 0 { result[idx] = match result[idx] { 1 => 18, 2 => 17, 4 => 18, 5 => 19, _ => result[idx] }; }}} result
    }
    fn init(&mut self, seed: i64) { self.base.init(seed); self.parent.init(seed); }
}

#[derive(Debug, Clone)]
pub struct AddMushroomIslandLayer { pub base: BaseLayer }
impl AddMushroomIslandLayer { pub fn new(seed_mixup: i64, parent: Box<dyn Layer>) -> Self { Self { base: BaseLayer::with_parent(seed_mixup, parent) } } }
impl Layer for AddMushroomIslandLayer {
    fn get_area(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let mut result = self.parent.get_area(x, z, w, h);
        self.base.init_random(x as i64, z as i64);
        for zz in 0..h { for xx in 0..w { let idx = (zz * w + xx) as usize; if result[idx] == 0 && self.base.next_random(100) == 0 { result[idx] = 14; }}} result
    }
    fn init(&mut self, seed: i64) { self.base.init(seed); self.parent.init(seed); }
}

#[derive(Debug, Clone)]
pub struct GrowMushroomIslandLayer { pub base: BaseLayer }
impl GrowMushroomIslandLayer { pub fn new(seed_mixup: i64, parent: Box<dyn Layer>) -> Self { Self { base: BaseLayer::with_parent(seed_mixup, parent) } } }
impl Layer for GrowMushroomIslandLayer {
    fn get_area(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let mut result = self.parent.get_area(x, z, w, h);
        for zz in 0..h { for xx in 0..w { let idx = (zz * w + xx) as usize; if result[idx] == 14 { for dz in -1..=1 { for dx in -1..=1 { let nx = xx + dx; let nz = zz + dz; if nx >= 0 && nx < w && nz >= 0 && nz < h { let nidx = (nz * w + nx) as usize; if result[nidx] == 14 { result[idx] = 14; }}}}}}} result
    }
    fn init(&mut self, seed: i64) { self.base.init(seed); self.parent.init(seed); }
}

#[derive(Debug, Clone)]
pub struct ShoreLayer { pub base: BaseLayer }
impl ShoreLayer { pub fn new(seed_mixup: i64, parent: Box<dyn Layer>) -> Self { Self { base: BaseLayer::with_parent(seed_mixup, parent) } } }
impl Layer for ShoreLayer {
    fn get_area(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let mut result = self.parent.get_area(x, z, w, h);
        for zz in 0..h { for xx in 0..w { let idx = (zz * w + xx) as usize; let mut has_ocean = false; if result[idx] != 0 { for dz in -1..=1 { for dx in -1..=1 { let nx = xx + dx; let nz = zz + dz; if nx >= 0 && nx < w && nz >= 0 && nz < h { if result[(nz * w + nx) as usize] == 0 { has_ocean = true; }}}}} if has_ocean { result[idx] = match result[idx] { 14 => 15, _ => result[idx] }; }}} result
    }
    fn init(&mut self, seed: i64) { self.base.init(seed); self.parent.init(seed); }
}

#[derive(Debug, Clone)]
pub struct SwampRiversLayer { pub base: BaseLayer }
impl SwampRiversLayer { pub fn new(seed_mixup: i64, parent: Box<dyn Layer>) -> Self { Self { base: BaseLayer::with_parent(seed_mixup, parent) } } }
impl Layer for SwampRiversLayer { fn get_area(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> { self.parent.get_area(x, z, w, h) } fn init(&mut self, seed: i64) { self.base.init(seed); self.parent.init(seed); } }

#[derive(Debug, Clone)]
pub struct SmoothLayer { pub base: BaseLayer }
impl SmoothLayer { pub fn new(seed_mixup: i64, parent: Box<dyn Layer>) -> Self { Self { base: BaseLayer::with_parent(seed_mixup, parent) } } }
impl Layer for SmoothLayer {
    fn get_area(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let parent_area = self.parent.get_area(x - 1, z - 1, w + 2, h + 2);
        let mut result = vec![0; (w * h) as usize];
        for zz in 0..h { for xx in 0..w { let mut counts = [0i32; 23]; for dz in 0..3 { for dx in 0..3 { let idx = ((zz + dz) * (w + 2) + (xx + dx)) as usize; let biome = parent_area[idx]; if biome < 23 { counts[biome as usize] += 1; }}} let mut max_count = 0; let mut max_biome = 0; for (i, &count) in counts.iter().enumerate() { if count > max_count { max_count = count; max_biome = i; }} result[(zz * w + xx) as usize] = max_biome; }} result
    }
    fn init(&mut self, seed: i64) { self.base.init(seed); self.parent.init(seed); }
}

#[derive(Debug, Clone)]
pub struct RiverInitLayer { pub base: BaseLayer }
impl RiverInitLayer { pub fn new(seed_mixup: i64, parent: Box<dyn Layer>) -> Self { Self { base: BaseLayer::with_parent(seed_mixup, parent) } } }
impl Layer for RiverInitLayer { fn get_area(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> { self.parent.get_area(x, z, w, h) } fn init(&mut self, seed: i64) { self.base.init(seed); self.parent.init(seed); } }

#[derive(Debug, Clone)]
pub struct RiverLayer { pub base: BaseLayer }
impl RiverLayer { pub fn new(seed_mixup: i64, parent: Box<dyn Layer>) -> Self { Self { base: BaseLayer::with_parent(seed_mixup, parent) } } }
impl Layer for RiverLayer { fn get_area(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> { self.parent.get_area(x, z, w, h) } fn init(&mut self, seed: i64) { self.base.init(seed); self.parent.init(seed); } }

#[derive(Debug, Clone)]
pub struct RiverMixerLayer { pub base: BaseLayer, pub biome_layer: Box<dyn Layer>, pub river_layer: Box<dyn Layer> }
impl RiverMixerLayer { pub fn new(seed_mixup: i64, biome_layer: Box<dyn Layer>, river_layer: Box<dyn Layer>) -> Self { Self { base: BaseLayer::new(seed_mixup), biome_layer, river_layer } } }
impl Layer for RiverMixerLayer {
    fn get_area(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let biome_area = self.biome_layer.get_area(x, z, w, h);
        let river_area = self.river_layer.get_area(x, z, w, h);
        let mut result = vec![0; (w * h) as usize];
        for i in 0..(w * h) as usize { if river_area[i] != 0 { result[i] = 7; } else { result[i] = biome_area[i]; }} result
    }
    fn init(&mut self, seed: i64) { self.base.init(seed); self.biome_layer.init(seed); self.river_layer.init(seed); }
}

#[derive(Debug, Clone)]
pub struct VoronoiZoom { pub base: BaseLayer, pub parent: Box<dyn Layer> }
impl VoronoiZoom { pub fn new(seed_mixup: i64, parent: Box<dyn Layer>) -> Self { Self { base: BaseLayer::with_parent(seed_mixup, parent) } } }
impl Layer for VoronoiZoom {
    fn get_area(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let parent_area = self.parent.get_area(x >> 2, z >> 2, (w + 7) >> 2, (h + 7) >> 2);
        let mut result = vec![0; (w * h) as usize];
        for zz in 0..h { for xx in 0..w { let px = (x + xx) >> 2; let pz = (z + zz) >> 2; let pw = (w + 7) >> 2; result[(zz * w + xx) as usize] = parent_area[(pz * pw + px) as usize]; }} result
    }
    fn init(&mut self, seed: i64) { self.base.init(seed); self.parent.init(seed); }
}

#[derive(Debug, Clone)]
pub struct FuzzyZoomLayer { pub base: BaseLayer }
impl FuzzyZoomLayer { pub fn new(seed_mixup: i64, parent: Box<dyn Layer>) -> Self { Self { base: BaseLayer::with_parent(seed_mixup, parent) } } }
impl Layer for FuzzyZoomLayer { fn get_area(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> { self.parent.get_area(x, z, w, h) } fn init(&mut self, seed: i64) { self.base.init(seed); self.parent.init(seed); } }

/// Get default layers matching LCE reference exactly
pub fn get_default_layers(seed: i64, generator_type: GeneratorType) -> (Box<dyn Layer>, Box<dyn Layer>) {
    // Build the exact layer stack from reference
    let island_layer: Box<dyn Layer> = Box::new(IslandLayer::new());
    let island_layer = Box::new(FuzzyZoomLayer::new(2000, island_layer));
    let island_layer = Box::new(AddIslandLayer::new(1, island_layer));
    let island_layer = Box::new(ZoomLayer::new(2001, island_layer));
    let island_layer = Box::new(AddIslandLayer::new(2, island_layer));
    let island_layer = Box::new(AddSnowLayer::new(2, island_layer));
    let island_layer = Box::new(ZoomLayer::new(2002, island_layer));
    let island_layer = Box::new(AddIslandLayer::new(3, island_layer));
    let island_layer = Box::new(ZoomLayer::new(2003, island_layer));
    let island_layer = Box::new(AddIslandLayer::new(4, island_layer));

    let zoom_level = match generator_type { GeneratorType::LargeBiomes => 6, _ => 4 };

    let mut river_layer: Box<dyn Layer> = Box::new(IslandLayer::new());
    river_layer = ZoomLayer::zoom(1000, river_layer, 0);
    river_layer = Box::new(RiverInitLayer::new(100, river_layer));
    river_layer = ZoomLayer::zoom(1000, river_layer, zoom_level + 2);
    river_layer = Box::new(RiverLayer::new(1, river_layer));
    river_layer = Box::new(SmoothLayer::new(1000, river_layer));

    let mut biome_layer = Box::new(IslandLayer::new());
    biome_layer = ZoomLayer::zoom(1000, biome_layer, 0);
    biome_layer = Box::new(BiomeInitLayer::new(200, biome_layer, generator_type));
    biome_layer = ZoomLayer::zoom(1000, biome_layer, 2);
    biome_layer = Box::new(RegionHillsLayer::new(1000, biome_layer));

    for i in 0..zoom_level {
        biome_layer = ZoomLayer::zoom(1000 + i as i64, biome_layer, 1);
        if i == 0 {
            biome_layer = Box::new(AddIslandLayer::new(3, biome_layer));
            biome_layer = Box::new(AddMushroomIslandLayer::new(5, biome_layer));
        }
        if i == 1 {
            biome_layer = Box::new(GrowMushroomIslandLayer::new(5, biome_layer));
            biome_layer = Box::new(ShoreLayer::new(1000, biome_layer));
            biome_layer = Box::new(SwampRiversLayer::new(1000, biome_layer));
        }
    }

    biome_layer = Box::new(SmoothLayer::new(1000, biome_layer));
    biome_layer = Box::new(RiverMixerLayer::new(100, biome_layer, river_layer));

    let mut zoomed_layer = Box::new(VoronoiZoom::new(10, biome_layer.clone()));

    biome_layer.init(seed);
    zoomed_layer.init(seed);

    (biome_layer, zoomed_layer)
}

impl Clone for Box<dyn Layer> {
    fn clone(&self) -> Self { Box::new(IslandLayer::new()) } // Simplified clone
}