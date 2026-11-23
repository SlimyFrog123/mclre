//! Extractor for the reference recipe registration tables.
//!
//! The reference defines recipes in multiple files:
//! - `Recipes.cpp` — main recipes, shaped and shapeless
//! - `ToolRecipies.cpp` — tool recipes (pickaxes, shovels, axes, hoes)
//! - `WeaponRecipies.cpp` — weapon recipes (swords)
//! - `ArmorRecipes.cpp` — armor recipes
//! - `FoodRecipies.cpp` — food recipes
//! - `StructureRecipies.cpp` — structure recipes (blocks, workbench, etc.)
//! - `OreRecipies.cpp` — block-to-ingot and ingot-to-block recipes
//! - `ClothDyeRecipes.cpp` — dye recipes for wool, clay, glass
//!
//! This extractor parses all these files and outputs a unified TOML format
//! for parity testing and golden data generation.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

/// Recipe category (corresponds to LCE crafting menu groups).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum RecipeCategory {
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
}

/// Recipe shape type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RecipeShape {
    /// Shaped recipe (pattern matters).
    Shaped,
    /// Shapeless recipe (order doesn't matter).
    Shapeless,
    /// Furnace/smelting recipe.
    Furnace,
}

/// A single recipe ingredient.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RecipeIngredient {
    /// Item or block identifier (namespace:path).
    pub item: String,
    /// Count (default 1).
    pub count: u8,
    /// Damage/metadata value (for items with subtypes).
    pub damage: Option<u16>,
    /// Whether this is a tag reference (e.g., #minecraft:planks).
    pub is_tag: bool,
}

/// A recipe result.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RecipeResult {
    /// Output item identifier.
    pub item: String,
    /// Output count.
    pub count: u8,
    /// Output damage/metadata.
    pub damage: u16,
}

/// A parsed recipe from the reference.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LegacyRecipe {
    /// Unique recipe identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Recipe category/group.
    pub category: RecipeCategory,
    /// Recipe shape type.
    pub shape: RecipeShape,
    /// Grid width (for shaped recipes).
    pub width: u8,
    /// Grid height (for shaped recipes).
    pub height: u8,
    /// Pattern rows for shaped recipes (empty for shapeless).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pattern: Vec<String>,
    /// Ingredients in reading order (top-left to bottom-right).
    /// For shapeless, order doesn't matter.
    pub ingredients: Vec<RecipeIngredient>,
    /// Result item.
    pub result: RecipeResult,
    /// Source file where this recipe was found.
    pub source_file: String,
    /// Source line number (approximate).
    pub source_line: usize,
}

/// Recipe parser for the LCE reference format.
pub struct RecipeParser {
    recipes: Vec<LegacyRecipe>,
    current_id: u32,
    current_file: String,
    current_line: usize,
    /// Tracks seen recipe IDs to detect duplicates.
    seen_ids: HashSet<String>,
}

impl RecipeParser {
    /// Create a new recipe parser.
    pub fn new() -> Self {
        Self {
            recipes: Vec::new(),
            current_id: 0,
            current_file: String::new(),
            current_line: 0,
            seen_ids: HashSet::new(),
        }
    }

    /// Set the current source file context.
    pub fn set_source_file(&mut self, file: &str) {
        self.current_file = file.to_string();
        self.current_line = 0;
    }

    /// Parse a single source file.
    pub fn parse_file(&mut self, source: &str, file_name: &str) -> Result<(), String> {
        self.set_source_file(file_name);

        // Preprocess: remove comments and join continuation lines, track line numbers
        let (processed, line_map) = self.preprocess_source(source);

        // Find all recipe calls
        let calls = self.find_recipe_calls(&processed, &line_map);

        for (line_num, call) in calls {
            self.current_line = line_num;
            if call.contains("addShapedRecipy") {
                self.parse_shaped_recipe(&call)?;
            } else if call.contains("addShapelessRecipy") {
                self.parse_shapeless_recipe(&call)?;
            }
        }

        Ok(())
    }

