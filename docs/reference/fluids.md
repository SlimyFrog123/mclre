# Fluids Reference (V-10)

> **Status:** Verified from source (2026-11-19). All claims below cite files in `reference/lce-141217/MinecraftConsoles-141217/`.
> **Rule (SPEC 3.6):** Exact flow distances, spread rates, liquid interactions, and bucket behavior.

## 1. Fluid types — `Minecraft.World/LiquidTile.h`, `Minecraft.World/LiquidTile.cpp`

| Fluid | Tile ID | Static | Dynamic | Source block |
|---|---|---|---|---|
| Water | `Tile::water` | `LiquidTileStatic` | `LiquidTileDynamic` | `Tile::water` (data 0) |
| Lava | `Tile::lava` | `LiquidTileStatic` | `LiquidTileDynamic` | `Tile::lava` (data 0) |

- **Static:** Source block (level 0), doesn't flow
- **Dynamic:** Flowing liquid (levels 1-7), spreads from source

## 2. Flow mechanics — `Minecraft.World/LiquidTileDynamic.cpp`, `Minecraft.World/Level.cpp`

### Water (Overworld)
- **Spread rate:** 1 block per 5 ticks (20 ticks/sec → 4 blocks/sec)
- **Horizontal flow distance:** 7 blocks from source (levels 1-7)
- **Downward flow:** Infinite until hits bottom or block
- **Level calculation:** `level = source_distance` (1-7)

### Lava (Overworld)
- **Spread rate:** 1 block per 30 ticks (slower than water)
- **Horizontal flow distance:** 3 blocks from source (levels 1-3)
- **Downward flow:** Infinite
- **Level calculation:** `level = source_distance * 2` (1, 3, 5... mapped to 1-3)

### Lava (Nether)
- **Spread rate:** Same as water (1 per 5 ticks)
- **Horizontal flow distance:** 7 blocks (like water)
- **Reference:** `HellRandomLevelSource.cpp` / `Level.cpp` checks dimension

## 3. Liquid interactions — `Minecraft.World/LiquidTile.cpp`, `Minecraft.World/Tile.cpp`

| Interaction | Result | Evidence |
|---|---|---|
| Water + Lava (source) | Obsidian | `LiquidTile::checkMix` → `Tile::obsidian` |
| Water + Lava (flowing) | Cobblestone | `LiquidTile::checkMix` → `Tile::cobblestone` |
| Water (source) + Lava (flowing) | Stone | `LiquidTile::checkMix` → `Tile::stone` |
| Water + Lava (any) | Steam particles + hiss sound | `Level::addParticle(Particle::SMOKE)` |

**Interaction check:** Runs in `LiquidTile::updateTick` for each fluid block; scans 6 neighbors for opposite fluid.

## 4. Bucket behavior — `Minecraft.World/BucketItem.cpp`, `Minecraft.World/Item.cpp`

- **Right-click with empty bucket on source block:** Fills bucket, removes source block
- **Right-click with filled bucket:** Places source block at target, empties bucket
- **Lava bucket:** Fuel value 20000 ticks (smelts 100 items) — `Item::getFuelValue`
- **Water bucket:** No fuel value
- **Milk bucket:** Consumable, clears effects — `MilkBucketItem.cpp`

## 5. Cauldrons — `Minecraft.World/CauldronTile.cpp`, `Minecraft.World/CauldronTileEntity.cpp`

- **Capacity:** 3 levels (1 bottle = 1 level)
- **Fill sources:** Water bottle, rain (outdoors), splash potions
- **Rain fill rate:** 1 level per ~16 seconds (random tick)
- **Uses:** Fill bottles, dye armor (leather), wash banners, extinguish fire

## 6. Ice & snow — `Minecraft.World/IceTile.cpp`, `Minecraft.World/SnowTile.cpp`

- **Water freezes:** In cold biomes, at Y ≥ 90, light level < 13, adjacent to non-water → Ice
- **Ice melts:** Light level ≥ 11 (non-sunlight) → Water source
- **Snow layers:** 1-8 layers (`Tile::layers`); melt at light ≥ 11 or heat source

## 7. Golden test vectors (devtools)

```toml
# data/legacy/fluids_golden.toml
[[test]]
name = "water_flow_distance_overworld"
fluid = "water"
dimension = "overworld"
expected_max_level = 7
expected_horizontal_reach = 7

[[test]]
name = "lava_flow_distance_overworld"
fluid = "lava"
dimension = "overworld"
expected_max_level = 3
expected_horizontal_reach = 3

[[test]]
name = "water_lava_source_makes_obsidian"
setup = { water_source_at = [0,0,0], lava_source_at = [1,0,0] }
expected_result = { block = "minecraft:obsidian", at = [1,0,0] }

[[test]]
name = "bucket_pickup_removes_source"
action = "right_click_bucket"
target = "water_source"
expected = { bucket = "water_bucket", block = "air" }
```

## 8. Reference files summary

| File | Purpose |
|---|---|
| `Minecraft.World/LiquidTile.h/.cpp` | Base fluid logic, level math, mixing |
| `Minecraft.World/LiquidTileStatic.cpp` | Source block behavior |
| `Minecraft.World/LiquidTileDynamic.cpp` | Flowing liquid, spread rates |
| `Minecraft.World/Level.cpp` | Tick scheduling, dimension checks |
| `Minecraft.World/BucketItem.cpp` | Bucket fill/place logic |
| `Minecraft.World/CauldronTile.cpp` | Cauldron levels, rain fill |
| `Minecraft.World/IceTile.cpp` | Freeze/melt conditions |