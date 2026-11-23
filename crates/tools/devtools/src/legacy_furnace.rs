//! Extractor for the reference furnace (smelting) recipes.
//!
//! The reference defines furnace recipes in `FurnaceRecipes.cpp` with calls like:
//! ```cpp
//! addFurnaceRecipy(Tile::ironOre_Id, new ItemInstance(Item::ironIngot), .7f);
//! ```
//!
//! This extractor parses those into a TOML format for parity testing.

use std::fmt::Write as _;

/// A furnace (smelting) recipe from the reference.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LegacyFurnaceRecipe {
    /// Input item/block identifier.
    pub input: String,
    /// Output item.
    pub output: String,
    /// Output count.
    pub output_count: u8,
    /// Output damage/metadata.
    pub output_damage: u16,
    /// Experience value (float).
    pub experience: f32,
    /// Source file.
    pub source_file: String,
    /// Source line number.
    pub source_line: usize,
}

/// Furnace recipe parser.
pub struct FurnaceRecipeParser {
    recipes: Vec<LegacyFurnaceRecipe>,
    current_file: String,
    current_line: usize,
}

impl FurnaceRecipeParser {
    /// Create a new furnace recipe parser.
    pub fn new() -> Self {
        Self {
            recipes: Vec::new(),
            current_file: String::new(),
            current_line: 0,
        }
    }

    /// Set the current source file context.
    pub fn set_source_file(&mut self, file: &str) {
        self.current_file = file.to_string();
        self.current_line = 0;
    }

    /// Parse a source file containing furnace recipes.
    pub fn parse_file(&mut self, source: &str, file_name: &str) -> Result<(), String> {
        self.set_source_file(file_name);
        let lines: Vec<&str> = source.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            self.current_line = i + 1;
            let trimmed = line.trim();

            // Look for addFurnaceRecipy calls
            if trimmed.contains("addFurnaceRecipy") {
                // Skip function signatures (they contain parameter types)
                if trimmed.contains("int itemId")
                    || trimmed.contains("ItemInstance *result")
                    || trimmed.contains("float value")
                {
                    continue;
                }
                // Skip lines that don't look like actual calls (no "new ItemInstance")
                if !trimmed.contains("new ItemInstance") {
                    continue;
                }
                self.parse_furnace_call(trimmed)?;
            }
        }

