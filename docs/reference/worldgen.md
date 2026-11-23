# World Generation Reference Analysis (LCE 141217 / TU19)

> Source: `reference/lce-141217/MinecraftConsoles-141217/Minecraft.World/`

## Overview

This document captures the exact world generation pipeline from the Legacy Console Edition reference source (commit 141217, corresponding to TU19/CU7). The goal is 1:1 seed-compatible recreation.

---

## 1. Biome Layer Stack (`GenLayer` Chain)

The LCE biome generation uses a chain of `GenLayer` subclasses. Each layer transforms the output of its parent. The chain is built in `GenLayer::a(seed, generatorType)`.

### Layer Stack (Bottom → Top)

| Order | Layer Class | Purpose | Seed Mixup |
|-------|-------------|---------|------------|
| 1 | `IslandLayer` | Initial land/ocean separation | 1 |
| 2 | `FuzzyZoom` | Zoom 2× with jitter | 2000 |
| 3 | `AddIslandLayer` | Add small islands | 1 |
| 4 | `ZoomLayer` | Zoom 2× | 2001 |
| 5 | `AddIslandLayer` | Add more islands | 2 |
| 6 | `AddSnowLayer` | Add snow biomes | 2 |
| 7 | `ZoomLayer` | Zoom 2× | 2002 |
| 8 | `AddIslandLayer` | Add islands | 3 |
| 9 | `ZoomLayer` | Zoom 2× | 2003 |
| 10 | `AddIslandLayer` | Add islands | 4 |
| **Branch** | | **Biome generation splits here** | |

### Biome Generation Branch

| Order | Layer Class | Purpose | Seed Mixup |
|-------|-------------|---------|------------|
| 11 | `ZoomLayer` (zoom_level=0) | Zoom 2× | 1000 |
| 12 | `BiomeInitLayer` | Initialize biome IDs | 200 |
| 12 | `ZoomLayer` (zoom_level=1) | Zoom 2× | 1001 |
| 12 | `RegionHillsLayer` | Add hills variants | 1000 |
| 13 | `ZoomLayer` (zoom_level=2) | Zoom 2× | 1002 |
| 13 | `AddIslandLayer` | Add mushroom islands | 3 |
| 13 | `AddMushroomIslandLayer` | Add mushroom islands | 5 |
| 14 | `ZoomLayer` (zoom_level=3) | Zoom 2× | 1003 |
| 14 | `GrowMushroomIslandLayer` | Grow mushroom islands | 5 |
| 14 | `ShoreLayer` | Add beach biomes | 1000 |
| 14 | `SwampRiversLayer` | Add swamp/river biomes | 1000 |
| 15 | `ZoomLayer` (zoom_level=4+) | Zoom 2× per level | 1004+ |
| 16 | `SmoothLayer` | Smooth biome borders | 1000 |
| 17 | `RiverMixerLayer` | Mix rivers with biomes | 100 |

**Generator Types:**
- `Normal`: zoom_level = 4
- `LargeBiomes`: zoom_level = 6
- `Flat`: special handling (no biomes)

### River Generation Branch

| Order | Layer Class | Purpose | Seed Mixup |
|-------|-------------|---------|------------|
| R1 | `IslandLayer` | Base | - |
| R2 | `ZoomLayer` (0) | Zoom 2× | 1000 |
| R3 | `RiverInitLayer` | Initialize rivers | 100 |
| R4 | `ZoomLayer` (zoom_level+2) | Zoom to match biome layer | 1000 |
| R5 | `RiverLayer` | Carve rivers | 1 |
| R6 | `SmoothLayer` | Smooth rivers | 1000 |

### Final Composition

```
biome_layer (after RiverMixer) + river_layer → RiverMixerLayer
zoomed_layer = biome_layer + VoronoiZoom + 4-6 additional zooms
```

---

## 2. Biome Registry (23 Biomes)

