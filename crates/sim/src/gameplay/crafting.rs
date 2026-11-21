//! Crafting system for MCLRE baseline profile (LCE-style console crafting).

use crate::{SimWorld, SimError, SimPlayer};
use mclre_core::{Identifier, BlockId};
use mclre_core::registry::Registry;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Crafting grid sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridSize {
    /// 2x2 player inventory crafting.
    Small,
    /// 3x3 crafting table.
    Large,
}

/// Crafting recipe for baseline profile.
#[derive(Debug, Clone)]
pub struct Recipe {
    /// Recipe identifier.
    pub id: Identifier,
    /// Recipe group/category (for LCE console crafting menu).
    pub group: Option<String>,
    /// Grid size required.
    pub grid_size: GridSize,
    /// Ingredients in reading order (top-left to bottom-right).
    /// None = empty slot.
    pub ingredients: Vec<Option<RecipeIngredient>>,
    /// Result item stack.
    pub result: RecipeResult,
    /// Whether this recipe is shapeless (order doesn't matter).
    pub shapeless: bool,
}

/// Recipe ingredient.
#[derive(Debug, Clone)]
pub struct RecipeIngredient {
    /// Item or block identifier.
    pub item: Identifier,
    /// Required count (default 1).
    pub count: u8,
    /// Required damage (for tools/armor), or wildcard.
    pub damage: Option<u16>,
    /// Whether this is a tag (e.g., #minecraft:planks).
    pub is_tag: bool,
}

/// Recipe result.
#[derive(Debug, Clone)]
pub struct RecipeResult {
    /// Output item.
    pub item: Identifier,
    /// Output count.
    pub count: u8,
    /// Result damage (for damaged tools).
    pub damage: u16,
    /// Result NBT data.
    pub nbt: Option<Vec<u8>>,
}

/// LCE crafting recipe groups (tabs in console crafting menu).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecipeGroup {
    BuildingBlocks,
    DecorationBlocks,
    Redstone,
    Transportation,
    Miscellaneous,
    Food,
    Tools,
    Combat,
    Brewing,
    Materials,
    // LCE-specific groups
    Decorations,
    Mechanisms,
    // Custom/mod groups can be added
}

/// Crafting manager for a player.
#[derive(Debug, Clone, Default)]
pub struct CraftingManager {
    /// 2x2 crafting grid (player inventory).
    pub small_grid: [Option<crate::gameplay::inventory::ItemStack>; 4],
    /// 3x3 crafting grid (crafting table).
    pub large_grid: [Option<crate::gameplay::inventory::ItemStack>; 9],
    /// Crafting output.
    pub output: Option<crate::gameplay::inventory::ItemStack>,
    /// Currently known recipes (unlocked).
    pub known_recipes: HashMap<Identifier, Recipe>,
    /// Recipe book state (which recipes are shown).
    pub recipe_book: RecipeBookState,
}

/// Recipe book state for LCE-style crafting menu.
#[derive(Debug, Clone, Default)]
pub struct RecipeBookState {
    /// Which recipes have been discovered/unlocked.
    pub unlocked: Vec<Identifier>,
    /// Current filter/group.
    pub current_group: Option<RecipeGroup>,
    /// Search filter text.
    pub search_filter: String,
    /// Whether to show only craftable recipes.
    pub craftable_only: bool,
}

/// Initialize a new crafting manager for a player.
pub fn create_crafting_manager() -> CraftingManager {
    CraftingManager::default()
}

/// Check if a crafting grid matches a recipe.
pub fn matches_recipe(
    grid: &[Option<crate::gameplay::inventory::ItemStack>],
    grid_size: GridSize,
    recipe: &Recipe,
) -> bool {
    if grid_size != recipe.grid_size {
        return false;
    }
    
    let expected_slots = match grid_size {
        GridSize::Small => 4,
        GridSize::Large => 9,
    };
    
    if grid.len() != expected_slots || recipe.ingredients.len() != expected_slots {
        return false;
    }
    
    if recipe.shapeless {
        // For shapeless, just check if all ingredients are present (count matters)
        let mut available: Vec<_> = grid.iter().filter_map(|s| s.clone()).collect();
        
        for ingredient in &recipe.ingredients {
            if let Some(ing) = ingredient {
                let mut found = false;
                for (i, stack) in available.iter_mut().enumerate() {
                    if matches_ingredient(stack, ing) {
                        // Consume from available
                        if stack.count >= ing.count {
                            stack.count -= ing.count;
                            if stack.count == 0 {
                                available.remove(i);
                            }
                            found = true;
                            break;
                        }
                    }
                }
                if !found {
                    return false;
                }
            }
        }
        true
    } else {
        // Shaped recipe - exact position matching
        for (i, ingredient) in recipe.ingredients.iter().enumerate() {
            if let Some(ing) = ingredient {
                let stack = &grid[i];
                if !matches_ingredient_opt(stack.as_ref(), ing) {
                    return false;
                }
            } else {
                // Slot should be empty
                if grid[i].is_some() {
                    return false;
                }
            }
        }
        true
    }
}

