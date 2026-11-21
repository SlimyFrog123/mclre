//! Developer tools: reference-data extractors, registry dumpers, worldgen viewers, packet inspectors.
//!
//! Currently implemented:
//! - [`legacy_blocks`] — parses the reference `Tile.cpp` registration table into
//!   `data/legacy/blocks.toml` (legacy `(id)` → block metadata).
//! - [`legacy_recipes`] — parses the reference `Recipes.cpp`, `ToolRecipies.cpp`,
//!   `FoodRecipies.cpp`, etc. into `data/legacy/recipes.toml`.
//! - [`legacy_furnace`] — parses the reference `FurnaceRecipes.cpp` into
//!   `data/legacy/furnace.toml`.
//! - [`legacy_items`] — parses the reference `Item.cpp` into `data/legacy/items.toml`.
//!
//! The legacy mappings are import/parity data only; game logic never reads
//! numeric ids (SPEC.md 5.2/5.3).

pub mod legacy_blocks;
pub mod legacy_recipes;
pub mod legacy_furnace;
pub mod legacy_items;