| ID | Name | Temperature | Downfall | Depth | Scale | Top Block | Filler Block |
|----|------|-------------|----------|-------|-------|-----------|--------------|
| 0 | Ocean | 0.5 | 0.5 | 1.0 | 0.1 | Water | Sand |
| 1 | Plains | 0.8 | 0.4 | 0.125 | 0.05 | Grass | Dirt |
| 2 | Desert | 2.0 | 0.0 | 0.125 | 0.05 | Sand | Sand |
| 3 | ExtremeHills | 0.2 | 0.3 | 1.0 | 0.5 | Stone | Stone |
| 4 | Forest | 0.7 | 0.8 | 0.125 | 0.05 | Grass | Dirt |
| 5 | Taiga | 0.05 | 0.4 | 0.125 | 0.05 | Grass | Dirt |
| 6 | Swampland | 0.8 | 0.9 | -0.2 | 0.1 | Grass | Dirt |
| 7 | River | 0.5 | 0.5 | -0.5 | 0.0 | Grass | Dirt |
| 8 | Hell (Nether) | 2.0 | 0.0 | 0.1 | 0.0 | Netherrack | Netherrack |
| 9 | Sky (End) | 0.5 | 0.0 | 0.0 | 0.0 | End Stone | End Stone |
| 10 | FrozenOcean | 0.0 | 0.5 | 1.0 | 0.1 | Ice | Sand |
| 11 | FrozenRiver | 0.0 | 0.5 | -0.5 | 0.0 | Ice | Dirt |
| 12 | IcePlains | 0.0 | 0.5 | 0.125 | 0.05 | Snow | Dirt |
| 13 | IceMountains | 0.0 | 0.5 | 1.0 | 0.5 | Snow | Stone |
| 14 | MushroomIsland | 0.9 | 1.0 | 0.2 | 0.3 | Mycelium | Dirt |
| 15 | MushroomIslandShore | 0.9 | 1.0 | -0.1 | 0.1 | Mycelium | Dirt |
| 16 | Beach | 0.8 | 0.4 | 0.0 | 0.025 | Sand | Sand |
| 17 | DesertHills | 2.0 | 0.0 | 0.45 | 0.3 | Sand | Sand |
| 18 | ForestHills | 0.7 | 0.8 | 0.45 | 0.3 | Grass | Dirt |
| 19 | TaigaHills | 0.05 | 0.4 | 0.45 | 0.3 | Grass | Dirt |
| 20 | SmallExtremeHills | 0.2 | 0.3 | 0.5 | 0.3 | Stone | Stone |
| 21 | Jungle | 0.95 | 0.9 | 0.125 | 0.05 | Grass | Dirt |
| 22 | JungleHills | 0.95 | 0.9 | 0.45 | 0.3 | Grass | Dirt |

**Decoration counts per biome** (from `BiomeDecorator::_init`):
- Default: trees=0, flowers=2, grass=1, dead_bush=0, mushroom=0, reeds=0, cactus=0, sand=3, clay=1, gravel=1, waterlily=0, huge_mushroom=0
- Plains: trees=-999, flowers=4, grass=10
- Desert: trees=-999, dead_bush=2, reeds=50, cactus=10
- Forest: trees=10, flowers=2, grass=2
- Taiga: trees=10, flowers=2, grass=1
- Swampland: trees=2, flowers=-999, dead_bush=1, mushroom=8, reeds=10, clay=1, waterlily=4
- Jungle: trees=50, flowers=4, grass=25
- MushroomIsland: trees=-100, flowers=-100, grass=-100, mushroom=1, huge_mushroom=1

---

## 3. Terrain Generation (`RandomLevelSource`)

### Seed Derivation

```cpp
// Chunk seed
chunkSeed = (chunkX * 341873128712 + chunkZ * 132897987541) ^ worldSeed

// Per-section seed
sectionSeed = (sectionX * 341873128712 + sectionZ * 132897987541) ^ worldSeed
```

### Heightmap Generation (`prepareHeights`)

1. Generate noise using `ImprovedNoise` (Perlin simplex-like)
2. Sample at 4×4 grid per chunk (17×17 points for borders)
3. Apply biome-specific height scaling:
   - Ocean: depth = 0, deep ocean = 1.0
   - Plains/Forest: base 64 + noise × 0.05
   - ExtremeHills: base 64 + noise × 1.0
   - Mountain variants: increased height
4. Blend between biomes at borders

