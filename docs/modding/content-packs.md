# MCLRE Content Packs Guide

This guide covers data packs and resource packs in MCLRE, how to create them, and how they integrate with the modding system.

## Table of Contents

1. [Overview](#overview)
2. [Data Packs](#data-packs)
3. [Resource Packs](#resource-packs)
3. [Pack Structure](#pack-structure)
4. [Loading Order](#loading-order)
5. [Hot Reloading](#hot-reloading)
6. [Examples](#examples)

---

## Overview

MCLRE supports two types of content packs, compatible with Minecraft Java Edition formats:

- **Data Packs**: Modify game logic (recipes, loot tables, tags, structures, advancements, etc.)
- **Resource Packs**: Modify client-side assets (textures, models, sounds, languages, fonts, etc.)

Both pack types use the same directory structure and `pack.mcmeta` format as Java Edition.

---

## Data Packs

Data packs modify server-side game logic. They are loaded by the server and synchronized to clients.

### Supported Data Pack Features

| Feature | Status | Description |
|---------|--------|-------------|
| Recipes | ✅ | Shaped, shapeless, smelting, blasting, smoking, campfire, stonecutting |
| Loot Tables | ✅ | Block, entity, chest, fishing, gift loot |
| Tags | ✅ | Block, item, entity, fluid, function tags |
| Structures | ✅ | Structure files (NBB/Structure Block format) |
| Advancements | ✅ | Custom advancement trees |
| Functions | ✅ | Function files for command sequences |
| Predicates | ✅ | Condition predicates for loot/advancements |
| Dimensions | ✅ | Custom dimension definitions |
| Worldgen | ✅ | Biome, noise, feature configuration |
| Game Rules | ✅ | Custom game rules |

### Data Pack Structure

```
datapack_name/
├── pack.mcmeta
├── data/
│   ├── minecraft/
│   │   ├── tags/
│   │   │   ├── blocks/
│   │   │   ├── items/
│   │   │   └── ...
│   │   ├── recipes/
│   │   ├── loot_tables/
│   │   ├── structures/
│   │   ├── advancements/
│   │   ├── functions/
│   │   ├── predicates/
│   │   ├── worldgen/
│   │   └── ...
│   └── custom_namespace/
│       ├── recipes/
│       ├── loot_tables/
│       └── ...
```

### pack.mcmeta

```json
{
  "pack": {
    "pack_format": 15,
    "description": "My Custom Data Pack"
  }
}
```

**pack_format**: Use `15` for MCLRE (matches Java Edition 1.20.x compatibility).

---

### Recipes

#### Shaped Recipe
```json
{
  "type": "minecraft:crafting_shaped",
  "pattern": [
    "AAA",
    "ABA",
    "AAA"
  ],
  "key": {
    "A": { "item": "minecraft:iron_ingot" },
    "B": { "item": "minecraft:diamond" }
  },
  "result": { "id": "custom:reinforced_iron", "count": 1 }
}
```

#### Shapeless Recipe
```json
{
  "type": "minecraft:crafting_shapeless",
  "ingredients": [
    { "item": "minecraft:iron_ingot" },
    { "item": "minecraft:gold_ingot" }
  ],
  "result": { "id": "custom:iron_gold_alloy", "count": 2 }
}
```

#### Smelting Recipe
```json
{
  "type": "minecraft:smelting",
  "ingredient": { "item": "custom:raw_alloy" },
  "result": "custom:alloy_ingot",
  "experience": 0.7,
  "cookingtime": 200
}
```

---

### Loot Tables

```json
{
  "type": "minecraft:block",
  "pools": [
    {
      "rolls": 1,
      "entries": [
        {
          "type": "minecraft:item",
          "name": "minecraft:diamond",
          "functions": [
            {
              "function": "minecraft:set_count",
              "count": { "min": 1, "max": 4 }
            },
            {
              "function": "minecraft:apply_bonus",
              "enchantment": "minecraft:fortune",
              "formula": "minecraft:ore_drops"
            }
          ]
        }
      ]
    }
  ]
}
```

---

### Tags

Tags group blocks/items/entities for easy referencing.

```json
{
  "values": [
    "minecraft:oak_log",
    "minecraft:spruce_log",
    "custom:custom_log"
  ]
}
```

Usage in recipes:
```json
{
  "ingredient": { "tag": "custom:custom_logs" }
}
```

---

## Resource Packs

Resource packs modify client-side assets. They are loaded by the client and can be server-enforced.

### Supported Resource Pack Features

| Feature | Status | Description |
|---------|--------|-------------|
| Textures | ✅ | Block, item, entity, particle, GUI textures |
| Models | ✅ | Block, item, entity models (JSON) |
| Block States | ✅ | Block state definitions |
| Sounds | ✅ | Sound events, sound definitions |
| Languages | ✅ | Translation files |
| Fonts | ✅ | Bitmap and TTF fonts |
| Shaders | ✅ | GLSL shaders (Iris/OptiFine compatible) |

### Resource Pack Structure

```
resourcepack_name/
├── pack.mcmeta
├── assets/
│   ├── minecraft/
│   │   ├── textures/
│   │   │   ├── block/
│   │   │   ├── item/
│   │   │   ├── entity/
│   │   │   ├── particle/
│   │   │   └── gui/
│   │   ├── models/
│   │   │   ├── block/
│   │   │   ├── item/
│   │   │   └── entity/
│   │   ├── blockstates/
│   │   ├── sounds.json
│   │   ├── sounds/
│   │   ├── lang/
│   │   ├── fonts/
│   │   └── shaders/
│   └── custom_namespace/
│       └── ...
```

### pack.mcmeta

```json
{
  "pack": {
    "pack_format": 15,
    "description": "My Custom Resource Pack"
  }
}
```

---

### Block Models

```json
{
  "parent": "minecraft:block/cube_all",
  "textures": {
    "all": "custom:blocks/custom_block"
  }
}
```

### Block States

```json
{
  "variants": {
    "": { "model": "custom:block/custom_block" }
  }
}
```

### Item Models

```json
{
  "parent": "minecraft:item/generated",
  "textures": {
    "layer0": "custom:items/custom_item"
  }
}
```

### Sounds

**sounds.json**:
```json
{
  "custom.block.custom_block.break": {
    "sounds": [
      { "name": "custom:block/custom_break", "pitch": 1.0, "volume": 1.0 }
    ]
  }
}
```

### Languages

`assets/custom_namespace/lang/en_us.json`:
```json
{
  "block.custom_namespace.custom_block": "Custom Block",
  "item.custom_namespace.custom_item": "Custom Item"
}
```

---

## Pack Structure

### Mod-Bundled Packs

Mods can include data/resource packs within their directory structure:

```
mod_folder/
├── mod.toml
├── mod.wasm
├── data/           # Data pack (auto-loaded)
│   └── custom_namespace/
└── assets/         # Resource pack (auto-loaded)
    └── custom_namespace/
```

Packs bundled with mods are automatically loaded when the mod is loaded.

### Standalone Packs

Place packs in the server/client `packs/` directory:

```
server/
├── world/
├── mods/
└── packs/
    ├── datapacks/
    │   ├── vanilla_tweaks/
    │   └── custom_datapack/
    └── resourcepacks/
        ├── faithful/
        └── custom_resourcepack/
```

---

## Loading Order

Packs are loaded in the following order (later packs override earlier ones):

1. **Vanilla** (built-in)
2. **Mod-bundled data packs** (in mod load order)
3. **Standalone data packs** (alphabetical)
4. **Vanilla resource pack** (built-in)
5. **Mod-bundled resource packs** (in mod load order)
6. **Standalone resource packs** (alphabetical, client-side order)

### Client-Side Resource Pack Ordering

Clients can reorder resource packs in the Resource Pack menu. Server-enforced packs (sent via `MOD_LIST`/`MOD_MISMATCH`) are automatically applied and cannot be disabled.

---

## Hot Reloading

### Development Mode

In development mode (`--dev` flag), MCLRE supports hot reloading:

- **Data packs**: Press `F3+T` to reload data packs
- **Resource packs**: Press `F3+T` to reload resource packs
- **Mods**: Use `/mod reload <mod_id>` command

### Configuration

```toml
# server.toml
[dev]
hot_reload = true
data_pack_reload_key = "F3+T"
resource_pack_reload_key = "F3+T"
```

---

## Examples

### Example 1: Custom Ore Data Pack

**Structure**:
```
custom_ores/
├── pack.mcmeta
└── data/
    └── custom_ores/
        ├── tags/
        │   └── blocks/
        │       └── ores.json
        ├── recipes/
        │   └── copper_ingot_from_raw.json
        └── loot_tables/
            └── blocks/
                └── copper_ore.json
```

**pack.mcmeta**:
```json
{
  "pack": {
    "pack_format": 15,
    "description": "Custom Ores Data Pack"
  }
}
```

**tags/blocks/ores.json**:
```json
{
  "values": [
    "custom_ores:copper_ore",
    "custom_ores:tin_ore"
  ]
}
```

---

### Example 2: Custom Block Resource Pack

**Structure**:
```
custom_blocks_rp/
├── pack.mcmeta
└── assets/
    └── custom_mod/
        ├── models/
        │   ├── block/
        │   │   └── ruby_block.json
        │   └── item/
        │       └── ruby.json
        ├── blockstates/
        │   └── ruby_block.json
        ├── textures/
        │   └── block/
        │       └── ruby_block.png
        └── lang/
            └── en_us.json
```

**blockstates/ruby_block.json**:
```json
{
  "variants": {
    "": { "model": "custom_mod:block/ruby_block" }
  }
}
```

**models/block/ruby_block.json**:
```json
{
  "parent": "minecraft:block/cube_all",
  "textures": {
    "all": "custom_mod:block/ruby_block"
  }
}
```

**lang/en_us.json**:
```json
{
  "block.custom_mod.ruby_block": "Ruby Block",
  "item.custom_mod.ruby": "Ruby"
}
```

---

## Best Practices

1. **Namespace everything**: Use `custom_mod:feature` to avoid conflicts
2. **Version your packs**: Include version in pack description
3. **Test on clean installs**: Verify packs work without other mods
4. **Use tags**: Tag your blocks/items for compatibility
5. **Document dependencies**: List required mods/packs in README

---

## Troubleshooting

### Pack Not Loading
- Check `pack.mcmeta` syntax (valid JSON, correct pack_format)
- Verify directory structure matches specification
- Check server logs for errors

### Texture Not Showing
- Verify texture path matches model reference
- Ensure texture is power-of-2 dimensions (16x16, 32x32, etc.)
- Check for transparency issues

### Recipe Not Working
- Verify ingredient item IDs are correct
- Check for typos in pattern/key
- Ensure result item exists

---

## See Also

- [API Reference](api-reference.md)
- [Hello Mod Tutorial](hello-mod-tutorial.md)
- [Java Edition Data Pack Wiki](https://minecraft.wiki/w/Data_pack)
- [Java Edition Resource Pack Wiki](https://minecraft.wiki/w/Resource_pack)