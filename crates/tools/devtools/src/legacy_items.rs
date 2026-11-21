//! Extractor for the reference item definitions.
//!
//! The reference defines items in `Item.cpp` with static instances and properties.
//! This extractor parses item IDs, names, max stack sizes, max damage, food properties, etc.

use std::fmt::Write as _;

/// Item properties from the reference.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LegacyItem {
    /// Legacy numeric item ID.
    pub id: u32,
    /// Reference symbol name (e.g., `ironIngot`, `sword_wood`).
    pub symbol: String,
    /// Human-readable name.
    pub name: String,
    /// Maximum stack size.
    pub max_stack_size: u8,
    /// Maximum damage (durability) - 0 if not damageable.
    pub max_damage: u16,
    /// Food properties (if edible).
    pub food: Option<LegacyFoodProperties>,
    /// Tool properties (if a tool).
    pub tool: Option<LegacyToolProperties>,
    /// Armor properties (if armor).
    pub armor: Option<LegacyArmorProperties>,
    /// Whether this item can be enchanted.
    pub enchantable: bool,
    /// Repair items.
    pub repair_items: Vec<String>,
    /// Source file.
    pub source_file: String,
    /// Source line number.
    pub source_line: usize,
}

/// Food properties for edible items.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LegacyFoodProperties {
    /// Hunger restored (food level).
    pub nutrition: u8,
    /// Saturation modifier.
    pub saturation: f32,
    /// Whether this is a "fast food" (eaten quickly).
    pub fast_eat: bool,
    /// Whether this is meat (wolves can eat).
    pub meat: bool,
    /// Always edible even when full.
    pub always_edible: bool,
}

/// Tool properties.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LegacyToolProperties {
    /// Tool tier (wood, stone, iron, gold, diamond).
    pub tier: String,
    /// Maximum damage (durability).
    pub max_damage: u16,
    /// Mining speed multiplier.
    pub mining_speed: f32,
    /// Attack damage bonus.
    pub attack_damage: f32,
    /// Harvest level (0=wood, 1=stone, 2=iron, 3=diamond).
    pub harvest_level: u8,
    /// Blocks this tool is effective on.
    pub effective_on: Vec<String>,
}

/// Armor properties.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LegacyArmorProperties {
    /// Armor type (helmet, chestplate, leggings, boots).
    pub armor_type: String,
    /// Maximum damage (durability).
    pub max_damage: u16,
    /// Protection amount (armor points).
    pub protection: u8,
    /// Toughness.
    pub toughness: f32,
    /// Knockback resistance.
    pub knockback_resistance: f32,
    /// Armor material.
    pub material: String,
}

/// Item parser for the LCE reference format.
pub struct ItemParser {
    items: Vec<LegacyItem>,
    current_file: String,
    current_line: usize,
}

