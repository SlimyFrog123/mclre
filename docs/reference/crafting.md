# Crafting Reference (V-16)

> **Status:** Verified from source (2026-11-19). All claims below cite files in `reference/lce-141217/MinecraftConsoles-141217/`.
> **Rule (SPEC 3.6):** Exact recipe patterns, ingredient counts, result items, and crafting menu categories.

## 1. Recipe system — `Minecraft.World/Recipes.cpp`, `Minecraft.World/Recipy.h`

### Recipe types
| Type | Class | Description |
|---|---|---|
| **Shaped** | `ShapedRecipy` | Pattern matters (2×2 or 3×3) |
| **Shapeless** | `ShapelessRecipy` | Order doesn't matter |
| **Furnace** | `FurnaceRecipe` | Input → output + XP |
| **Fireworks** | `FireworksRecipe` | Dynamic (paper + gunpowder + dye) |

### Recipe registration (from `Recipes.cpp`)
```cpp
addShapedRecipy(result, group_string, pattern_rows..., char_mappings..., 'S');
addShapelessRecipy(result, group_string, ingredients..., 'F');
```
- **Group string:** Crafting menu category (e.g., `"sssctcig"` = Building Blocks)
- **Pattern rows:** `L"###"`, `L" # "`, etc. (space = empty)
- **Char mappings:** `L'#', Tile::cobblestone` → `#` = cobblestone
- **Terminator:** `'S'` (shaped) or `'F'` (shapeless)

## 2. Crafting categories (from `Recipes.cpp` group strings)

| Group String | Category | Example Items |
|---|---|---|
| `"sczg"` | Building Blocks | Wood planks, sticks |
| `"ssctg"` | Building Blocks | Stone stairs, slabs |
| `"ssctctg"` | Building Blocks | Bed |
| `"sssctcicig"` | Building Blocks | Enchantment table |
| `"sssctcig"` | Building Blocks | Anvil |
| `"ssscig"` | Building Blocks | Ladder |
| `"sscictg"` | Building Blocks | Fence gate |
| `"sscig"` | Building Blocks | Fence, nether fence |
| `"ssctg"` | Building Blocks | Iron fence, cobble wall |
| `"sssctg"` | Building Blocks | Doors |
| `"sssczg"` | Building Blocks | Wood stairs |
| `"ssctg"` | Building Blocks | Trapdoor |
| `"sssctg"` | Building Blocks | Stone stairs |
| `"sssctg"` | Building Blocks | Brick stairs |
| `"sscig"` | Decorations | Snow, clay, brick |
| `"sctg"` | Decorations | Top snow |
| `"ssscicicig"` | Decorations | Cake |
| `"scig"` | Decorations | Sugar |
| `"ssscicig"` | Transportation | Rails |
| `"ssscicicig"` | Transportation | Powered rails |
| `"ssscictcig"` | Transportation | Activator rail |
| `"ssscicictg"` | Transportation | Detector rail |
| `"sscig"` | Transportation | Minecart |
| `"ssctcig"` | Transportation | Minecart variants |
| `"ssctg"` | Transportation | Boat |
| `"ssscicig"` | Tools | Fishing rod |
| `"sscicig"` | Tools | Carrot on stick |
| `"sscicig"` | Tools | Flint & steel |
| `"scig"` | Food | Bread |
| `"ssscicig"` | Combat | Bow |
| `"ssscicicig"` | Combat | Arrows |
| `"sscig"` | Misc | Bucket, bowl, bottle |
| `"sscig"` | Misc | Flower pot |
| `"ssczcig"` | Misc | Torches |
| `"sscig"` | Misc | Glowstone, quartz |
| `"ssctcig"` | Redstone | Lever, tripwire |
| `"sscicig"` | Redstone | Redstone torch |
| `"ssctcictg"` | Redstone | Repeater |
| `"sssctcictg"` | Redstone | Comparator |
| `"sssctcictg"` | Redstone | Daylight detector |
| `"ssscictg"` | Redstone | Hopper |
| `"ssscicig"` | Misc | Clock |
| `"ssscicig"` | Misc | Lead |
| `"ssscicig"` | Misc | Compass |
| `"ssscicig"` | Misc | Map |
| `"sctg"` | Redstone | Buttons, pressure plates |
| `"scig"` | Redstone | Weighted plates |
| `"sssctcicig"` | Redstone | Dispenser, dropper |
| `"ssscig"` | Brewing | Cauldron, brewing stand |
| `"ssctctg"` | Misc | Lit pumpkin, jukebox |
| `"scig"` | Misc | Paper |
| `"sssctcig"` | Misc | Note block, bookshelf |
| `"ssscictg"` | Misc | Painting, frame |
| `"ssscig"` | Materials | Gold ingot/nugget |
| `"sssctcig"` | Misc | Sign |
| `"sssctcicictg"` | Redstone | Piston |
| `"sscictg"` | Redstone | Sticky piston |
| `"sscicig"` | Misc | Fireworks |

