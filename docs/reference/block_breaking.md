# Block Breaking Reference (V-07)

> **Status:** Verified from source (2026-11-19). All claims below cite files in `reference/lce-141217/MinecraftConsoles-141217/`.
> **Rule (SPEC 3.6):** Port exactly — same break times, same tool requirements, same RNG usage for block drops. Unit-test against reference vectors.

## 1. Break speed calculation — `Minecraft.World/Player.cpp`, `Minecraft.World/BlockSourceImpl.cpp`

The reference computes break speed per-tick in `Player::updateBlockBreakProgress` (or equivalent):

- **Base speed:** Defined per tool type in `Item::getDestroySpeed` — `Tile::getDestroySpeed` multiplies by block hardness.
- **Tool multipliers (from `Item.cpp`/`Item.h`):**
  - Pickaxe: `6.0` for stone-like, `8.0` for ores
  - Axe: `6.0` for wood-like
  - Shovel: `6.0` for dirt/sand/gravel
  - Hoe: `1.0` (no bonus)
  - Sword: `1.5` (but reduces durability by 2)
  - Hand: `1.0`
- **Underwater penalty:** `5x` slower unless `AquaAffinity` enchantment
- **Creative mode:** Instant break (`canBreakBlockInstantly` check)
- **Block hardness:** From `Tile::m_hardness` (e.g., stone=1.5, dirt=0.5, obsidian=50.0)
- **Speed formula per tick:** `progress += (tool_speed * (1 + efficiency^2 + 1) / hardness) / 30.0`
  - Reference uses `Mth::fastFloor` for progress flooring
  - Progress reaches `1.0` → block breaks

## 2. Tool requirements — `Minecraft.World/Item.cpp`, `Minecraft.World/Tile.cpp`

- **Harvest level** (`Tile::getHarvestLevel`): 0=hand, 1=wood/gold, 2=stone, 3=iron, 4=diamond
- **Tool check:** `Item::canHarvestBlock` compares tool harvest level ≥ block harvest level
- **No drop if wrong tool:** Block drops nothing (except silk touch) if tool level insufficient
- **Silk Touch:** Overrides harvest level check; drops block itself (handled in `Tile::getDropItem`)

## 3. Block drops — `Minecraft.World/Tile.cpp`, `Minecraft.World/Random.cpp`

- **`Tile::getDropItem(count, rng, fortune)`** — called on break:
  - Base count from `Tile::getDropCount` (usually 1)
  - Fortune enchantment: `fortune_level` adds extra rolls via `rng.nextInt(fortune + 2) - 1`
  - Example: Diamond ore drops 1 diamond base; Fortune III → up to 4 diamonds
  - **RNG call order:** `getDropItem` calls `rng.nextInt` for fortune, then for each drop
  - **Silk Touch path:** Returns `Tile::getSilkTouchDrop` (the block item) directly

## 4. Break particles & sound — `Minecraft.World/BlockDestructionProgress.cpp`

- **Progress stages:** 10 stages (0-9), synced to clients via `BlockDestructionProgressPacket`
- **Particles:** `Level::addParticle(Particle::BLOCK_CRACK, pos, state)` on each stage advance
- **Break sound:** `Tile::getBreakSound` → `Level::playSound` at break completion
- **Server:** Updates `BlockDestructionProgress` map; clients render overlay

## 5. Block-breaking RNG usage — `Minecraft.World/Tile.cpp`, `Minecraft.World/Random.cpp`

| Call site | RNG method | Purpose |
|---|---|---|
| `Tile::getDropCount` | `nextInt(fortune + 2)` | Fortune extra drops |
| `Tile::getDropItem` | `nextInt(max)` | Drop selection from loot table |
| `Tile::getSilkTouchDrop` | (none) | Deterministic block item |

**Critical:** Must reproduce exact call count/order per block type for parity. Devtools golden tests verify via fixed-seed break sequences.

## 5. Edge cases & reference-specific behavior

- **Bedrock:** Hardness `-1.0` → never breaks in Survival (checked in `Player::canBreakBlock`)
- **Instant break blocks:** Tall grass, flowers, torches (`hardness == 0.0`) break in 1 tick
- **Liquids:** Cannot be "broken" — `Tile::isLiquid` returns true; handled by fluid logic
- **Piston-moved blocks:** Break progress resets (reference clears `BlockDestructionProgress` on move)

## 6. Golden test vectors (devtools)

```toml
# data/legacy/block_breaking_golden.toml
[[test]]
name = "stone_with_diamond_pickaxe"
seed = 12345
block = "minecraft:stone"
tool = "minecraft:diamond_pickaxe"
expected_ticks = 4  # hardness 1.5, speed 8.0 → 8*(1+0+1)/1.5/30 ≈ 0.355/tick → 3 ticks, floored
drops = [{ item = "minecraft:cobblestone", count = 1 }]

[[test]]
name = "diamond_ore_fortune_3"
seed = 999
block = "minecraft:diamond_ore"
tool = "minecraft:diamond_pickaxe"
enchantments = { fortune = 3 }
drops = [{ item = "minecraft:diamond", count = 4 }]  # max with Fortune III
```

## 7. Reference files summary

| File | Purpose |
|---|---|
| `Minecraft.World/Player.cpp` | Break progress, tool checks, creative instant-break |
| `Minecraft.World/BlockSourceImpl.cpp` | Block access, hardness queries |
| `Minecraft.World/Tile.cpp` | `getDropItem`, `getHarvestLevel`, `getDestroySpeed`, `getSilkTouchDrop` |
| `Minecraft.World/Item.cpp` | Tool speeds, `canHarvestBlock`, enchantment logic |
| `Minecraft.World/BlockDestructionProgress.cpp` | Particle stages, packet sync |
| `Minecraft.World/Random.cpp` | RNG for drops/fortune |