### Surface Building (`buildSurfaces`)

For each column:
1. Determine top block from biome properties
2. Fill down to bedrock (y=0)
3. Replace stone with biome-specific stone variants at depth
3. Add biome-specific surface materials (grass, sand, mycelium, snow, ice)

### Caves (`CaveFeature`)

- Large caves: spiral tunnels with variable thickness
- Uses A* pathfinding for connected cave systems
- Carves through all blocks except bedrock

### Ravines (`CanyonFeature`)

- Long narrow fissures with vertical walls
- Generated with A* pathfinding between two points
- Depth varies, can reach bedrock

---

## 4. Structures

| Structure | Spacing | Min Separation | Biome Restrictions | Seed |
|---------|---------|----------------|-------------------|------|
| Village | 64 chunks | 8 chunks | Plains, Desert, Taiga, Savanna | 10387312 |
| Stronghold | 3 rings | - | Any | 432 |
| Mineshaft | 0.01 chance | - | Underground | 10387313 |
| Temple (Desert/Jungle) | 32 chunks | 8 chunks | Desert/Jungle | 14357617 |
| Witch Hut | 0.01 chance | - | Swampland | 165745296 |
| Dungeon | 8 per chunk | - | Underground | seed^worldSeed |

---

## 5. Decoration (Per-Chunk)

Biome-specific decoration applied after terrain:
1. **Trees**: Oak, Birch, Spruce, Jungle, Acacia, Dark Oak, Huge Mushroom
2. **Flowers**: Yellow Flower, Red Flower, Double Plants
3. **Grass/Tall Grass**: Random placement
4. **Reeds/Sugar Cane**: Near water
5. **Cactus**: Desert only
6. **Dead Bush**: Desert/Savanna
7. **Waterlily**: Swampland water
8. **Huge Mushrooms**: Mushroom Island
8. **Clay/Gravel/Sand**: River/ocean beds

---

## 6. Nether Generation

- Biome: Hell (single biome)
- Scale: 3× (Classic), 3× (Small), 6× (Medium), 8× (Large) relative to Overworld
- Terrain: Netherrack base, Soul Sand valleys, Basalt Deltas (not in TU19)
- Structures: Nether Fortress (spacing 32, separation 8)
- Features: Glowstone blobs, Lava oceans, Soul Sand valleys

---

## 7. End Generation

- Biome: Sky (single biome)
- Fixed size: 18×18 chunks
- Terrain: End Stone platform, obsidian pillars
- Structure: Exit Portal at center, Ender Dragon spawn

---

## 8. World Size Presets

| Preset | Overworld Chunks | Overworld Blocks | Nether Scale | Edge Behavior |
|--------|------------------|------------------|--------------|---------------|
| Classic | 54×54 | 864×864 | 3× | Ocean moat |
| Small | 64×64 | 1024×1024 | 3× | Ocean moat |
| Medium | 192×192 | 3072×3072 | 6× | Ocean moat |
| Large | 320×320 | 5120×5120 | 8× | Ocean moat |
| Infinite | Unbounded | Unbounded | 8× | None |

Vertical: 128 blocks (y=0 to y=127), sea level y=63, max build height 256

---

## 9. RNG Details

### `GameRandom` (Exact Java `java.util.Random`)

```cpp
multiplier = 0x5DEECE66D
addend = 0xB
mask = (1 << 48) - 1

next(bits):
  seed = (seed * multiplier + addend) & mask
  return (seed >> (48 - bits))

nextInt(bound):
  if (bound & -bound) == bound: // power of 2
    return (bound * next(31)) >> 31
  do:
    bits = next(31)
    val = bits % bound
  while (bits - val + (bound - 1) < 0)
  return val
```

### Sine Lookup Table

- 65536 entries (`sin` index = `(angle * 65536 / 2π) & 0xFFFF`)
- `cos(x) = sin(x + 16384)`
- Matches MSVC `cvttss2si` overflow semantics (INT_MIN cast)

---

## 10. Reference Files

