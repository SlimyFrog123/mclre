# Legacy data tables

These files map **legacy numeric ids** to engine data. They exist only for save
importers, parity tools, and data extraction — game logic, saves, and the wire
protocol use namespaced identifiers and block states (SPEC.md 5.2/5.3).

All files here are generated; do not edit them by hand.

| File | Generator | Source |
|---|---|---|
| `blocks.toml` | `cargo run -p mclre-devtools --bin extract-legacy` | `reference/lce-141217/MinecraftConsoles-141217/Minecraft.World/Tile.cpp` |

## `blocks.toml`

One `[[block]]` entry per legacy tile id (sorted by id):

| Field | Meaning |
|---|---|
| `id` | Legacy numeric tile id (`Tile::tiles[id]`) |
| `symbol` | Reference static member name (`stone`, `treeTrunk`, ...) |
| `class` | Reference C++ class (`StoneTile`, `LiquidTileDynamic`, ...) |
| `icon` | `setIconName` base name, if declared |
| `destroy_time` | `setDestroyTime` hardness in seconds (`-1` = indestructible) |
| `blast_resistance` | `setExplodeable`, if declared |
| `light_emission` | `setLightEmission` (reference stores `0.0..1.0`), if declared |
| `light_block` | `setLightBlock` opacity, if declared |
| `description_id` | Localization key source (`IDS_TILE_*`), if declared |
| `sends_tile_data` | Whether the block used legacy metadata (`sendTileData`) |

The extraction is mechanical: statements the parser does not recognize are
skipped, and duplicate ids are a hard error. Regenerate whenever the reference
submodule changes; the output must be byte-identical for the same source
(it is sorted and deterministically formatted).
