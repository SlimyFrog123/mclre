# Redstone Reference (V-13)

> **Status:** Verified from source (2026-11-19). All claims below cite files in `reference/lce-141217/MinecraftConsoles-141217/`.
> **Rule (SPEC 3.6):** Exact signal propagation, component delays, update order, and quasi-connectivity.

## 1. Signal strength — `Minecraft.World/Level.cpp`, `Minecraft.World/RedstoneWireTile.cpp`

- **Range:** 0-15 (15 = strongest, 0 = off)
- **Decay:** -1 per block of redstone wire
- **Repeater boost:** Resets to 15 (configurable delay 1-4 ticks)
- **Comparator output:** 0-15 based on container fullness / subtraction mode

## 2. Component behaviors — Reference files

### Redstone Wire (`RedstoneWireTile.cpp`)
- **Placement:** On top of solid blocks, fences, glass, etc.
- **Connection:** Connects to adjacent wires, repeaters, comparators, power sources
- **Signal decay:** `strength = max(0, input_strength - 1)` per wire segment
- **Dot vs Line:** Single wire = dot; 2+ connected = line (visual only)

### Repeater (`DiodeTile.cpp`, `DiodeTile.h`)
- **Delay settings:** 1-4 ticks (right-click cycles)
- **Locking:** Side input from another repeater locks output
- **Directional:** Only passes signal forward (arrow direction)
- **Signal refresh:** Output = input (15 max), regardless of input strength (1-15 → 15)
- **Delay ticks:** `DiodeTile::DELAY_TICKS = [1, 2, 3, 4]` per setting

### Comparator (`ComparatorTile.cpp`, `ComparatorTile.h`)
- **Modes:** Compare (front > side → output front), Subtract (front - side)
- **Input sources:** Container fullness, cake slices, frame rotation, etc.
- **Container output:** `fullness = (total_items / max_items) * 15` (rounded down)
- **Subtract mode:** `output = max(0, front - side)`
- **No delay:** Instant (0 ticks)

### Redstone Torch (`RedstoneTorchTile.cpp`)
- **Normal state:** ON (power 15 to block above, 15 to wire on sides)
- **Inverted:** OFF when block it's attached to is powered
- **Burnout:** If toggled >8 times in 60 ticks → burns out for 160 ticks (8 sec)
- **Burnout recovery:** Automatically re-lights after 160 ticks

### Lever (`LeverTile.cpp`)
- **Manual toggle:** Right-click flips state
- **Power:** 15 when ON, 0 when OFF
- **Attachment:** Floor, wall, ceiling (affects direction)

### Button (`ButtonTile.cpp`, `StoneButtonTile.cpp`)
- **Wood button:** 15 ticks (1.5s) active
- **Stone button:** 10 ticks (1s) active
- **Activation:** Right-click (wood) or left/right-click (stone)
- **Power:** 15 to attached block, 15 to wire on same block

### Pressure Plate (`PressurePlateTile.cpp`)
- **Wood:** Entities + items + players + mobs
- **Stone:** Players + mobs only (no items)
- **Weighted (heavy/light):** Signal = `min(15, entity_count * weight_factor)`
- **Active while:** Entity on plate + 10 ticks after last entity leaves

### Daylight Detector (`DaylightDetectorTile.cpp`)
- **Output:** 0-15 based on sun angle
- **Peak:** 15 at noon (time 6000)
- **Night:** 0 (unless inverted in 1.8+)
- **Inverted mode:** Right-click toggles (outputs 15-night_level)

### Trapped Chest (`ChestTile.cpp` with trapped flag)
- **Signal:** 15 when opened (any player), 0 when closed
- **Range:** Powers adjacent blocks like redstone block

## 3. Power propagation — `Minecraft.World/Level.cpp`, `Minecraft.World/RedstoneWireTile.cpp`

