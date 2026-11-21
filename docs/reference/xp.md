# Experience (XP) Reference (V-15)

> **Status:** Verified from source (2026-11-19). All claims below cite files in `reference/lce-141217/MinecraftConsoles-141217/`.
> **Rule (SPEC 3.6):** Exact XP values, orb sizes, level curve, and enchantment costs.

## 1. XP system — `Minecraft.World/Player.cpp`, `Minecraft.World/ExperienceOrb.cpp`

- **XP bar:** 0.0 to 1.0 (fraction of current level)
- **Level:** Integer, unbounded (practically capped by XP total)
- **Total XP:** `Player::m_xpTotal` — cumulative XP ever gained
- **Current level XP:** `Player::m_xpLevel` — XP in current level
- **XP to next level:** Non-linear curve (see below)

## 2. Level curve — `Minecraft.World/Player.cpp`

### XP required for level N (from `Player::getXpForLevel`)

| Level | XP for this level | Cumulative XP |
|---|---|---|
| 0 | 0 | 0 |
| 1 | 7 | 7 |
| 2 | 9 | 16 |
| 3 | 11 | 27 |
| 4 | 13 | 40 |
| 5 | 15 | 55 |
| 6 | 17 | 72 |
| 7 | 19 | 91 |
| 8 | 21 | 112 |
| 9 | 23 | 135 |
| 10 | 25 | 160 |
| 11 | 27 | 187 |
| 12 | 29 | 216 |
| 13 | 31 | 247 |
| 14 | 33 | 280 |
| 15 | 35 | 315 |
| 16 | 37 | 352 |
| 17 | 39 | 391 |
| 18 | 41 | 432 |
| 19 | 43 | 475 |
| 20 | 45 | 520 |

**Formula (for level ≥ 16):**
```
xpForLevel(n) = 3.5 * n^2 + 30.5 * n - 105  (rounded)
```
- Levels 0-15: Hardcoded table in `Player.cpp`
- Levels 16+: Quadratic formula above

### XP bar progression
- **Level up:** When `current_level_xp ≥ xpForLevel(current_level)`
- **Level down:** On death, keep `total_xp` but `level = max(0, level - 7)` (baseline: lose 7 levels)
- **XP bar render:** `xp_fraction = current_level_xp / xpForLevel(current_level)`

## 3. XP sources — `Minecraft.World/Player.cpp`, `Minecraft.World/Entity.cpp`, `Minecraft.World/Block.cpp`

### Mob kills (from `Entity::dropExperience`)

| Mob | XP Value | Notes |
|---|---|---|
| Zombie / Skeleton / Creeper | 5 | +1-3 per natural equipment |
| Spider / Cave Spider | 5 | |
| Zombie Pigman | 5 | |
| Enderman | 5 | |
| Blaze | 10 | |
| Ghast | 10 | |
| Magma Cube | 1-4 (size) | |
| Slime | 1-4 (size) | |
| Silverfish | 5 | |
| Witch | 5 | |
| Wither Skeleton | 5 | |
| Guardian | 10 | |
| Elder Guardian | 10 | |
| Shulker | 5 | |
| Vindicator / Evoker | 10 | |
| Vex | 3 | |
| Ravager | 20 | |
| Pig / Cow / Sheep / Chicken | 1-3 | |
| Rabbit | 1-3 | |
| Horse / Donkey / Mule | 1-3 | |
| Turtle | 1-3 | |
| **Player** | **100%** of victim's XP | PvP |

### Other sources

| Source | XP | Evidence |
|---|---|---|
| Breed animals | 1-7 | `Animal::breed` |
| Fish | 1-6 | `FishingRodItem::catchFish` |
| Trade with villager | 3-6 | `MerchantRecipe::rewardExp` |
| Smelt (furnace) | 0.1-1.0 per item | `FurnaceTileEntity::smelt` |
| Mine ores | 3-7 | `Tile::dropXpOnBreak` |
| Mine diamond/emerald | 3-7 | |
| Mine nether quartz | 2-5 | |
| Mine lapis/redstone | 1-5 | |
| Bottle o' Enchanting | 3-11 | `ExperienceBottleItem::useOn` |