impl ItemParser {
    /// Create a new item parser.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            current_file: String::new(),
            current_line: 0,
        }
    }

    /// Set the current source file context.
    pub fn set_source_file(&mut self, file: &str) {
        self.current_file = file.to_string();
        self.current_line = 0;
    }

    /// Parse a source file containing item definitions.
    pub fn parse_file(&mut self, source: &str, file_name: &str) -> Result<(), String> {
        self.set_source_file(file_name);
        let lines: Vec<&str> = source.lines().collect();
        
        // The reference typically has Item.cpp with static instances like:
        // Item *Item::ironIngot = NULL;
        // And then in staticCtor:
        // Item::ironIngot = (new Item(265))->setMaxStackSize(64)->setIconName(L"iron_ingot");
        
        for (i, line) in lines.iter().enumerate() {
            self.current_line = i + 1;
            let trimmed = line.trim();
            
            // Look for Item instance assignments in staticCtor
            if trimmed.contains("Item::") && trimmed.contains("new Item(") && trimmed.contains("->") {
                self.parse_item_assignment(trimmed)?;
            }
        }
        
        Ok(())
    }

    /// Parse an item assignment like `Item::ironIngot = (new Item(265))->setMaxStackSize(64)->setIconName(L"iron_ingot");`
    fn parse_item_assignment(&mut self, line: &str) -> Result<(), String> {
        // Extract symbol (e.g., "ironIngot" from "Item::ironIngot = ")
        let symbol = if let Some(eq_pos) = line.find('=') {
            let left = &line[..eq_pos].trim();
            if let Some(colon_pos) = left.rfind("::") {
                left[colon_pos + 2..].trim().to_string()
            } else {
                return Ok(()); // Not an item assignment we can parse
            }
        } else {
            return Ok(());
        };

        // Extract numeric ID from `new Item(265)`
        let id = if let Some(paren_start) = line.find("new Item(") {
            let start = paren_start + 9; // "new Item("
            let end = line[start..].find(')').map(|e| start + e).unwrap_or(start);
            let id_str = &line[start..end].trim();
            id_str.parse::<u32>().unwrap_or(0)
        } else {
            return Ok(());
        };

        // Extract properties from method calls
        let mut max_stack_size = 64;
        let mut max_damage = 0;
        let mut enchantable = true;
        let mut food = None;
        let mut tool = None;
        let armor = None;

        // Parse method chain after `->`
        let after_new = if let Some(pos) = line.find(")") {
            &line[pos + 1..]
        } else {
            return Ok(());
        };

        // Split by ->
        for part in after_new.split("->") {
            let part = part.trim();
            if part.starts_with("setMaxStackSize(") {
                let arg = self.extract_call_arg(part, "setMaxStackSize");
                if let Some(arg) = arg {
                    max_stack_size = arg.parse().unwrap_or(64);
                }
            } else if part.starts_with("setMaxDamage(") {
                let arg = self.extract_call_arg(part, "setMaxDamage");
                if let Some(arg) = arg {
                    max_damage = arg.parse().unwrap_or(0);
                }
            } else if part.starts_with("setNoRepair()") {
                // Not enchantable/repairable
                enchantable = false;
            } else if part.starts_with("setFood(") {
                // Food properties would be parsed here
                // Format: setFood(nutrition, saturation, is_wolf_meat)
                let arg = self.extract_call_arg(part, "setFood");
                if let Some(arg) = arg {
                    let parts: Vec<&str> = arg.split(',').map(|s| s.trim()).collect();
                    if parts.len() >= 2 {
                        food = Some(LegacyFoodProperties {
                            nutrition: parts[0].parse().unwrap_or(0),
                            saturation: parts[1].trim_end_matches('f').parse().unwrap_or(0.0),
                            fast_eat: false,
                            meat: parts.get(2).map(|s| s.trim() == "true").unwrap_or(false),
                            always_edible: false,
                        });
                    }
                }
            } else if part.starts_with("setToolClass(") {
                // Tool properties
                let arg = self.extract_call_arg(part, "setToolClass");
                if let Some(arg) = arg {
                    // Format: setToolClass("pickaxe", 2, 8.0f, 3)
                    let parts: Vec<&str> = arg.split(',').map(|s| s.trim()).collect();
                    if parts.len() >= 3 {
                        tool = Some(LegacyToolProperties {
                            tier: parts[0].trim_matches('"').to_string(),
                            max_damage: 0, // Would need separate parsing
                            mining_speed: parts[1].trim_end_matches('f').parse().unwrap_or(1.0),
                            attack_damage: parts[2].trim_end_matches('f').parse().unwrap_or(0.0),
                            harvest_level: parts.get(3).map(|s| s.parse().unwrap_or(0)).unwrap_or(0),
                            effective_on: Vec::new(),
                        });
                    }
                }
            }
        }

        let item = LegacyItem {
            id,
            symbol: symbol.clone(),
            name: self.symbol_to_name(&symbol),
            max_stack_size,
            max_damage,
            food,
            tool,
            armor,
            enchantable,
            repair_items: Vec::new(),
            source_file: self.current_file.clone(),
            source_line: self.current_line,
        };

        self.items.push(item);
        Ok(())
    }

    /// Extract argument from a method call like `setMaxStackSize(64)`.
    fn extract_call_arg(&self, text: &str, method: &str) -> Option<String> {
        let needle = format!("{method}(");
        let start = text.find(&needle)? + needle.len();
        let end = text[start..].find(')')? + start;
        Some(text[start..end].trim().to_string())
    }

    /// Convert symbol to human-readable name.
    fn symbol_to_name(&self, symbol: &str) -> String {
        symbol.replace('_', " ")
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

    /// Get all parsed items.
    pub fn items(&self) -> &[LegacyItem] {
        &self.items
    }

    /// Render as deterministic TOML.
    pub fn render_toml(&self, source_label: &str) -> String {
        let mut out = String::new();
        out.push_str("# Generated by `cargo run -p mclre-devtools --bin extract-legacy-items`.\n");
        out.push_str("# Do not edit by hand: regenerate from the reference source.\n");
        let _ = writeln!(out, "# Source: {source_label}");
        let _ = writeln!(out, "\nitem_count = {}", self.items.len());
        
        for item in &self.items {
            out.push_str("\n[[item]]\n");
            let _ = writeln!(out, "id = {}", item.id);
            let _ = writeln!(out, "symbol = \"{}\"", item.symbol);
            let _ = writeln!(out, "name = \"{}\"", item.name);
            let _ = writeln!(out, "max_stack_size = {}", item.max_stack_size);
            let _ = writeln!(out, "max_damage = {}", item.max_damage);
            let _ = writeln!(out, "enchantable = {}", item.enchantable);
            if let Some(food) = &item.food {
                out.push_str("food = { ");
                let _ = writeln!(out, "nutrition = {}, saturation = {}, fast_eat = {}, meat = {}, always_edible = {} }},",
                    food.nutrition, food.saturation, food.fast_eat, food.meat, food.always_edible);
            }
            if let Some(tool) = &item.tool {
                out.push_str("tool = { ");
                let _ = writeln!(out, "tier = \"{}\", max_damage = {}, mining_speed = {}, attack_damage = {}, harvest_level = {} }},",
                    tool.tier, tool.max_damage, tool.mining_speed, tool.attack_damage, tool.harvest_level);
            }
            if let Some(armor) = &item.armor {
                out.push_str("armor = { ");
                let _ = writeln!(out, "type = \"{}\", max_damage = {}, protection = {}, toughness = {}, knockback_resistance = {}, material = \"{}\" }},",
                    armor.armor_type, armor.max_damage, armor.protection, armor.toughness, armor.knockback_resistance, armor.material);
            }
            let _ = writeln!(out, "source_file = \"{}\"", item.source_file);
            let _ = writeln!(out, "source_line = {}", item.source_line);
        }
        out
    }
}