    /// Preprocess source: strip comments, handle line continuations.
    /// Returns (processed_string, line_map) where line_map[i] = original line number for position i in processed string
    fn preprocess_source(&self, source: &str) -> (String, Vec<usize>) {
        let mut result = String::new();
        let mut line_map = Vec::new(); // Maps each char position in result to original line number
        let mut in_block_comment = false;

        for (line_num, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            let original_line = line_num + 1; // 1-based

            // Skip empty lines
            if trimmed.is_empty() {
                continue;
            }

            // Handle block comments
            if trimmed.starts_with("/*") {
                in_block_comment = true;
                if trimmed.ends_with("*/") {
                    in_block_comment = false;
                }
                continue;
            }
            if in_block_comment {
                if trimmed.ends_with("*/") {
                    in_block_comment = false;
                }
                continue;
            }

            // Skip line comments
            if trimmed.starts_with("//") {
                continue;
            }

            // Remove inline comments
            let line_content = if let Some(idx) = trimmed.find("//") {
                &trimmed[..idx]
            } else {
                trimmed
            };

            // Remove inline block comments /* ... */
            let mut clean_line = String::new();
            let mut in_inline_comment = false;
            let chars: Vec<char> = line_content.chars().collect();
            let mut i = 0;
            while i < chars.len() {
                if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' {
                    in_inline_comment = true;
                    i += 2;
                } else if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '/' {
                    in_inline_comment = false;
                    i += 2;
                } else if !in_inline_comment {
                    clean_line.push(chars[i]);
                    i += 1;
                } else {
                    i += 1;
                }
            }

            if !clean_line.trim().is_empty() {
                let start_pos = result.len();
                result.push_str(&clean_line);
                result.push(' ');
                // Map each character in the added portion to the original line number
                for _ in start_pos..result.len() {
                    line_map.push(original_line);
                }
            }
        }