/// Check if an item stack matches an ingredient.
fn matches_ingredient(stack: &crate::gameplay::inventory::ItemStack, ingredient: &RecipeIngredient) -> bool {
    if stack.is_empty() {
        return false;
    }
    
    if ingredient.is_tag {
        // TODO: Check if item is in tag
        false
    } else {
        stack.item == Some(ingredient.item) &&
        (ingredient.damage.is_none() || stack.damage == ingredient.damage.unwrap())
    }
}

fn matches_ingredient_opt(stack: Option<&crate::gameplay::inventory::ItemStack>, ingredient: &RecipeIngredient) -> bool {
    match stack {
        Some(s) => matches_ingredient(s, ingredient),
        None => false,
    }
}

/// Get the crafting result for a grid.
pub fn get_crafting_result(
    grid: &[Option<crate::gameplay::inventory::ItemStack>],
    grid_size: GridSize,
    recipes: &[Recipe],
) -> Option<RecipeResult> {
    for recipe in recipes {
        if matches_recipe(grid, grid_size, recipe) {
            return Some(recipe.result.clone());
        }
    }
    None
}

/// Consume ingredients from crafting grid.
pub fn consume_ingredients(
    grid: &mut [Option<crate::gameplay::inventory::ItemStack>],
    recipe: &Recipe,
) {
    if recipe.shapeless {
        // For shapeless, consume from any matching slots
        for ingredient in &recipe.ingredients {
            if let Some(ing) = ingredient {
                for slot in grid.iter_mut() {
                    if let Some(stack) = slot {
                        if matches_ingredient(stack, ing) {
                            stack.count = stack.count.saturating_sub(ing.count);
                            if stack.count == 0 {
                                *slot = None;
                            }
                            break;
                        }
                    }
                }
            }
        }
    } else {
        // Shaped: consume from exact positions
        for (i, ingredient) in recipe.ingredients.iter().enumerate() {
            if let Some(ing) = ingredient {
                if let Some(stack) = &mut grid[i] {
                    stack.count = stack.count.saturating_sub(ing.count);
                    if stack.count == 0 {
                        grid[i] = None;
                    }
                }
            }
        }
    }
}

/// LCE-style recipe groups for console crafting menu.
/// These match the reference LCE crafting menu tabs.
pub const RECIPE_GROUPS: &[(RecipeGroup, &str)] = &[
    (RecipeGroup::BuildingBlocks, "Building Blocks"),
    (RecipeGroup::DecorationBlocks, "Decoration Blocks"),
    (RecipeGroup::Redstone, "Redstone"),
    (RecipeGroup::Transportation, "Transportation"),
    (RecipeGroup::Miscellaneous, "Miscellaneous"),
    (RecipeGroup::Food, "Food"),
    (RecipeGroup::Tools, "Tools"),
    (RecipeGroup::Combat, "Combat"),
    (RecipeGroup::Brewing, "Brewing"),
    (RecipeGroup::Materials, "Materials"),
];

/// Get all recipes for a group.
pub fn recipes_for_group(
    recipes: &[Recipe],
    group: RecipeGroup,
) -> Vec<&Recipe> {
    recipes.iter()
        .filter(|r| r.group.as_ref().map(|g| g == &group.to_string()).unwrap_or(false))
        .collect()
}

/// Convert RecipeGroup to string.
impl std::fmt::Display for RecipeGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecipeGroup::BuildingBlocks => write!(f, "Building Blocks"),
            RecipeGroup::DecorationBlocks => write!(f, "Decoration Blocks"),
            RecipeGroup::Redstone => write!(f, "Redstone"),
            RecipeGroup::Transportation => write!(f, "Transportation"),
            RecipeGroup::Miscellaneous => write!(f, "Miscellaneous"),
            RecipeGroup::Food => write!(f, "Food"),
            RecipeGroup::Tools => write!(f, "Tools"),
            RecipeGroup::Combat => write!(f, "Combat"),
            RecipeGroup::Brewing => write!(f, "Brewing"),
            RecipeGroup::Materials => write!(f, "Materials"),
            RecipeGroup::Decorations => write!(f, "Decorations"),
            RecipeGroup::Mechanisms => write!(f, "Mechanisms"),
        }
    }
}

/// JSON recipe format for loading from generated data.
#[derive(Debug, serde::Deserialize)]
struct JsonRecipe {
    id: String,
    name: String,
    category: String,
    shape: String,
    width: u8,
    height: u8,
    pattern: Vec<String>,
    ingredients: Vec<JsonIngredient>,
    result: JsonResult,
}