impl Default for ItemParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse items from the reference Item.cpp.
pub fn parse_items(source: &str) -> Result<Vec<LegacyItem>, String> {
    let mut parser = ItemParser::new();
    parser.parse_file(source, "Item.cpp")?;
    Ok(parser.items)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ITEM_FIXTURE: &str = r#"
        Item::ironIngot = (new Item(265))->setMaxStackSize(64)->setIconName(L"iron_ingot");
        Item::diamondSword = (new Item(276))->setMaxDamage(1561)->setMaxStackSize(1);
        Item::apple = (new Item(260))->setMaxStackSize(64)->setFood(4, 2.4f, false);
    "#;

    #[test]
    fn parses_simple_item() {
        let items = parse_items(ITEM_FIXTURE).expect("parses");
        assert_eq!(items.len(), 3);
        
        let iron = &items[0];
        assert_eq!(iron.symbol, "ironIngot");
        assert_eq!(iron.id, 265);
        assert_eq!(iron.max_stack_size, 64);
        assert_eq!(iron.max_damage, 0);
        
        let sword = &items[1];
        assert_eq!(sword.symbol, "diamondSword");
        assert_eq!(sword.id, 276);
        assert_eq!(sword.max_damage, 1561);
        assert_eq!(sword.max_stack_size, 1);
        
        let apple = &items[2];
        assert_eq!(apple.symbol, "apple");
        assert!(apple.food.is_some());
        let food = apple.food.as_ref().unwrap();
        assert_eq!(food.nutrition, 4);
        assert_eq!(food.saturation, 2.4);
    }
}