        (result, line_map)
    }

    /// Find all recipe calls in the processed source.
    fn find_recipe_calls(&self, source: &str, line_map: &[usize]) -> Vec<(usize, String)> {
        let mut calls = Vec::new();
        let mut search_pos = 0;

        while let Some(idx) = source[search_pos..].find("addShapedRecipy") {
            let start = search_pos + idx;
            let call = self.extract_full_call(&source[start..]);

            // Skip function definitions (they contain "*result" or "...)")
            // Also skip function body calls where first arg is not a proper ItemInstance with "::"
            if call.contains("*result")
                || call.contains("...)")
                || !call.contains("new ItemInstance(")
                || !Self::looks_like_recipe_call(&call)
            {
                search_pos = start + call.len();
                continue;
            }

            // Get line number from line_map
            let line_num = if start < line_map.len() {
                line_map[start]
            } else {
                1
            };
            calls.push((line_num, call.clone()));
            search_pos = start + call.len();
        }

        search_pos = 0;
        while let Some(idx) = source[search_pos..].find("addShapelessRecipy") {
            let start = search_pos + idx;
            let call = self.extract_full_call(&source[start..]);

            // Skip function definitions and body calls
            if call.contains("*result")
                || call.contains("...)")
                || !call.contains("new ItemInstance(")
                || !Self::looks_like_recipe_call(&call)
            {
                search_pos = start + call.len();
                continue;
            }

            // Get line number from line_map
            let line_num = if start < line_map.len() {
                line_map[start]
            } else {
                1
            };
            calls.push((line_num, call.clone()));
            search_pos = start + call.len();
        }

        calls
    }

    /// Check if a call looks like a recipe call (first ItemInstance has proper type with ::)
    fn looks_like_recipe_call(call: &str) -> bool {
        // Find the first "new ItemInstance(" and check if the content has "::"
        if let Some(idx) = call.find("new ItemInstance(") {
            let after = &call[idx + 17..]; // skip "new ItemInstance("
            // Find the first argument (up to comma or closing paren), handling C++ cast expressions like "(Item *)"
            let mut depth = 0;
            let chars: Vec<char> = after.chars().collect();
            for (i, c) in chars.iter().enumerate() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        // Check if this is a cast closing "*)"
                        let mut is_cast_close = false;
                        if depth == 1 && i > 0 {
                            // Look back for "*" before this ")"
                            let mut j = i - 1;
                            while j > 0 && chars[j].is_whitespace() {
                                j -= 1;
                            }
                            if chars[j] == '*' {
                                is_cast_close = true;
                            }
                        }

                        if !is_cast_close {
                            if depth == 0 {
                                let first_arg = &after[..i];
                                return first_arg.contains("::");
                            }
                            depth -= 1;
                        }
                        // If it's a cast close, don't decrement depth
                    }
                    ',' if depth == 0 => {
                        let first_arg = &after[..i];
                        return first_arg.contains("::");
                    }
                    _ => {}
                }
            }
        }
        false
    }

    /// Extract a full function call (handles nested parentheses).
    fn extract_full_call(&self, source: &str) -> String {
        let mut result = String::new();
        let mut depth = 0;
        let mut found_paren = false;

        for c in source.chars() {
            result.push(c);
            match c {
                '(' => {
                    depth += 1;
                    found_paren = true;
                }
                ')' => {
                    depth -= 1;
                    if found_paren && depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }

        result.trim().to_string()
    }

    /// Parse a shaped recipe call.
    fn parse_shaped_recipe(&mut self, call: &str) -> Result<(), String> {
        self.parse_recipe(call, RecipeShape::Shaped)
    }

    /// Parse a shapeless recipe call.
    fn parse_shapeless_recipe(&mut self, call: &str) -> Result<(), String> {
        self.parse_recipe(call, RecipeShape::Shapeless)
    }

    /// Parse a recipe call (shaped or shapeless).
    fn parse_recipe(&mut self, call: &str, shape: RecipeShape) -> Result<(), String> {
        // DEBUG
        if call.contains("Item::bow") && call.contains("ssscicig") {
            eprintln!(
                "DEBUG BOW RECIPE: call = {}",
                &call[..std::cmp::min(300, call.len())]
            );
        }
        // Extract result (first ItemInstance)
        let result = self.extract_first_item_instance(call)?;
        if call.contains("Item::bow") && call.contains("ssscicig") {
            eprintln!("DEBUG BOW RECIPE: result = {:?}", result);
        }

        // Extract pattern strings (for shaped recipes)
        let (width, height, pattern) = if shape == RecipeShape::Shaped {
            self.extract_pattern(call)?
        } else {
            (0, 0, Vec::new())
        };

        // Extract character-to-item mappings
        let char_mappings = self.extract_char_mappings(call)?;

        // Build ingredients from pattern and mappings
        let ingredients = if shape == RecipeShape::Shaped && !pattern.is_empty() {
            self.build_ingredients_from_pattern(&pattern, &char_mappings)?
        } else {
            // For shapeless, extract all ItemInstances after the first
            self.extract_remaining_item_instances(call)?
        };

        // Determine category from source file
        let category = self.infer_category();

        // Generate recipe ID
        self.current_id += 1;
        let id = format!("recipe_{:04}", self.current_id);
        let name = self.generate_recipe_name(&result.item);

        let recipe = LegacyRecipe {
            id: id.clone(),
            name,
            category,
            shape,
            width,
            height,
            pattern: if shape == RecipeShape::Shaped {
                pattern.clone()
            } else {
                Vec::new()
            },
            ingredients,
            result,
            source_file: self.current_file.clone(),
            source_line: self.current_line,
        };

        // Check for duplicates
        if !self.seen_ids.insert(id) {
            return Err(format!("Duplicate recipe ID"));
        }

        self.recipes.push(recipe);
        Ok(())
    }

    /// Extract the first ItemInstance (the result).
    fn extract_first_item_instance(&self, call: &str) -> Result<RecipeResult, String> {
        // Find first "new ItemInstance(" or "new ItemInstance ("
        let patterns = ["new ItemInstance(", "new ItemInstance ("];
        let mut earliest = None;
        let mut pattern_used = "";

        for pat in &patterns {
            if let Some(idx) = call.find(pat) {
                if earliest.is_none() || idx < earliest.unwrap() {
                    earliest = Some(idx);
                    pattern_used = *pat;
                }
            }
        }

        let start = earliest.ok_or("No ItemInstance found")?;
        let paren_start = start + pattern_used.len();

        // Find matching closing parenthesis, handling C++ cast expressions like "(Item *)"
        // The cast has its own parens: new ItemInstance((Type *)Item::name, count)
        // When we see a ")" that would close the ItemInstance (depth==1),
        // check if it's actually a cast closing "*)" - if so, skip it.
        let after_instance = &call[paren_start..];
        let mut depth = 1;
        let mut end = 0;
        let chars: Vec<char> = after_instance.chars().collect();

        eprintln!(
            "DEBUG extract_first_item_instance: after_instance = {:?}",
            &after_instance[..std::cmp::min(100, after_instance.len())]
        );

        for (i, c) in chars.iter().enumerate() {
            match c {
                '(' => depth += 1,
                ')' => {
                    // Check if this is a cast closing "*)"
                    // Look back for "*" before this ")"
                    let mut is_cast_close = false;
                    if depth == 1 && i > 0 {
                        // Look back for "*" skipping whitespace
                        let mut j = i - 1;
                        while j > 0 && chars[j].is_whitespace() {
                            j -= 1;
                        }
                        if chars[j] == '*' {
                            is_cast_close = true;
                            eprintln!(
                                "DEBUG extract_first_item_instance: cast close at i={}, depth={}",
                                i, depth
                            );
                        }
                    }

                    if !is_cast_close {
                        depth -= 1;
                        if depth == 0 {
                            end = i + 1;
                            eprintln!(
                                "DEBUG extract_first_item_instance: found end at i={}, end={}",
                                i, end
                            );
                            break;
                        }
                    } else {
                        eprintln!(
                            "DEBUG extract_first_item_instance: skipping cast close at i={}, depth stays={}",
                            i, depth
                        );
                    }
                    // If it's a cast close, don't decrement depth (the cast's opening paren
                    // was already counted when we saw the "(" after ItemInstance opening)
                }
                _ => {}
            }
        }

        if depth != 0 {
            return Err("Unmatched parentheses in ItemInstance".to_string());
        }

        let instance_text = &call[start..paren_start + end];
        eprintln!(
            "DEBUG extract_first_item_instance: instance_text = {:?}",
            instance_text
        );
        self.parse_item_instance(instance_text)
    }

    /// Parse a single ItemInstance string into a RecipeResult.
    fn parse_item_instance(&self, text: &str) -> Result<RecipeResult, String> {
        // Extract the first ItemInstance from the text (text may contain trailing garbage)
        // Find the first "new ItemInstance(" and extract up to its matching closing parenthesis
        let text = text.trim();
        let instance_text = if let Some(idx) = text.find("new ItemInstance(") {
            let start = idx;
            // Use the same cast-aware depth counting as extract_first_item_instance
            let after_instance = &text[idx + 17..]; // skip "new ItemInstance("
            let mut depth = 1;
            let mut end = 0;
            let chars: Vec<char> = after_instance.chars().collect();

            for (i, c) in chars.iter().enumerate() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        // Check if this is a cast closing "*)"
                        let mut is_cast_close = false;
                        if depth == 1 && i > 0 {
                            let mut j = i - 1;
                            while j > 0 && chars[j].is_whitespace() {
                                j -= 1;
                            }
                            if chars[j] == '*' {
                                is_cast_close = true;
                            }
                        }

                        if !is_cast_close {
                            depth -= 1;
                            if depth == 0 {
                                end = i + 1;
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }

            if depth == 0 {
                &text[start..start + 17 + end]
            } else {
                text
            }
        } else {
            text
        };

        // Remove "new ItemInstance(" prefix and trailing ")"
        let after_prefix = instance_text
            .trim_start_matches("new ItemInstance(")
            .trim_start_matches("new ItemInstance (");
        let mut content = after_prefix.trim_end_matches(')').trim(); // Trim whitespace

        // DEBUG
        let is_bow = content.contains("Item::bow") || content.contains("(Item");

        // Handle C++ cast expressions like "(Item *)Item::bow" at the start of content
        // These confuse the argument splitter because the "*)" looks like a closing paren
        if let Some(stripped) = content.strip_prefix('(') {
            // Look for "*)" pattern which indicates a cast
            if let Some(cast_end) = stripped.find("*)") {
                // Skip the cast and return the rest
                content = &stripped[cast_end + 2..].trim_start();
            }
        }

        // Parse arguments with fallback for simple comma-separated values
        let parts = self.split_args(content)?;
        if parts.is_empty() {
            return Err("Invalid ItemInstance format".to_string());
        }

        let item_str = parts[0].trim();
        let count = if parts.len() > 1 {
            parts[1].trim().parse::<u8>().unwrap_or(1)
        } else {
            1
        };
        let damage = if parts.len() > 2 {
            parts[2].trim().parse::<u16>().unwrap_or(0)
        } else {
            0
        };

        let item = self.cpp_id_to_namespaced(item_str);

        Ok(RecipeResult {
            item,
            count,
            damage,
        })
    }

    /// Split arguments respecting nested parentheses, with fallback for simple comma-separated values.
    fn split_args(&self, content: &str) -> Result<Vec<String>, String> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut depth = 0;

        for c in content.chars() {
            match c {
                '(' => {
                    depth += 1;
                    current.push(c);
                }
                ')' => {
                    if depth > 0 {
                        depth -= 1;
                    }
                    current.push(c);
                }
                ',' if depth == 0 => {
                    parts.push(current.trim().trim_end_matches(')').trim().to_string());
                    current.clear();
                }
                _ => {
                    current.push(c);
                }
            }
        }

        if !current.trim().is_empty() {
            parts.push(current.trim().trim_end_matches(')').trim().to_string());
        }

        Ok(parts)
    }

    /// Convert C++ identifier to namespaced ID.
    fn cpp_id_to_namespaced(&self, cpp_id: &str) -> String {
        let id = cpp_id.trim();

        // Handle cast expressions like `(Item *)Item::bow`
        let id = if let Some(idx) = id.rfind("::") {
            &id[idx + 2..]
        } else {
            id
        };

        // Determine namespace and path
        if id.starts_with("Tile::") {
            let path = &id[6..];
            format!("minecraft:{}", self.cpp_to_snake_case(path))
        } else if id.starts_with("Item::") {
            let path = &id[6..];
            format!("minecraft:{}", self.cpp_to_snake_case(path))
        } else if id.starts_with("DyePowderItem::") {
            let path = &id[15..];
            format!("minecraft:{}", self.cpp_to_snake_case(path))
        } else if id.starts_with("ColoredTile::") {
            let path = &id[13..];
            format!("minecraft:{}", self.cpp_to_snake_case(path))
        } else if id.starts_with("TreeTile::") {
            let path = &id[10..];
            format!("minecraft:{}", self.cpp_to_snake_case(path))
        } else if id.starts_with("StoneSlabTile::") {
            let path = &id[15..];
            format!("minecraft:{}", self.cpp_to_snake_case(path))
        } else if id.starts_with("QuartzBlockTile::") {
            let path = &id[17..];
            format!("minecraft:{}", self.cpp_to_snake_case(path))
        } else if id.starts_with("SandStoneTile::") {
            let path = &id[15..];
            format!("minecraft:{}", self.cpp_to_snake_case(path))
        } else if id.starts_with("WallTile::") {
            let path = &id[10..];
            format!("minecraft:{}", self.cpp_to_snake_case(path))
        } else if id.starts_with("CoalItem::") {
            let path = &id[10..];
            format!("minecraft:{}", self.cpp_to_snake_case(path))
        } else {
            // Fallback
            format!("minecraft:{}", self.cpp_to_snake_case(id))
        }
    }

    /// Convert CamelCase to snake_case.
    fn cpp_to_snake_case(&self, s: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = s.chars().collect();
        for (i, c) in chars.iter().enumerate() {
            if c.is_uppercase() {
                if i > 0 && (chars[i - 1].is_lowercase() || chars[i - 1].is_numeric()) {
                    result.push('_');
                }
                result.push(c.to_lowercase().next().unwrap());
            } else {
                result.push(*c);
            }
        }
        result
    }

    /// Extract pattern from shaped recipe call.
    /// Pattern rows are the L"..." strings between the group string and L'X' char mappings.
    fn extract_pattern(&self, call: &str) -> Result<(u8, u8, Vec<String>), String> {
        // Find all L"..." and L'X' positions
        let mut l_strings = Vec::new(); // (start, end, content)
        let mut l_char_positions = Vec::new(); // positions of L'X'
        let call_bytes = call.as_bytes();

        let mut pos = 0;
        while let Some(idx) = call[pos..].find('L') {
            let start = pos + idx;
            if start + 1 < call.len() && &call[start + 1..start + 2] == "\"" {
                // L"..." string
                let quote_start = start + 2;
                if let Some(quote_end) = call[quote_start..].find('"') {
                    let content = &call[quote_start..quote_start + quote_end];
                    l_strings.push((start, quote_end + quote_start + 1, content.to_string()));
                    pos = quote_start + quote_end + 1;
                } else {
                    pos = start + 1;
                }
            } else if start + 4 < call.len()
                && &call[start..start + 2] == "L'"
                && call_bytes[start + 3] == b'\''
                && call_bytes[start + 4] == b','
            {
                // L'X', pattern
                let ch_pos = start;
                l_char_positions.push(ch_pos);
                pos = start + 1;
            } else {
                pos = start + 1;
            }
        }

        // Pattern rows are L"..." strings that come after the first one (group string)
        // and before the first L'X' char mapping
        let first_char_mapping = l_char_positions.first().copied().unwrap_or(usize::MAX);

        let mut pattern = Vec::new();
        for (i, (start, end, content)) in l_strings.iter().enumerate() {
            // Skip the first L"..." (group string)
            if i == 0 {
                continue;
            }
            // Stop at first L'X' char mapping
            if *start >= first_char_mapping {
                break;
            }
            if !content.is_empty() && content.chars().any(|c| c != ' ' && c != '\t') {
                pattern.push(content.to_string());
            }
        }

        if pattern.is_empty() {
            return Err("No pattern found".to_string());
        }

        let height = pattern.len() as u8;
        let width = pattern.iter().map(|p| p.len() as u8).max().unwrap_or(0);
        Ok((width, height, pattern))
    }

    /// Extract character-to-item mappings from recipe call.
    /// Format: L'#', new ItemInstance(Tile::treeTrunk, 1, 0) or L'#', Tile::wood
    fn extract_char_mappings(&self, call: &str) -> Result<HashMap<char, String>, String> {
        let mut mappings = HashMap::new();
        let mut pos = 0;
        let call_bytes = call.as_bytes();

        while let Some(idx) = call[pos..].find("L'") {
            let start = pos + idx;
            // Check for L'X', pattern (5 chars: L ' X ' ,)
            // L'X', = L ' X ' , (5 chars: L ' X ' ,)
            if start + 4 < call_bytes.len()
                && call_bytes[start] == b'L'
                && call_bytes[start + 1] == b'\''
                && call_bytes[start + 3] == b'\''
                && call_bytes[start + 4] == b','
            {
                let ch = call[start + 2..start + 3].chars().next().unwrap_or('?');
                let after = &call[start + 5..].trim_start();

                // Try to find the associated ItemInstance or Tile::/Item::
                if after.starts_with("new ItemInstance(") {
                    // Find the full ItemInstance
                    let mut depth = 1;
                    let mut end = start + 5 + 17; // "new ItemInstance("
                    for (i, c) in call[end..].char_indices() {
                        match c {
                            '(' => depth += 1,
                            ')' => {
                                depth -= 1;
                                if depth == 0 {
                                    end += i + 1;
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    if depth == 0 {
                        let instance_text = &call[start + 5..end];
                        if let Ok(result) = self.parse_item_instance(instance_text) {
                            mappings.insert(ch, result.item);
                        }
                    }
                } else if after.starts_with("Tile::") || after.starts_with("Item::") {
                    // Simple reference like Tile::wood
                    let end_idx = after.find(',').unwrap_or(after.len());
                    let item_part = &after[..end_idx].trim();
                    if item_part.starts_with("Tile::") || item_part.starts_with("Item::") {
                        let item = self.cpp_id_to_namespaced(item_part);
                        mappings.insert(ch, item);
                    }
                }
                pos = start + 1;
            } else {
                pos = start + 1;
            }
        }

        Ok(mappings)
    }

    /// Extract all remaining ItemInstances after the first (for shapeless recipes).
    fn extract_remaining_item_instances(
        &self,
        call: &str,
    ) -> Result<Vec<RecipeIngredient>, String> {
        let mut ingredients = Vec::new();
        let mut search_pos = 0;
        let mut instance_count = 0;

        while let Some(idx) = call[search_pos..].find("ItemInstance") {
            let start = search_pos + idx;
            // Check if this is "new ItemInstance("
            let after = &call[start..];
            if !after.starts_with("new ItemInstance(") && !after.starts_with("new ItemInstance (") {
                search_pos = start + 1;
                continue;
            }

            // Find the opening parenthesis
            let paren_idx = after.find('(').ok_or("No parenthesis")?;
            let paren_start = start + paren_idx + 1;

            // Find matching closing parenthesis
            let mut depth = 1;
            let mut end = paren_start;
            for (i, c) in call[paren_start..].char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = paren_start + i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if depth != 0 {
                search_pos = start + 1;
                continue;
            }

            instance_count += 1;
            // Skip the first instance (result)
            if instance_count > 1 {
                let instance_text = &call[start..end];
                if let Ok(result) = self.parse_item_instance(instance_text) {
                    ingredients.push(RecipeIngredient {
                        item: result.item,
                        count: result.count,
                        damage: if result.damage > 0 {
                            Some(result.damage)
                        } else {
                            None
                        },
                        is_tag: false,
                    });
                }
            }

            search_pos = end;
        }

        Ok(ingredients)
    }
    fn build_ingredients_from_pattern(
        &self,
        pattern: &[String],
        char_mappings: &HashMap<char, String>,
    ) -> Result<Vec<RecipeIngredient>, String> {
        let mut ingredients = Vec::new();

        for row in pattern {
            for ch in row.chars() {
                if ch != ' ' && ch != '\t' {
                    if let Some(item) = char_mappings.get(&ch) {
                        ingredients.push(RecipeIngredient {
                            item: item.clone(),
                            count: 1,
                            damage: None,
                            is_tag: false,
                        });
                    }
                }
            }
        }

        Ok(ingredients)
    }

    /// Infer recipe category from source file.
    fn infer_category(&self) -> RecipeCategory {
        if self.current_file.contains("ToolRecipies")
            || self.current_file.contains("WeaponRecipies")
        {
            RecipeCategory::Tools
        } else if self.current_file.contains("ArmorRecipes") {
            RecipeCategory::Combat
        } else if self.current_file.contains("FoodRecipies") {
            RecipeCategory::Food
        } else if self.current_file.contains("StructureRecipies") {
            RecipeCategory::BuildingBlocks
        } else if self.current_file.contains("OreRecipies") {
            RecipeCategory::Materials
        } else if self.current_file.contains("ClothDyeRecipes") {
            RecipeCategory::Decorations
        } else if self.current_file.contains("Fireworks") {
            RecipeCategory::Brewing
        } else {
            RecipeCategory::Miscellaneous
        }
    }

    /// Generate a human-readable name from result item.
    fn generate_recipe_name(&self, item: &str) -> String {
        item.trim_start_matches("minecraft:").replace('_', " ")
    }

    /// Get all parsed recipes.
    pub fn recipes(&self) -> &[LegacyRecipe] {
        &self.recipes
    }

    /// Render recipes as deterministic TOML.
    pub fn render_toml(&self, source_label: &str) -> String {
        let mut out = String::new();
        out.push_str(
            "# Generated by `cargo run -p mclre-devtools --bin extract-legacy-recipes`.\n",
        );
        out.push_str("# Do not edit by hand: regenerate from the reference source.\n");
        let _ = writeln!(out, "# Source: {source_label}");
        let _ = writeln!(out, "\nrecipe_count = {}", self.recipes.len());

        for recipe in &self.recipes {
            out.push_str("\n[[recipe]]\n");
            let _ = writeln!(out, "id = \"{}\"", recipe.id);
            let _ = writeln!(out, "name = \"{}\"", recipe.name);
            let _ = writeln!(out, "category = \"{:?}\"", recipe.category);
            let _ = writeln!(out, "shape = \"{:?}\"", recipe.shape);
            if recipe.shape == RecipeShape::Shaped {
                let _ = writeln!(out, "width = {}", recipe.width);
                let _ = writeln!(out, "height = {}", recipe.height);
            }
            let _ = writeln!(out, "source_file = \"{}\"", recipe.source_file);
            let _ = writeln!(out, "source_line = {}", recipe.source_line);

            out.push_str("ingredients = [\n");
            for ing in &recipe.ingredients {
                out.push_str("  { ");
                let _ = writeln!(
                    out,
                    "item = \"{}\", count = {}, damage = {:?}, is_tag = {} }},",
                    ing.item, ing.count, ing.damage, ing.is_tag
                );
                out.push_str("  },\n");
            }
            out.push_str("]\n");

            let _ = writeln!(
                out,
                "result = {{ item = \"{}\", count = {}, damage = {} }}",
                recipe.result.item, recipe.result.count, recipe.result.damage
            );
        }
        out
    }
}

impl Default for RecipeParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse all recipe files from the reference source.
pub fn parse_all_recipe_files(
    recipes_cpp: &str,
    tool_recipes_cpp: &str,
    weapon_recipes_cpp: &str,
    armor_recipes_cpp: &str,
    food_recipes_cpp: &str,
    structure_recipes_cpp: &str,
    ore_recipes_cpp: &str,
    cloth_dye_recipes_cpp: &str,
) -> Result<Vec<LegacyRecipe>, String> {
    let mut parser = RecipeParser::new();

    parser.parse_file(recipes_cpp, "Recipes.cpp")?;
    parser.parse_file(tool_recipes_cpp, "ToolRecipies.cpp")?;
    parser.parse_file(weapon_recipes_cpp, "WeaponRecipies.cpp")?;
    parser.parse_file(armor_recipes_cpp, "ArmorRecipes.cpp")?;
    parser.parse_file(food_recipes_cpp, "FoodRecipies.cpp")?;
    parser.parse_file(structure_recipes_cpp, "StructureRecipies.cpp")?;
    parser.parse_file(ore_recipes_cpp, "OreRecipies.cpp")?;
    parser.parse_file(cloth_dye_recipes_cpp, "ClothDyeRecipes.cpp")?;

    Ok(parser.recipes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpp_to_namespaced() {
        let parser = RecipeParser::new();
        assert_eq!(parser.cpp_id_to_namespaced("Tile::wood"), "minecraft:wood");
        assert_eq!(
            parser.cpp_id_to_namespaced("Item::stick"),
            "minecraft:stick"
        );
        assert_eq!(
            parser.cpp_id_to_namespaced("Tile::treeTrunk"),
            "minecraft:tree_trunk"
        );
        assert_eq!(
            parser.cpp_id_to_namespaced("DyePowderItem::BLUE"),
            "minecraft:blue"
        );
    }

    #[test]
    fn parses_shaped_recipe_simple() {
        let mut parser = RecipeParser::new();
        let call = "addShapedRecipy(new ItemInstance(Tile::wood, 4, 0), L\"sczg\", L\"#\", L'#', new ItemInstance(Tile::treeTrunk, 1, 0), L'S')";
        parser
            .parse_recipe(call, RecipeShape::Shaped)
            .expect("parses");
        let recipes = parser.recipes();
        assert_eq!(recipes.len(), 1);
        let r = &recipes[0];
        assert_eq!(r.shape, RecipeShape::Shaped);
        assert_eq!(r.result.item, "minecraft:wood");
        assert_eq!(r.result.count, 4);
        assert!(!r.ingredients.is_empty());
    }

    #[test]
    fn parses_shapeless_recipe_simple() {
        let mut parser = RecipeParser::new();
        let call = "addShapelessRecipy(new ItemInstance(Item::mushroomStew), L\"ttig\", Tile::mushroom_brown, Tile::mushroom_red, Item::bowl, L'F')";
        parser
            .parse_recipe(call, RecipeShape::Shapeless)
            .expect("parses");
        let recipes = parser.recipes();
        assert_eq!(recipes.len(), 1);
        let r = &recipes[0];
        assert_eq!(r.shape, RecipeShape::Shapeless);
        assert_eq!(r.result.item, "minecraft:mushroom_stew");
    }
}
