# Parity Test Scenarios

This directory contains golden-data driven parity tests that verify MCLRE behavior matches the LCE reference (Dec 17, 2014 baseline, profile `lce:141217`).

## Structure

```
parity/
├── scenarios/
│   ├── block_breaking/     # Tool effectiveness, hardness, drops
│   ├── crafting/           # Recipe matching, shaped/shapeless
│   ├── fluids/             # Flow mechanics, spread rules
│   ├── fire/               # Spread, ignition, burn times
│   ├── redstone/           # Signal strength, propagation
│   ├── tnt/                # Explosion radius, blast resistance
│   ├── lighting/           # Sky/block light propagation
│   ├── health_hunger/      # Damage, exhaustion, regen
│   ├── xp/                 # Orb values, level curve
│   └── inventory/          # Stack sizes, container operations
├── golden/
│   ├── recipes.json        # Extracted reference recipes
│   ├── furnace.json        # Extracted furnace recipes
│   ├── items.json          # Extracted item definitions
│   └── blocks.toml         # Extracted legacy blocks
└── runner.rs               # Test runner infrastructure
```

## Test Format

Each scenario is a YAML file describing inputs and expected outputs:

```yaml
# parity/scenarios/block_breaking/stone_pickaxe.yml
scenario: "stone_pickaxe_effectiveness"
description: "Stone pickaxe breaks stone at correct speed"
profile: "lce:141217"

setup:
  player:
    gamemode: Survival
    held_item: "minecraft:stone_pickaxe"
  world:
    block_at_0_64_0: "minecraft:stone"

action:
  type: "break_block"
  position: [0, 64, 0]
  duration_ticks: 20  # 1 second at 20 TPS

expected:
  block_broken: true
  drops: ["minecraft:cobblestone"]
  tool_damage: 1
```

## Running Parity Tests

```bash
# Run all parity tests
cargo test -p mclre-sim parity::

# Run specific category
cargo test -p mclre-sim parity::block_breaking::

# Update golden data (after verified reference changes)
cargo run -p mclre-devtools --bin update-golden
```

## Golden Data Sources

Golden data is extracted from the LCE reference (`reference/lce-141217/`) using the devtools extractors:

- `extract-legacy` → `data/legacy/blocks.toml` → `parity/golden/blocks.toml`
- `extract-legacy-recipes` → `data/legacy/recipes.toml` → `parity/golden/recipes.json`
- `extract-legacy-furnace` → `data/legacy/furnace.toml` → `parity/golden/furnace.json`
- `extract-legacy-items` → `data/legacy/items.toml` → `parity/golden/items.json`

## Adding New Scenarios

1. Identify the reference behavior from LCE source code
2. Create a scenario YAML in the appropriate category
3. Run the test - it will fail initially
4. Implement the behavior in the simulation
4. Test passes - scenario is now a regression guard

## Reference Citations

Each scenario should cite the reference source file and function:
- `# [REF: Tile.cpp:1234]` - Block definitions
- `# [REF: Recipes.cpp:567]` - Crafting recipes
- `# [REF: FurnaceRecipes.cpp:89]` - Smelting recipes
- `# [REF: Item.cpp:234]` - Item properties