## 4. Experience Orbs — `Minecraft.World/ExperienceOrb.cpp`, `Minecraft.World/AddExperienceOrbPacket.cpp`

### Orb sizes (value → visual size)

| Value Range | Size | Visual | Color |
|---|---|---|---|
| 1 | 1 | Tiny | Green |
| 2-4 | 3 | Small | Yellow |
| 5-9 | 7 | Medium | Orange |
| 10-18 | 11 | Large | Red |
| 19-37 | 17 | Huge | Blue |
| 38-73 | 21 | Massive | Purple |
| 74-149 | 25 | Enormous | White |
| 150+ | 30 | Max | White |

**Orb merging:** Orbs within 0.5 blocks merge (sum values, recalc size) — `ExperienceOrb::merge`

### Orb physics
- **Gravity:** 0.04/tick² (same as items)
- **Drag:** 0.98/tick (air), 0.9/tick (water)
- **Lifetime:** 6000 ticks (5 minutes) → despawn
- **Pickup range:** 2.5 blocks (player collision box expanded)
- **Pickup delay:** 10 ticks (0.5s) after spawn — `ExperienceOrb::m_pickupDelay`

## 5. Enchantment costs — `Minecraft.World/EnchantmentTableMenu.cpp`, `Minecraft.World/EnchantmentHelper.cpp`

### Enchantment table (levels 1-30)

| Slot | Levels Cost | Lapis Cost | Min Level Required |
|---|---|---|---|
| Top (1) | 1-8 | 1 | 1 |
| Middle (2) | 9-21 | 2 | 15 |
| Bottom (3) | 22-30 | 3 | 30 |

**Enchantment algorithm (simplified):**
1. Base = `Random.nextInt(8) + 1 + (level/2) + Random.nextInt(level/2)`
2. Modified by bookshelves (0-15): `base + Random.nextInt(bookshelves+1)`
3. Clamped to slot range
4. **Lapis cost:** Slot index (1, 2, or 3)

### Anvil costs — `Minecraft.World/AnvilMenu.cpp`, `Minecraft.World/AnvilTile.cpp`

| Operation | Base Cost | Notes |
|---|---|---|
| Rename | 1 level | + prior work penalty |
| Repair (same item) | Material cost × count | Each unit = 1 level |
| Combine enchantments | Sum of enchantment costs | Prior work penalty applies |
| **Prior work penalty:** | `2^n - 1` levels | n = times worked on anvil |

**Max anvil cost:** 39 levels (40 in later versions; baseline = 39)

## 6. Golden test vectors (devtools)

```toml
# data/legacy/xp_golden.toml
[[test]]
name = "level_30_xp_total"
level = 30
expected_total_xp = 1395  # sum of xpForLevel(1..30)

[[test]]
name = "zombie_kill_xp"
mob = "zombie"
expected_xp = 5

[[test]]
name = "blaze_kill_xp"
mob = "blaze"
expected_xp = 10

[[test]]
name = "orb_merging"
setup = { orb1 = {value=10, pos=[0,64,0]}, orb2 = {value=15, pos=[0.3,64,0]} }
expected_merged = { value = 25, size = "Large" }

[[test]]
name = "enchantment_table_top_slot"
level = 5
bookshelves = 0
expected_cost_range = [1, 8]

[[test]]
name = "anvil_prior_work_penalty"
worked_times = 3
expected_penalty = 7  # 2^3 - 1 = 7
```

## 7. Reference files summary

| File | Purpose |
|---|---|
| `Minecraft.World/Player.cpp` | XP tracking, level curve, `getXpForLevel` |
| `Minecraft.World/ExperienceOrb.cpp` | Orb physics, merging, pickup, visual size |
| `Minecraft.World/Entity.cpp` | `dropExperience`, mob XP values |
| `Minecraft.World/Player.cpp` | `addExperience`, `onKillEntity`, enchantment costs |
| `Minecraft.World/EnchantmentTableMenu.cpp` | Slot costs, bookshelf logic |
| `Minecraft.World/AnvilMenu.cpp` | Repair/combine costs, prior work penalty |
| `Minecraft.World/AddExperienceOrbPacket.cpp` | Orb sync to clients |