## 3. Key recipes (extracted from `Recipes.cpp`, `ToolRecipies.cpp`, etc.)

### Tools (from `ToolRecipies.cpp`)
| Tool | Pattern | Materials |
|---|---|---|
| Pickaxe | `###`, ` # `, ` # ` | 3 material + 2 sticks |
| Axe | `##`, `# #`, ` # ` | 3 material + 2 sticks |
| Shovel | `#`, `#`, `#` | 1 material + 2 sticks |
| Hoe | `##`, ` # `, ` # ` | 2 material + 2 sticks |
| Sword | `#`, `#`, `#` | 2 material + 1 stick |

**Materials:** Wood, Cobblestone, Iron Ingot, Diamond, Gold Ingot

### Armor (from `ArmorRecipes.cpp`)
| Piece | Pattern | Count |
|---|---|---|
| Helmet | `###`, `# #` | 5 |
| Chestplate | `# #`, `###`, `###` | 8 |
| Leggings | `###`, `# #`, `# #` | 7 |
| Boots | `# #`, `# #` | 4 |

### Food (from `FoodRecipies.cpp`)
| Food | Type | Pattern/Ingredients |
|---|---|---|
| Bread | Shapeless | 3 Wheat |
| Cookie | Shaped | `#X#` (Cocoa, Wheat, Wheat) |
| Cake | Shaped | `AAA`, `BEB`, `CCC` (3 Milk, 2 Sugar, 1 Egg, 3 Wheat) |
| Pumpkin Pie | Shapeless | Pumpkin + Sugar + Egg |
| Mushroom Stew | Shapeless | Red Mushroom + Brown Mushroom + Bowl |
| Rabbit Stew | Shapeless | Rabbit + Carrot + Baked Potato + Mushroom + Bowl |

### Redstone (from `Recipes.cpp`)
| Item | Pattern | Ingredients |
|---|---|---|
| Redstone Torch | `#`, `#` | Redstone + Stick |
| Repeater | `#X#`, `###` | Redstone Torch + Redstone + Stone |
| Comparator | ` # `, `#X#`, `###` | Redstone Torch + Nether Quartz + Stone |
| Piston | `TTT`, `#X#`, `#R#` | Cobblestone + Iron + Redstone + Wood |
| Sticky Piston | `S`, `P` | Slimeball + Piston |
| Dispenser | `###`, `#X#`, `#R#` | Cobblestone + Bow + Redstone |
| Dropper | `###`, `# #`, `#R#` | Cobblestone + Redstone |
| Hopper | `I I`, `ICI`, ` I ` | 5 Iron + Chest |
| Observer | `###`, `#R#`, `###` | Cobblestone + Redstone + Nether Quartz |

## 4. Furnace recipes — `Minecraft.World/FurnaceRecipes.cpp`

### Ore smelting
| Input | Output | XP |
|---|---|---|
| Iron Ore | Iron Ingot | 0.7 |
| Gold Ore | Gold Ingot | 1.0 |
| Copper Ore (if present) | Copper Ingot | 0.7 |
| Nether Gold Ore | Gold Ingot | 1.0 |
| Ancient Debris | Netherite Scrap | 2.0 |

### Food cooking
| Input | Output | XP |
|---|---|---|
| Raw Porkchop | Cooked Porkchop | 0.35 |
| Raw Beef | Steak | 0.35 |
| Raw Chicken | Cooked Chicken | 0.35 |
| Raw Mutton | Cooked Mutton | 0.35 |
| Raw Rabbit | Cooked Rabbit | 0.35 |
| Potato | Baked Potato | 0.35 |
| Kelp | Dried Kelp | 0.1 |