| File | Purpose |
|------|---------|
| `Layer.h/cpp` | Base layer class, `getArea`, `init` |
| `IslandLayer.h/cpp` | Initial land generation |
| `ZoomLayer.h/cpp` | 2× zoom with interpolation |
| `AddIslandLayer.h/cpp` | Add small islands |
| `AddSnowLayer.h/cpp` | Add snow biomes |
| `BiomeInitLayer.h/cpp` | Initialize biome IDs |
| `RegionHillsLayer.h/cpp` | Hills variants |
| `AddMushroomIslandLayer.h/cpp` | Mushroom islands |
| `GrowMushroomIslandLayer.h/cpp` | Expand mushroom islands |
| `ShoreLayer.h/cpp` | Beaches |
| `SwampRiversLayer.h/cpp` | Swamp/river biomes |
| `SmoothLayer.h/cpp` | Smooth transitions |
| `RiverInitLayer.h/cpp` | River initialization |
| `RiverLayer.h/cpp` | River carving |
| `RiverMixerLayer.h/cpp` | River + biome mixing |
| `VoronoiZoom.h/cpp` | Voronoi zoom for block resolution |
| `FuzzyZoomLayer.h/cpp` | Jittered zoom |
| `BiomeDecorator.h/cpp` | Decoration placement |
| `RandomLevelSource.h/cpp` | Terrain generation entry |
| `ChunkProvider.h/cpp` | Chunk generation pipeline |
| `ImprovedNoise.h/cpp` | Perlin noise implementation |
| `Level.h` | World constants (height, sea level, etc.) |

---

## 11. Known Divergences from Java Edition

1. **No 1.9+ combat changes** - TU19 is pre-combat update
2. **Finite worlds** - Console editions have bounded worlds
3. **Nether scale** - Console Nether is 3/3/6/8× not 8×
3. **No End cities** - Pre-1.9
4. **No woodland mansions** - Pre-1.11
4. **No ocean monuments** - Pre-1.8
5. **Different biome IDs** - Console uses sequential IDs
5. **No biome variants** - No "Mutated" biomes
6. **Different decoration** - Console has different tree/flower counts

---

## 12. Parity Test Vectors

### Fixed Seeds for Testing

| Seed | Preset | Expected Spawn | Notes |
|------|--------|----------------|-------|
| 0 | Classic | x=?, y=?, z=? | Empty string seed |
| 12345 | Classic | x=?, y=?, z=? | Numeric seed |
| -12345 | Small | x=?, y=?, z=? | Negative seed |
| "minecraft" | Medium | x=?, y=?, z=? | String hashCode |
| "test" | Large | x=?, y=?, z=? | String hashCode |

*Exact coordinates to be filled from reference dumps*

---

## 13. Differential Debugger Design

For debugging parity failures:

1. **Stage 1**: Compare biome maps (chunk-level)
2. **Stage 2**: Compare heightmaps (17×17 grid)
3. **Stage 3**: Compare surface blocks (first non-air)
4. **Stage 4**: Compare caves/ravines
5. **Stage 6**: Compare structures
6. **Stage 7**: Compare decoration

At each stage, report:
- First mismatching chunk (x, z)
- Expected vs actual value
- `GameRandom` state at divergence point
- Call stack to trace RNG consumption

---

## 14. Implementation Checklist

- [ ] Biome layer stack (16 layers) ✅
- [ ] Biome registry (23 biomes) ✅
- [ ] Seed derivation ✅
- [ ] Heightmap generation ✅
- [ ] Surface building ✅
- [ ] Cave generation (A* pathfinding) ✅
- [ ] Ravine generation (A* pathfinding) ✅
- [ ] Village generation ✅
- [ ] Stronghold generation ✅
- [ ] Mineshaft generation ✅
- [ ] Temple generation ✅
- [ ] Witch hut generation ✅
- [ ] Dungeon generation ✅
- [ ] Decoration system ✅
- [ ] Nether generation ✅
- [ ] End generation ✅
- [ ] World size presets ✅
- [ ] `GameRandom` exact port ✅
- [ ] Sine lookup table ✅
- [ ] Parity harness ⬜
- [ ] Reference dumps ⬜
- [ ] Differential debugger ⬜
- [ ] `docs/reference/worldgen.md` ✅ (this file)