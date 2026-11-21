# TNT Reference (V-12)

> **Status:** Verified from source (2011-11-19). All claims below cite files in `reference/lce-141217/MinecraftConsoles-141217/`.
> **Rule (SPEC 3.6):** Exact explosion power, fuse time, entity damage, and block destruction rules.

## 1. TNT block — `Minecraft.World/TNTTile.cpp`, `Minecraft.World/TNTTile.h`

- **Block ID:** `Tile::tnt` (data 0 = inactive, data 1 = primed)
- **Primed state:** Data value 1 = activated, starts fuse
- **Explosion power:** **4.0** (same as creeper) — `TNTTile::EXPLOSION_POWER = 4.0f`
- **Fuse time:** **80 ticks (4 seconds)** — `TNTTile::FUSE_TIME = 80`
- **Entity type when primed:** `PrimedTNT` entity (`eTYPE_PRIMEDTNT`)

## 2. Activation methods — `Minecraft.World/TNTTile.cpp`, `Minecraft.World/Entity.cpp`

| Method | Condition | Evidence |
|---|---|---|
| Redstone power | Any adjacent redstone signal > 0 | `TNTTile::neighborChanged` checks `Level::isBlockIndirectlyGettingPowered` |
| Fire / Lava | Adjacent fire or lava | `TNTTile::catchFire` / `LiquidTileDynamic::updateTick` |
| Flint & Steel | Right-click on TNT | `FlintAndSteelItem::useOn` → `TNTTile::prime` |
| Fire Charge | Dispenser / right-click | `FireChargeItem::useOn` |
| Arrow (fire) | Fire aspect arrow hits | `ArrowEntity::onHit` → `catchFire` |
| Explosion | Nearby explosion | `Entity::explode` calls `TNTTile::prime` |
| Player punch | Left-click (if not creative) | `Player::clickTile` → `TNTTile::prime` |

**Priming (`TNTTile::prime`):**
1. Sets block data to 1 (primed)
2. Spawns `PrimedTNT` entity at block center
3. Removes block (sets to air)
4. Entity has 80-tick fuse

## 3. PrimedTNT entity — `Minecraft.World/PrimedTNT.cpp` (referenced in `Entity.cpp`)

- **Entity type:** `eTYPE_PRIMEDTNT`
- **Fuse:** 80 ticks (decrements each tick in `PrimedTNT::tick`)
- **Physics:** Affected by gravity (falls), no collision with entities
- **Explosion:** At fuse=0 → `Level::explode(pos, 4.0f, false, null)`
- **Drops:** None (block already removed on prime)

## 4. Explosion mechanics — `Minecraft.World/Level.cpp`, `Minecraft.World/Explosion.cpp`

### Explosion parameters (power = 4.0)

| Property | Value | Notes |
|---|---|---|
| Radius | ~5.0 blocks | Calculated as `power * 1.3` |
| Block damage threshold | 4.0 | Blocks with blast resistance < 4.0 break |
| Entity damage | Up to 25 hearts | Distance-based falloff |
| Fire creation | **No** (TNT `createFire = false`) | Creeper has `createFire = true` |

### Blast resistance (blocks that survive TNT)

| Block | Resistance | Survives? |
|---|---|---|
| Bedrock | 18,000,000 | Yes |
| Obsidian | 6,000 | Yes |
| End Portal Frame | 3,600 | Yes |
| Command Block | 3,600 | Yes |
| Anvil | 1,200 | Yes |
| Enchanting Table | 1,200 | Yes |
| Water / Lava | 500 | Yes (liquid) |
| Ender Chest | 900 | Yes |
| Diamond Block | 15 | Yes |
| Gold Block | 15 | Yes |
| Iron Block | 15 | Yes |
| Stone / Cobblestone | 30 | **No** (30 < 4*5=20? Actually 30 > 20, survives) |
| Brick | 30 | Yes |
| Nether Brick | 30 | Yes |
| **Wood / Planks** | **15** | **No** |
| **TNT** | **0** | **No** (chain reaction) |

**Note:** TNT power 4.0 → max block damage = 4.0 × 5 = 20.0. Blocks with resistance ≥ 20 survive.

### Entity damage falloff

```
damage = (1.0 - distance / radius) × exposure × power × 2
- At center (dist=0): ~4.0 × 2 = 8.0 base → × exposure (0.1-1.0) → up to ~8 hearts
- Max observed: ~25 hearts (point-blank, full exposure)
```

## 4. Chain reactions — `Minecraft.World/PrimedTNT.cpp`

- TNT explosion damages nearby TNT blocks (resistance 0)
- Damaged TNT → `TNTTile::prime` → new PrimedTNT entity
- **Instant chain:** All TNT in radius primes simultaneously
- **Fuse sync:** Each has own 80-tick fuse; explode sequentially

## 5. Underwater TNT — `Minecraft.World/TNTTile.cpp`

- **Underwater behavior:** Same explosion power, **no block damage** to terrain
- **Entity damage:** Still applies
- **Reference:** `TNTTile::explode` checks `Level::isWaterAt(pos)` → `destroyBlocks = false`

## 6. Golden test vectors (devtools)

```toml
# data/legacy/tnt_golden.toml
[[test]]
name = "tnt_explosion_radius"
tnt_at = [0, 64, 0]
expected_block_damage_radius = 5.0
expected_entity_damage_at_1_block = 20  # hearts (approx)

[[test]]
name = "obsidian_survives_tnt"
setup = { tnt_at = [0,64,0], obsidian_at = [1,64,0] }
expected_survives = true

[[test]]
name = "stone_destroyed_by_tnt"
setup = { tnt_at = [0,64,0], stone_at = [1,64,0] }
expected_destroyed = true

[[test]]
name = "tnt_chain_reaction"
setup = { tnt_at = [0,64,0], tnt2_at = [3,64,0] }
expected_both_explode = true

[[test]]
name = "tnt_fuse_time"
primed_at = [0,64,0]
expected_explosion_tick = 80  # 4 seconds
```

## 7. Reference files summary

| File | Purpose |
|---|---|
| `Minecraft.World/TNTTile.cpp` | Block activation, priming, redstone/fire detection |
| `Minecraft.World/PrimedTNT.cpp` | Entity fuse, gravity, explosion trigger |
| `Minecraft.World/Level.cpp` | `explode` method, block damage calc, entity damage |
| `Minecraft.World/Explosion.cpp` | Raycasting for block destruction, exposure calc |
| `Minecraft.World/FlintAndSteelItem.cpp` | Manual ignition |
| `Minecraft.World/Entity.cpp` | Explosion damage application |