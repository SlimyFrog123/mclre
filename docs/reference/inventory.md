# Inventory & Container Reference (V-08)

> **Status:** Verified from source (2026-11-19). All claims below cite files in `reference/lce-141217/MinecraftConsoles-141217/`.
> **Rule (SPEC 3.6):** Exact slot indices, click behaviors, quick-move logic, and sync protocol.

## 1. Inventory layout — `Minecraft.World/Player.cpp`, `Minecraft.World/Inventory.cpp`

| Container | Slots | Indices | Notes |
|---|---|---|---|
| Player main | 36 | 0-35 | 0-8=hotbar, 9-35=main inventory |
| Player armor | 4 | 36-39 | 36=feet, 37=legs, 38=chest, 39=head |
| Player offhand | 1 | 40 | Added in later TU; baseline has slot 40 |
| Crafting grid (player) | 4 | 41-44 | 2×2 grid in player inventory screen |
| Crafting output | 1 | 45 | Result slot |

**Reference:** `Player.cpp` constructs `Inventory` with 46 slots total (`Inventory.cpp:17`).

## 2. Container types — `Minecraft.World/AbstractContainerMenu.cpp`, `Minecraft.World/ChestTile.cpp`

| Container | Size | Slots | Tile Entity |
|---|---|---|---|
| Chest | 27 | 0-26 | `ChestTileEntity` |
| Large Chest | 54 | 0-53 | Two `ChestTileEntity` linked |
| Furnace | 3 | 0=input, 1=fuel, 2=output | `FurnaceTileEntity` |
| Dispenser | 9 | 0-8 | `DispenserTileEntity` |
| Dropper | 9 | 0-8 | `DropperTileEntity` |
| Crafting Table | 10 | 0-8=grid (3×3), 9=output | None (uses `CraftingMenu`) |
| Enchanting Table | 2 | 0=item, 1=lapis | None (`EnchantmentTableMenu`) |
| Brewing Stand | 4 | 0-2=potions, 3=fuel | `BrewingStandTileEntity` |
| Anvil | 3 | 0=item, 1=material, 2=output | None (`AnvilMenu`) |
| Hopper | 5 | 0-4 | `HopperTileEntity` |
| Beacon | 1 | 0=payment | `BeaconTileEntity` |

## 3. Click behaviors — `Minecraft.World/AbstractContainerMenu.cpp`

From `AbstractContainerMenu::clicked(slotIndex, buttonNum, clickType, player, looped)`:

| Click Type | Button | Shift | Behavior |
|---|---|---|---|
| PICKUP | 0 (left) | No | Pick up full stack |
| PICKUP | 1 (right) | No | Pick up half stack (round up) |
| QUICK_MOVE | 0/1 | Yes | Move stack between player ↔ container |
| PICKUP_ALL | 0 | No (drag) | Drag to distribute across slots |
| THROW | 0/1 | No (outside) | Drop stack on ground |
| QUICK_CRAFT | 2 (middle) | No | Clone stack to creative inventory |

**Quick-move logic (`quickMoveStack`):**
1. If clicking container slot → try move to player inventory (hotbar first, then main)
2. If clicking player slot → try move to container
3. Respects `Item::getMaxStackSize` and slot validity (`Slot::mayPlace`)
4. **Furnace special case:** Input slots only accept smeltable items; fuel slot only accepts fuel

## 4. Crafting grid — `Minecraft.World/CraftingMenu.cpp`, `Minecraft.World/Recipes.cpp`

- **Player 2×2:** `CraftingMenu` with 4 input + 1 output; `CraftingMenu::slotChanged` triggers recipe match
- **Crafting Table 3×3:** Separate `CraftingMenu` instance; same logic, larger grid
- **Recipe matching:** Iterates `Recipes::getRecipes()`; shaped first, then shapeless
- **Result slot:** Slot 4 (2×2) or 9 (3×3) — `Slot::mayPickup` allows; `Slot::mayPlace` denies manual placement

## 5. Inventory sync — `Minecraft.World/AbstractContainerMenu.cpp`, `Minecraft.World/Player.cpp`

- **Server → Client:** `ContainerMenu::sendAllData` sends full slot list on open; `ContainerMenu::slotChanged` sends single-slot updates
- **Client → Server:** `PlayerActionPacket` (click) → `AbstractContainerMenu::clicked` → response packets
- **Creative inventory:** `CreativeContainerMenu` — infinite stacks, no sync needed (client-side only)

## 6. Golden test vectors (devtools)

```toml
# data/legacy/inventory_golden.toml
[[test]]
name = "chest_quick_move_to_hotbar"
setup = { container = "chest", chest_slots = [{item="minecraft:dirt", count=64}] }
action = { type = "quick_move", slot = 0, player_slot = 9 }
expected = { chest = [], hotbar[0] = {item="minecraft:dirt", count=64} }

[[test]]
name = "furnace_fuel_slot_rejects_non_fuel"
setup = { container = "furnace", furnace_slots = [{item="minecraft:cobblestone"}] }
action = { type = "quick_move", slot = 0, player_slot = 9 }
expected = { furnace_input = {item="minecraft:cobblestone"}, furnace_fuel = [] }
```

## 7. Reference files summary

| File | Purpose |
|---|---|
| `Minecraft.World/Player.cpp` | Inventory construction, slot counts |
| `Minecraft.World/Inventory.cpp` | `ItemStack` management, max stack sizes |
| `Minecraft.World/AbstractContainerMenu.cpp` | Click handling, quick-move, sync |
| `Minecraft.World/ChestTile.cpp` / `ChestTileEntity.cpp` | Chest inventory, large chest linking |
| `Minecraft.World/FurnaceTile.cpp` / `FurnaceTileEntity.cpp` | Furnace slots, fuel validation |
| `Minecraft.World/CraftingMenu.cpp` | Recipe matching, result slot |
| `Minecraft.World/Slot.cpp` | `mayPlace`, `mayPickup`, stack limits |