#[derive(Debug, serde::Deserialize)]
struct JsonIngredient {
    item: String,
    count: u8,
    damage: Option<u16>,
    is_tag: bool,
}

#[derive(Debug, serde::Deserialize)]
struct JsonResult {
    item: String,
    count: u8,
    damage: u16,
}

/// Load recipes from JSON data files (baseline profile).
pub fn load_recipes(registry: &Registry<crate::gameplay::inventory::ItemStack>) -> Vec<Recipe> {
    let mut recipes = Vec::new();
    
    // Load from data/legacy/recipes.json
    let recipes_path = Path::new("data/legacy/recipes.json");
    if recipes_path.exists() {
        if let Ok(content) = fs::read_to_string(recipes_path) {
            if let Ok(json_recipes) = serde_json::from_str::<Vec<JsonRecipe>>(&content) {
                for json_recipe in json_recipes {
                    if let Some(recipe) = convert_json_recipe(json_recipe) {
                        recipes.push(recipe);
                    }
                }
            }
        }
    }
    
    recipes
}

/// Convert a JSON recipe to our internal format.
fn convert_json_recipe(json: JsonRecipe) -> Option<Recipe> {
    let id = Identifier::parse(&json.id).ok()?;
    let group = Some(json.category);
    let grid_size = if json.width <= 2 && json.height <= 2 {
        GridSize::Small
    } else {
        GridSize::Large
    };
    let shapeless = json.shape.to_lowercase() == "shapeless";
    
    // Convert pattern to ingredients array
    let expected_slots = match grid_size {
        GridSize::Small => 4,
        GridSize::Large => 9,
    };
    
    let mut ingredients = Vec::with_capacity(expected_slots);
    
    // Fill ingredients based on pattern
    if shapeless {
        // For shapeless, ingredients order doesn't matter
        for ing in &json.ingredients {
            ingredients.push(Some(RecipeIngredient {
                item: Identifier::parse(&ing.item).ok()?,
                count: ing.count,
                damage: ing.damage,
                is_tag: ing.is_tag,
            }));
        }
    } else {
        // For shaped, we need to place ingredients in the grid according to pattern
        let mut pattern_chars = std::collections::HashMap::new();
        let mut char_index = 0;
        
        for row in &json.pattern {
            for ch in row.chars() {
                if ch != ' ' && !pattern_chars.contains_key(&ch) {
                    if char_index < json.ingredients.len() {
                        pattern_chars.insert(ch, char_index);
                        char_index += 1;
                    }
                }
            }
        }
        
        // Build ingredients array in grid order
        for row in &json.pattern {
            for ch in row.chars() {
                if ch == ' ' {
                    ingredients.push(None);
                } else if let Some(&idx) = pattern_chars.get(&ch) {
                    if idx < json.ingredients.len() {
                        let ing = &json.ingredients[idx];
                        ingredients.push(Some(RecipeIngredient {
                            item: Identifier::parse(&ing.item).ok()?,
                            count: ing.count,
                            damage: ing.damage,
                            is_tag: ing.is_tag,
                        }));
                    } else {
                        ingredients.push(None);
                    }
                } else {
                    ingredients.push(None);
                }
            }
        }
        
        // Pad or truncate to expected size
        ingredients.resize(expected_slots, None);
    }
    
    // Parse result
    let result = RecipeResult {
        item: Identifier::parse(&json.result.item).ok()?,
        count: json.result.count,
        damage: json.result.damage,
        nbt: None,
    };
    
    Some(Recipe {
        id,
        group,
        grid_size,
        ingredients,
        result,
        shapeless,
    })
}

/// Register a recipe.
pub fn register_recipe(manager: &mut CraftingManager, recipe: Recipe) {
    manager.known_recipes.insert(recipe.id.clone(), recipe);
}

/// Check if a recipe is unlocked for the player.
pub fn is_recipe_unlocked(manager: &CraftingManager, recipe_id: &Identifier) -> bool {
    manager.recipe_book.unlocked.contains(recipe_id)
}

/// Unlock a recipe for the player.
pub fn unlock_recipe(manager: &mut CraftingManager, recipe_id: Identifier) {
    if !manager.recipe_book.unlocked.contains(&recipe_id) {
        manager.recipe_book.unlocked.push(recipe_id);
    }
}

/// Get craftable recipes for current grid contents.
pub fn get_craftable_recipes<'a>(
    manager: &'a CraftingManager,
    grid_size: GridSize,
    recipes: &'a [Recipe],
) -> Vec<&'a Recipe> {
    let grid: &[Option<crate::gameplay::inventory::ItemStack>] = match grid_size {
        GridSize::Small => &manager.small_grid,
        GridSize::Large => &manager.large_grid,
    };
    
    recipes.iter()
        .filter(|r| {
            r.grid_size == grid_size && 
            matches_recipe(grid, grid_size, r) &&
            is_recipe_unlocked(manager, &r.id)
        })
        .collect()
}