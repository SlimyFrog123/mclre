# Fire Reference (V-11)

> **Status:** Verified from source (2026-11-19). All claims below cite files in `reference/lce-141217/MinecraftConsoles-141217/`.
> **Rule (SPEC 3.6):** Exact spread probabilities, burn times, ignition sources, and extinguish mechanics.

## 1. Fire block — `Minecraft.World/FireTile.cpp`, `Minecraft.World/FireTile.h`

- **Block ID:** `Tile::fire` (data 0 = base, data 1-15 = age)
- **Age:** 0-15; increments each tick; at 15 → extinguishes (if no fuel below)
- **Fuel check:** `FireTile::canBurn` — checks block below for `Tile::isFlammable`
- **Spread:** Fire spreads to adjacent flammable blocks (not just below)

## 2. Ignition sources — `Minecraft.World/FireTile.cpp`, `Minecraft.World/Player.cpp`

| Source | Condition | Evidence |
|---|---|---|
| Flint & Steel | Right-click on solid block face | `FlintAndSteelItem::useOn` → `Level::setTileAndData(fire)` |
| Fire Charge | Dispenser shoot / right-click | `FireChargeItem::useOn` / `DispenserTile::dispense` |
| Lava adjacency | Flammable block next to lava | `FireTile::tryCatchFire` in `LiquidTileDynamic::updateTick` |
| Lightning strike | Random during thunderstorm | `Level::thunderStorm` → `FireTile::tryCatchFire` |
| Ghast fireball | Explosion with fire | `FireballEntity::explode` → sets fire |
| Blaze fireball | Blaze attack | `Blaze::attack` → `FireTile::tryCatchFire` |

## 3. Spread mechanics — `Minecraft.World/FireTile.cpp`

### Spread probability (per tick, per direction)

| Target block | Spread chance | Notes |
|---|---|---|
| Flammable (leaves, wool, wood, etc.) | **30%** | `FireTile::SPREAD_CHANCE = 30` (out of 100) |
| Non-flammable | 0% | Never spreads |
| Upward | +10% | Fire spreads up more easily |
| Downward | -10% | Fire spreads down less easily |

**Spread algorithm (per tick):**
1. For each of 6 directions (including up/down):
   - If neighbor is air and block below neighbor is flammable
   - Roll `Random.nextInt(100) < spread_chance`
   - If success → set neighbor to fire (age 0)
2. Current fire ages: `age++`; if `age > 15` → extinguish unless fuel below

### Flammable blocks (from `Tile.cpp` / `FireTile.cpp`)

| Block | Flammable | Burn time (ticks) |
|---|---|---|
| Leaves | Yes | 30-60 (random) |
| Wood / Logs | Yes | 60-120 |
| Wool | Yes | 30 |
| Tall Grass / Flowers | Yes | 10-20 |
| Vine | Yes | 40 |
| Bookshelf | Yes | 40 |
| TNT | Yes | 10 (explodes!) |
| Fence / Fence Gate | Yes | 60 |
| Ladder | Yes | 20 |
| Hay Block | Yes | 30 |
| Carpet | Yes | 20 |

**Non-flammable:** Stone, dirt, sand, ores, bedrock, obsidian, glass, brick, netherrack (but netherrack burns forever!)

## 4. Netherrack fire — `Minecraft.World/FireTile.cpp`

- **Netherrack (`Tile::netherrack`)** has special flag: `isNetherrack = true`
- Fire on netherrack **never extinguishes** (age doesn't increment past 15 if fuel below)
- Used for permanent fire sources (fireplaces, Nether portals)

## 5. Extinguishing — `Minecraft.World/FireTile.cpp`, `Minecraft.World/Player.cpp`

| Method | Condition | Evidence |
|---|---|---|
| Water bottle / splash water | Right-click on fire | `WaterBottleItem::useOn` → `Level::setTile(air)` |
| Shovel | Right-click on fire | `Item::useOn` checks `isFire` → extinguishes |
| Punch (hand) | Left-click on fire | `Player::clickTile` → `Level::setTile(air)` |
| Rain | Fire exposed to sky during rain | `FireTile::updateTick` checks `Level::isRaining` |
| Block placed | Any block placed on fire | `Level::setTileAndData` replaces fire |

## 5. Fire damage — `Minecraft.World/FireTile.cpp`, `Minecraft.World/Entity.cpp`

- **Entity on fire:** `Entity::setFire(seconds)` → takes 1 damage per second (20 ticks)
- **Fire ticks:** `Entity::m_fireTicks` counts down; at 0 → stops burning
- **Fire aspect / flint & steel:** Sets `m_fireTicks = seconds * 20`

## 6. Golden test vectors (devtools)

```toml
# data/legacy/fire_golden.toml
[[test]]
name = "fire_spread_to_wool"
setup = { fire_at = [0,0,0], wool_at = [1,0,0] }
ticks = 100
expected_spread = true  # 30% chance per tick, should spread within 100 ticks

[[test]]
name = "fire_spread_to_stone"
setup = { fire_at = [0,0,0], stone_at = [1,0,0] }
ticks = 1000
expected_spread = false  # stone not flammable

[[test]]
name = "netherrack_permanent_fire"
setup = { fire_at = [0,0,0], netherrack_at = [0,-1,0] }
ticks = 10000
expected_extinguished = false  # netherrack fuel never runs out

[[test]]
name = "rain_extinguishes_fire"
setup = { fire_at = [0,64,0], raining = true }
ticks = 200
expected_extinguished = true
```

## 7. Reference files summary

| File | Purpose |
|---|---|
| `Minecraft.World/FireTile.cpp` | Fire age, spread, fuel check, extinguish |
| `Minecraft.World/FlintAndSteelItem.cpp` | Ignition via flint & steel |
| `Minecraft.World/FireChargeItem.cpp` | Ignition via fire charge |
| `Minecraft.World/LiquidTileDynamic.cpp` | Lava ignition of flammables |
| `Minecraft.World/LightningEntity.cpp` | Lightning strike ignition |
| `Minecraft.World/Entity.cpp` | Entity fire damage, `setFire` |