        Ok(())
    }

    /// Parse a single furnace recipe call.
    fn parse_furnace_call(&mut self, call: &str) -> Result<(), String> {
        // Format: addFurnaceRecipy(Tile::ironOre_Id, new ItemInstance(Item::ironIngot), .7f);
        // Or: addFurnaceRecipy(Item::porkChop_raw_Id, new ItemInstance(Item::porkChop_cooked), .35f);

        // Extract content between parentheses
        let start = call.find('(').ok_or("No opening parenthesis")? + 1;
        let end = call.rfind(')').ok_or("No closing parenthesis")?;
        let content = &call[start..end];

        // Split by comma (but respect parentheses)
        let parts = self.split_args(content)?;
        if parts.len() < 3 {
            return Err("Insufficient arguments".to_string());
        }

        // Parse input ID (first argument)
        let input_raw = parts[0].trim();
        let input = self.parse_input_id(input_raw)?;

        // Parse output ItemInstance (second argument)
        let output_raw = parts[1].trim();
        let (output, output_count, output_damage) = self.parse_output_item_instance(output_raw)?;

        // Parse experience (third argument)
        let exp_raw = parts[2].trim();
        let experience = exp_raw
            .trim_end_matches(['f', 'F'])
            .parse::<f32>()
            .map_err(|_| format!("Invalid experience value: {}", exp_raw))?;

        let recipe = LegacyFurnaceRecipe {
            input,
            output,
            output_count,
            output_damage,
            experience,
            source_file: self.current_file.clone(),
            source_line: self.current_line,
        };

        self.recipes.push(recipe);
        Ok(())
    }

    /// Split arguments respecting nested parentheses.
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
                    depth -= 1;
                    current.push(c);
                }
                ',' if depth == 0 => {
                    parts.push(current.trim().to_string());
                    current.clear();
                }
                _ => {
                    current.push(c);
                }
            }
        }

        if !current.is_empty() {
            parts.push(current.trim().to_string());
        }

        Ok(parts)
    }

    /// Parse input ID like `Tile::ironOre_Id` or `Item::porkChop_raw_Id`.
    fn parse_input_id(&self, raw: &str) -> Result<String, String> {
        // Remove _Id suffix
        let id = raw.trim_end_matches("_Id");

        // Convert to namespaced ID
        let result = if id.starts_with("Tile::") {
            let path = &id[6..];
            format!("minecraft:{}", self.cpp_to_snake_case(path))
        } else if id.starts_with("Item::") {
            let path = &id[6..];
            format!("minecraft:{}", self.cpp_to_snake_case(path))
        } else {
            format!("minecraft:{}", self.cpp_to_snake_case(id))
        };

        Ok(result)
    }

    /// Parse output ItemInstance.
    fn parse_output_item_instance(&self, raw: &str) -> Result<(String, u8, u16), String> {
        // Format: new ItemInstance(Item::ironIngot) or new ItemInstance(Item::coal, 1, CoalItem::CHAR_COAL)
        let content = raw
            .trim_start_matches("new ItemInstance(")
            .trim_end_matches(')');

        let parts: Vec<&str> = content.split(',').map(|s| s.trim()).collect();
        if parts.is_empty() {
            return Err("Invalid output ItemInstance".to_string());
        }

        let item_raw = parts[0];
        let count = if parts.len() > 1 {
            parts[1].parse::<u8>().unwrap_or(1)
        } else {
            1
        };
        let damage = if parts.len() > 2 {
            parts[2].parse::<u16>().unwrap_or(0)
        } else {
            0
        };

        let item = if item_raw.starts_with("Tile::") {
            let path = &item_raw[6..];
            format!("minecraft:{}", self.cpp_to_snake_case(path))
        } else if item_raw.starts_with("Item::") {
            let path = &item_raw[6..];
            format!("minecraft:{}", self.cpp_to_snake_case(path))
        } else {
            format!("minecraft:{}", self.cpp_to_snake_case(item_raw))
        };

        Ok((item, count, damage))
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

    /// Get all parsed recipes.
    pub fn recipes(&self) -> &[LegacyFurnaceRecipe] {
        &self.recipes
    }

    /// Render as deterministic TOML.
    pub fn render_toml(&self, source_label: &str) -> String {
        let mut out = String::new();
        out.push_str(
            "# Generated by `cargo run -p mclre-devtools --bin extract-legacy-furnace`.\n",
        );
        out.push_str("# Do not edit by hand: regenerate from the reference source.\n");
        let _ = writeln!(out, "# Source: {source_label}");
        let _ = writeln!(out, "\nfurnace_recipe_count = {}", self.recipes.len());

        for recipe in &self.recipes {
            out.push_str("\n[[furnace_recipe]]\n");
            let _ = writeln!(out, "input = \"{}\"", recipe.input);
            let _ = writeln!(out, "output = \"{}\"", recipe.output);
            let _ = writeln!(out, "output_count = {}", recipe.output_count);
            let _ = writeln!(out, "output_damage = {}", recipe.output_damage);
            let _ = writeln!(out, "experience = {}", recipe.experience);
            let _ = writeln!(out, "source_file = \"{}\"", recipe.source_file);
            let _ = writeln!(out, "source_line = {}", recipe.source_line);
        }
        out
    }
}

impl Default for FurnaceRecipeParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse furnace recipes from the reference source.
pub fn parse_furnace_recipes(source: &str) -> Result<Vec<LegacyFurnaceRecipe>, String> {
    let mut parser = FurnaceRecipeParser::new();
    parser.parse_file(source, "FurnaceRecipes.cpp")?;
    Ok(parser.recipes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FURNACE_FIXTURE: &str = r#"
        addFurnaceRecipy(Tile::ironOre_Id, new ItemInstance(Item::ironIngot), .7f);
        addFurnaceRecipy(Item::porkChop_raw_Id, new ItemInstance(Item::porkChop_cooked), .35f);
        addFurnaceRecipy(Tile::coalOre_Id, new ItemInstance(Item::coal), .1f);
    "#;

    #[test]
    fn parses_furnace_recipes() {
        let recipes = parse_furnace_recipes(FURNACE_FIXTURE).expect("parses");
        assert_eq!(recipes.len(), 3);

        let iron = &recipes[0];
        assert_eq!(iron.input, "minecraft:iron_ore");
        assert_eq!(iron.output, "minecraft:iron_ingot");
        assert_eq!(iron.output_count, 1);
        assert_eq!(iron.experience, 0.7);

        let pork = &recipes[1];
        assert_eq!(pork.input, "minecraft:pork_chop_raw");
        assert_eq!(pork.output, "minecraft:pork_chop_cooked");
        assert_eq!(pork.experience, 0.35);
    }

    #[test]
    fn cpp_to_snake_case() {
        let parser = FurnaceRecipeParser::new();
        assert_eq!(parser.cpp_to_snake_case("IronOre"), "iron_ore");
        assert_eq!(parser.cpp_to_snake_case("PorkChopRaw"), "pork_chop_raw");
        assert_eq!(parser.cpp_to_snake_case("CoalOre"), "coal_ore");
    }
}
