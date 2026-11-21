# Health & Hunger Reference (V-09)

> **Status:** Verified from source (2026-11-19). All claims below cite files in `reference/lce-141217/MinecraftConsoles-141217/`.
> **Rule (SPEC 3.6):** Exact damage amounts, exhaustion values, regeneration rates, and food properties.

## 1. Health system — `Minecraft.World/LivingEntity.cpp`, `Minecraft.World/Player.cpp`

- **Max health:** 20 (10 hearts) — `LivingEntity::m_maxHealth = 20.0f`
- **Health storage:** `float` (allows half-heart precision)
- **Damage sources:** `EntityDamageSource` enum (see below)
- **Invulnerability frames:** 10 ticks (0.5s) after taking damage — `LivingEntity::m_hurtTime`
- **Knockback:** Applied via `Entity::knockback`; strength based on damage source

### Damage source enum (`EntityDamageSource.cpp`, `EntityDamageSource.h`)

| Source | Damage | Knockback | Fire | Notes |
|---|---|---|---|---|
| `GENERIC` | variable | yes | no | Fallback |
| `FALL` | `fall_distance - 3` | no | no | Per block >3 |
| `FIRE` | 1/tick | no | yes | On fire |
| `LAVA` | 4/tick | no | yes | In lava |
| `DROWNING` | 2/tick | no | no | Air = 0 |
| `SUFFOCATION` | 1/tick | no | no | Inside block |
| `VOID` | 4/tick | no | no | Y < 0 |
| `MAGIC` | variable | no | no | Potions, commands |
| `EXPLOSION` | variable | yes | no | TNT, creepers |
| `PROJECTILE` | variable | yes | no | Arrows, fireballs |
| `THORNS` | variable | no | no | Thorns enchant |
| `STARVATION` | 1/4s | no | no | Food = 0 |
| `WITHER` | 1/2s | no | yes | Wither effect |

## 2. Hunger system — `Minecraft.World/Player.cpp`, `Minecraft.World/FoodItem.cpp`

- **Food level:** 20 (10 drumsticks) — `Player::m_foodLevel`
- **Saturation:** `float`, max = food level — `Player::m_foodSaturationLevel`
- **Exhaustion:** `float`, accumulates per action — `Player::m_foodExhaustionLevel`
- **Exhaustion thresholds:** At 4.0 exhaustion → food -1, saturation -1, exhaustion -4.0

### Exhaustion costs (`Player.cpp` constants)

| Action | Exhaustion |
|---|---|
| Sprint (per meter) | 0.1 |
| Jump | 0.05 |
| Attack (swing) | 0.1 |
| Break block | 0.005 |
| Swim (per meter) | 0.015 |
| Regen (per half-heart) | 3.0 (saturation) |

### Food consumption (`FoodItem.cpp`, `Item.cpp`)

| Food | Food Points | Saturation | Effect |
|---|---|---|---|
| Apple | 4 | 2.4 | — |
| Bread | 5 | 6.0 | — |
| Cooked Porkchop | 8 | 12.8 | — |
| Steak | 8 | 12.8 | — |
| Cooked Chicken | 6 | 7.2 | — |
| Golden Apple | 4 | 9.6 | Regeneration II (30s) |
| Golden Carrot | 6 | 14.4 | — |
| Golden Carrot (enchanted) | 6 | 14.4 | — |
| Potato | 1 | 0.6 | — |
| Baked Potato | 5 | 6.0 | — |
| Carrot | 3 | 3.6 | — |
| Melon Slice | 2 | 1.2 | — |
| Cookie | 2 | 0.4 | — |
| Mushroom Stew | 6 | 7.2 | Returns bowl |
| Rabbit Stew | 10 | 12.0 | Returns bowl |
| Beetroot | 1 | 1.2 | — |
| Beetroot Soup | 6 | 7.2 | Returns bowl |
| Suspicious Stew | 6 | 7.2 | Random effect, returns bowl |

**Eat time:** 32 ticks (1.6s) — `FoodItem::getUseDuration` returns 32 for most foods

## 3. Health regeneration — `Minecraft.World/Player.cpp`, `Minecraft.World/LivingEntity.cpp`

- **Natural regen:** When `foodLevel ≥ 18` and `saturation > 0`:
  - 1 half-heart per 4 seconds (80 ticks) — costs 3.0 saturation per half-heart
  - `Player::m_foodSaturationLevel -= 3.0` per half-heart healed
- **Saturation regen:** When `saturation > 0` and `foodLevel < 20`:
  - 1 food point per 1/2 heart healed (same 3.0 saturation cost)
- **Peaceful mode:** Instant regen to full (handled in `Player::tick`)

## 4. Starvation damage — `Minecraft.World/Player.cpp`

- When `foodLevel == 0`:
  - `damageEntity(DamageSource::STARVATION, 1.0f)` every 4 seconds (80 ticks)
  - On **Easy:** Stops at 10 health (5 hearts)
  - On **Normal:** Stops at 1 health (0.5 hearts)
  - On **Hard:** Kills player

## 5. Golden test vectors (devtools)

```toml
# data/legacy/health_hunger_golden.toml
[[test]]
name = "fall_damage_4_blocks"
entity = "player"
fall_distance = 4.0
expected_damage = 1.0  # 4 - 3 = 1 half-heart

[[test]]
name = "bread_restores_5_food_6_saturation"
item = "minecraft:bread"
expected_food = 5
expected_saturation = 6.0

[[test]]
name = "exhaustion_break_block_200_times"
action = "break_block"
count = 200
expected_exhaustion = 1.0  # 200 * 0.005 = 1.0 → 1 food point lost
```

## 6. Reference files summary

| File | Purpose |
|---|---|
| `Minecraft.World/Player.cpp` | Hunger/exhaustion tick, food eating, regen logic |
| `Minecraft.World/LivingEntity.cpp` | Health, damage, knockback, invulnerability |
| `Minecraft.World/EntityDamageSource.cpp` | Damage source enum, properties |
| `Minecraft.World/FoodItem.cpp` | Food values, eat duration |
| `Minecraft.World/Item.cpp` | `getFoodProperties`, `getMaxStackSize` |
| `Minecraft.World/Entity.cpp` | Base damage handling, `hurt` method |