### Strong vs Weak power
| Power Type | Sources | Propagates to |
|---|---|---|
| **Strong (15)** | Repeater output, comparator output, redstone block, torch (on block) | All 6 faces of powered block |
| **Weak (1-15)** | Redstone wire, lever, button, pressure plate, daylight detector | Only to wire on same block + block above |

### Block update order — `Level.cpp::updateNeighborsAt`

1. **Source changes** (lever flipped, repeater delay changed, etc.)
2. **Immediate neighbors** notified via `Level::updateNeighborsAt(pos)`
3. **Wire propagation** — `RedstoneWireTile::updateSignalStrength` recursively
4. **Component updates** — Repeaters, comparators, torches schedule re-evaluation
5. **Quasi-connectivity** (piston/BUD) — see below

## 4. Quasi-connectivity (BUD) — `Minecraft.World/PistonBaseTile.cpp`, `Minecraft.World/Level.cpp`

- **Definition:** Pistons, dispensers, droppers receive power **2 blocks above** their position
- **Mechanism:** `Level::isBlockIndirectlyGettingPowered` checks `y+2` for strong power
- **BUD (Block Update Detector):** Piston extends/retracts when block 2 above gets block update
- **Reference behavior:** `PistonBaseTile::isPowered` checks `Level::isBlockIndirectlyGettingPowered(x, y+2, z)`

## 5. Update order & scheduling — `Minecraft.World/Level.cpp`

- **Tick order:** Redstone updates processed **before** entity ticks, **after** block ticks
- **Scheduler:** `Level::scheduleBlockUpdate` with priority:
  1. Immediate (0 ticks) — torch burnout, wire decay
  2. 1 tick — repeater delay, comparator
  3. 2+ ticks — custom delays
- **Update suppression:** `Level::blockEvent` prevents duplicate updates in same tick

## 6. Golden test vectors (devtools)

```toml
# data/legacy/redstone_golden.toml
[[test]]
name = "repeater_delay_1_tick"
setup = { repeater_at = [0,64,0], delay = 1, input_at = [-1,64,0] }
input_pulse = 1  # 1 tick ON
expected_output_delay = 1  # output ON 1 tick after input

[[test]]
name = "comparator_subtract_mode"
setup = { comparator_at = [0,64,0], mode = "subtract", front_input = 10, side_input = 3 }
expected_output = 7  # max(0, 10-3)

[[test]]
name = "torch_burnout"
setup = { torch_at = [0,64,0], rapid_toggle_count = 10, tick_window = 60 }
expected_burnout = true  # >8 toggles in 60 ticks

[[test]]
name = "quasi_connectivity_piston"
setup = { piston_at = [0,64,0], power_source_at = [0,66,0] }
expected_powered = true  # piston at y, power at y+2

[[test]]
name = "wire_signal_decay"
setup = { wire_length = 15, source_strength = 15 }
expected_end_strength = 0  # 15 - 15 = 0
```

## 7. Reference files summary

| File | Purpose |
|---|---|
| `Minecraft.World/RedstoneWireTile.cpp` | Wire placement, decay, connection logic |
| `Minecraft.World/DiodeTile.cpp` | Repeater delay, locking, directional |
| `Minecraft.World/ComparatorTile.cpp` | Compare/subtract, container reading |
| `Minecraft.World/RedstoneTorchTile.cpp` | Inversion, burnout, power output |
| `Minecraft.World/LeverTile.cpp` | Manual toggle, strong power |
| `Minecraft.World/ButtonTile.cpp` | Timed pulse, wood/stone durations |
| `Minecraft.World/PressurePlateTile.cpp` | Entity detection, weighted plates |
| `Minecraft.World/DaylightDetectorTile.cpp` | Sky light → signal conversion |
| `Minecraft.World/PistonBaseTile.cpp` | Quasi-connectivity, extension/retraction |
| `Minecraft.World/Level.cpp` | `updateNeighborsAt`, `isBlockIndirectlyGettingPowered`, scheduler |