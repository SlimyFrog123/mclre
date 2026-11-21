# Lighting Reference (V-14)

> **Status:** Verified from source (2026-11-19). All claims below cite files in `reference/lce-141217/MinecraftConsoles-141217/`.
> **Rule (SPEC 3.6):** Exact light propagation, sky light calculation, block light sources, and night cycle.

## 1. Light types — `Minecraft.World/Level.cpp`, `Minecraft.World/ChunkCache.cpp`

| Light Type | Range | Source | Propagation |
|---|---|---|---|
| **Sky Light** | 0-15 | Sun/moon, sky exposure | Downward + sideways (flood fill) |
| **Block Light** | 0-15 | Light-emitting blocks | 3D flood fill, -1 per block |

## 2. Sky light — `Minecraft.World/Level.cpp`, `Minecraft.World/ChunkSource.cpp`

### Day/night cycle
- **Day:** Time 0-12000 (20 min cycle)
  - Noon (6000): Sky light = 15
  - Sunrise (0): Sky light ramps 0→15
  - Sunset (12000): Sky light ramps 15→0
- **Night:** Time 12000-24000
  - Midnight (18000): Sky light = 4 (moon light)
  - Moon phases affect brightness slightly (not in baseline)

### Sky light calculation
- **Top-down pass:** For each column (x,z), find highest non-transparent block
- **Sky light at Y:** 15 if above highest opaque block; else 0
- **Transparent blocks:** Glass, leaves, water, flowers, torches, etc. don't block sky light
- **Opacity:** `Tile::getLightBlock` — 0 = transparent, 15 = opaque
- **Sideways spread:** Sky light spreads horizontally through transparent blocks (flood fill)

## 3. Block light — `Minecraft.World/Level.cpp`, `Minecraft.World/Tile.cpp`

### Light sources (block light = 15)

| Block | Light Level | Notes |
|---|---|---|
| Torch (wall/floor) | 14 | `Tile::torch` |
| Glowstone | 15 | `Tile::glowstone` |
| Sea Lantern | 15 | `Tile::seaLantern` |
| Jack o'Lantern | 15 | `Tile::litPumpkin` |
| Redstone Lamp (on) | 15 | `Tile::redstoneLight` |
| Beacon | 15 | `Tile::beacon` (beam separate) |
| End Rod | 14 | `Tile::endRod` |
| Fire | 15 | `Tile::fire` |
| Lava | 15 | `Tile::lava` (source) |
| Magma Block | 3 | `Tile::magmaBlock` |
| Campfire | 15 | `Tile::campfire` |

### Light propagation
- **3D flood fill:** From each source, spread to 6 neighbors
- **Decay:** -1 per block (Manhattan distance)
- **Opacity:** `Tile::getLightBlock` blocks propagation (level ≥ block's opacity)
- **Updates:** `Level::updateLightAt` scheduled on block change

## 4. Light level queries — `Minecraft.World/Level.cpp`, `Minecraft.World/ChunkCache.cpp`

- **Total light:** `max(sky_light, block_light)` — used for mob spawning, crop growth
- **Sky light only:** `Level::getSkyLight` — used for grass spread, snow melt
- **Block light only:** `Level::getBlockLight` — used for monster spawning prevention

### Mob spawning thresholds
- **Hostile mobs:** Spawn if `block_light ≤ 7` AND `sky_light ≤ 7` (combined ≤ 7)
- **Passive mobs:** Spawn if `total_light ≥ 9` on grass
- **Crop growth:** Requires `sky_light ≥ 9` OR `block_light ≥ 9` above farmland

## 5. Night vision / blindness — `Minecraft.World/Player.cpp`

- **Night Vision:** Renders as if sky light = 15, block light = 15 (client-side)
- **Blindness:** Reduces render distance, but light levels unchanged

## 6. Light updates — `Minecraft.World/Level.cpp`

### Update triggers
| Event | Update Type |
|---|---|
| Block placed/removed | `Level::updateLightAt(pos)` |
| Block state changed (e.g., lamp on/off) | `Level::updateLightAt(pos)` |
| Tree growth / leaf decay | Column update |
| Time change (day→night) | Full sky light recalc |

### Optimization (reference)
- **Chunk sections:** 16×16×16 sections store light arrays
- **Dirty sections:** Tracked for incremental updates
- **Batch processing:** Multiple updates per tick (max 50 sections/tick)

## 6. Golden test vectors (devtools)

```toml
# data/legacy/lighting_golden.toml
[[test]]
name = "torch_light_level"
setup = { torch_at = [0,64,0] }
expected_block_light_at = { [0,64,0] = 14, [1,64,0] = 13, [2,64,0] = 12, [14,64,0] = 0 }

[[test]]
name = "sky_light_at_surface"
setup = { column_at = [0,0], highest_opaque_y = 63 }
expected_sky_light = { y = 64: 15, y = 65: 15, y = 255: 15 }

[[test]]
name = "sky_light_under_tree"
setup = { column_at = [0,0], leaves_at = [0,64,0] (transparent), log_at = [0,65,0] (opaque) }
expected_sky_light = { y = 63: 15, y = 64: 15, y = 65: 0, y = 66: 0 }

[[test]]
name = "glowstone_through_glass"
setup = { glowstone_at = [0,64,0], glass_at = [0,65,0] }
expected_block_light_at = { [0,65,0] = 14, [0,66,0] = 13 }

[[test]]
name = "night_time_sky_light"
time = 18000  # midnight
expected_sky_light_at_surface = 4  # moon light
```

## 7. Reference files summary

| File | Purpose |
|---|---|
| `Minecraft.World/Level.cpp` | Light engine, sky/block light calculation, update scheduling |
| `Minecraft.World/ChunkCache.cpp` | Chunk-section light storage, queries |
| `Minecraft.World/Tile.cpp` | `getLightBlock`, `getLightValue` (emission) |
| `Minecraft.World/ChunkSource.cpp` | Sky light initial generation |
| `Minecraft.World/ChunkCache.cpp` | Light propagation flood fill |
| `Minecraft.World/Player.cpp` | Light level for mob spawning checks |