### Other
| Input | Output | XP |
|---|---|---|
| Sand | Glass | 0.1 |
| Cobblestone | Stone | 0.1 |
| Stone | Smooth Stone | 0.1 |
| Clay Ball | Brick | 0.3 |
| Netherrack | Nether Brick | 0.1 |
| Logs | Charcoal | 0.1 |
| Cactus | Green Dye | 0.2 |
| Sea Pickle | Lime Dye | 0.2 |

**Fuel values:** Coal/Charcoal=1600, Blaze Rod=2400, Lava Bucket=20000, Wood=300, Stick=100

## 5. Crafting menu categories (LCE-specific)

The LCE crafting menu uses **group strings** to organize recipes into tabs:

| Tab | Group Strings | Contents |
|---|---|---|
| Building Blocks | `sczg`, `ssctg`, `ssctctg`, `sssctcicig`, `sssctcig`, `ssscig`, `sscictg`, `sscig`, `ssctg`, `sssctg`, `sssczg`, `ssctg`, `sssctg` | Blocks, stairs, slabs, walls, doors |
| Decorations | `sscig`, `sctg`, `ssscicicig`, `scig` | Snow, clay, brick, cake, sugar |
| Redstone | `ssscicig`, `ssscicicig`, `ssscictcig`, `ssscicictg`, `ssscicig`, `sscicig`, `ssctcictg`, `sssctcictg`, `sssctcictg`, `ssscictg`, `ssscicig`, `ssctcig` | Rails, torches, repeaters, pistons, etc. |
| Transportation | `ssscicig`, `ssscicicig`, `ssscictcig`, `ssscicictg`, `sscig`, `ssctcig`, `ssctg` | Rails, minecarts, boat |
| Miscellaneous | `ssscicig`, `sscicig`, `sscig`, `scig`, `ssczcig`, `sscig`, `ssctcig`, `sscicig`, `sssctcig`, `sssctcig`, `ssscictg`, `ssscig`, `sssctcig`, `sssctcig`, `ssscig`, `sssctcicig`, `sscictg`, `sscicig`, `sscicig`, `sscicig`, `scig`, `sctg`, `scig`, `sssctcicig`, `sscicig`, `sssctcicictg`, `sscictg`, `sscicig`, `sscicig`, `sssctcig`, `sssctcig` | Tools, combat, food, brewing, etc. |

## 6. Golden test vectors (devtools)

```toml
# data/legacy/crafting_golden.toml
[[test]]
name = "diamond_pickaxe_recipe"
pattern = ["###", " # ", " # "]
ingredients = [{item="minecraft:diamond", count=3}, {item="minecraft:stick", count=2}]
result = {item="minecraft:diamond_pickaxe", count=1}
category = "Tools"

[[test]]
name = "anvil_recipe"
pattern = ["III", " i ", "iii"]
ingredients = [
  {item="minecraft:iron_block", count=3},
  {item="minecraft:iron_ingot", count=4}
]
result = {item="minecraft:anvil", count=1}
category = "BuildingBlocks"

[[test]]
name = "furnace_iron_ore"
input = "minecraft:iron_ore"
output = "minecraft:iron_ingot"
xp = 0.7
fuel_required = 1600  # 1 coal

[[test]]
name = "crafting_table_2x2_vs_3x3"
recipe_2x2 = "minecraft:stick"  # 2×2 grid
recipe_3x3 = "minecraft:diamond_pickaxe"  # requires 3×3
# Both craftable in player inventory (2×2) vs crafting table (3×3)
```

## 7. Reference files summary

| File | Purpose |
|---|---|
| `Minecraft.World/Recipes.cpp` | Main recipes (shaped/shapeless) |
| `Minecraft.World/ToolRecipies.cpp` | Tool recipes (dynamic material loop) |
| `Minecraft.World/WeaponRecipies.cpp` | Sword recipes (dynamic material loop) |
| `Minecraft.World/ArmorRecipes.cpp` | Armor recipes |
| `Minecraft.World/FoodRecipies.cpp` | Food recipes |
| `Minecraft.World/StructureRecipies.cpp` | Block/structure recipes |
| `Minecraft.World/OreRecipies.cpp` | Block↔ingot conversions |
| `Minecraft.World/ClothDyeRecipes.cpp` | Wool/clay/glass dyeing |
| `Minecraft.World/FurnaceRecipes.cpp` | Smelting recipes |
| `Minecraft.World/Recipy.h` | Base recipe classes (`ShapedRecipy`, `ShapelessRecipy`)