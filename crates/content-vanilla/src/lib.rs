//! Baseline (vanilla) content, registered through the public API.
//!
//! "Vanilla is a mod" (SPEC.md 2.5): these definitions use exactly the same
//! registry/property API that third-party content will use — there is no
//! privileged path. Identifiers follow the namespace policy (ADR 0003):
//! `minecraft:` where a Java Edition id exists, `lce:` only for LCE-exclusive
//! content.
//!
//! This crate covers blocks (M1) and items (M5) for the baseline profile.

use mclre_core::{
    BlockProperties, BlockStateModel, Identifier, ParseError, PropertyDefinition, Registry,
    RegistryError, StateError,
    block_state::{HarvestTier, HarvestTool, SoundGroup},
};
use thiserror::Error;

/// Errors while registering vanilla content.
#[derive(Debug, Error)]
pub enum ContentError {
    /// An identifier literal failed validation (a bug in this crate).
    #[error(transparent)]
    Id(#[from] ParseError),
    /// A block property declaration was invalid.
    #[error(transparent)]
    State(#[from] StateError),
    /// The target registry rejected an entry (duplicate or frozen).
    #[error(transparent)]
    Registry(#[from] RegistryError),
}

/// Simple item properties (no block states — items are single-state).
#[derive(Debug, Clone, PartialEq)]
pub struct ItemProperties {
    /// Maximum stack size (default 64).
    pub max_stack_size: u8,
    /// Maximum damage (for tools/armor; 0 = not damageable).
    pub max_damage: u16,
    /// Food properties (if edible).
    pub food: Option<FoodProperties>,
    /// Tool tier (if a tool).
    pub tool: Option<ToolProperties>,
    /// Armor properties (if armor).
    pub armor: Option<ArmorProperties>,
    /// Whether this item can be enchanted.
    pub enchantable: bool,
    /// Repair items (for anvil).
    pub repair_items: Vec<Identifier>,
}

impl Default for ItemProperties {
    fn default() -> Self {
        Self {
            max_stack_size: 64,
            max_damage: 0,
            food: None,
            tool: None,
            armor: None,
            enchantable: true,
            repair_items: Vec::new(),
        }
    }
}

impl ItemProperties {
    /// Create a simple item (no special properties).
    pub fn simple() -> Self {
        Self::default()
    }

    /// Create a damageable item (tool/weapon/armor).
    pub fn damageable(max_damage: u16) -> Self {
        Self {
            max_damage,
            enchantable: true,
            ..Default::default()
        }
    }

    /// Create a food item.
    pub fn food(food: FoodProperties) -> Self {
        Self {
            food: Some(food),
            max_stack_size: 64,
            ..Default::default()
        }
    }

    /// Create a tool.
    pub fn tool(tool: ToolProperties) -> Self {
        Self {
            max_damage: tool.max_damage,
            tool: Some(tool),
            enchantable: true,
            ..Default::default()
        }
    }

    /// Create armor.
    pub fn armor(armor: ArmorProperties) -> Self {
        Self {
            max_damage: armor.max_damage,
            armor: Some(armor),
            enchantable: true,
            max_stack_size: 1,
            ..Default::default()
        }
    }
}

/// Food properties for edible items.
#[derive(Debug, Clone, PartialEq)]
pub struct FoodProperties {
    /// Hunger restored (food level).
    pub nutrition: u8,
    /// Saturation modifier (0.0 to 2.0 typically).
    pub saturation: f32,
    /// Whether this is a "fast food" (eaten quickly).
    pub fast_eat: bool,
    /// Whether this is meat (wolves can eat).
    pub meat: bool,
    /// Always edible even when full.
    pub always_edible: bool,
}

/// Tool properties.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolProperties {
    /// Tool tier (wood, stone, iron, gold, diamond).
    pub tier: ToolTier,
    /// Maximum damage (durability).
    pub max_damage: u16,
    /// Mining speed multiplier.
    pub mining_speed: f32,
    /// Attack damage bonus.
    pub attack_damage: f32,
    /// Blocks this tool can harvest effectively.
    pub harvest_level: u8,
    /// Tags of blocks this tool is effective on.
    pub effective_on: Vec<Identifier>,
}

/// Tool tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolTier {
    Wood,
    Stone,
    Iron,
    Gold,
    Diamond,
}

impl ToolTier {
    /// Get base durability for this tier.
    pub fn base_durability(&self) -> u16 {
        match self {
            ToolTier::Wood => 59,
            ToolTier::Stone => 131,
            ToolTier::Iron => 250,
            ToolTier::Gold => 32,
            ToolTier::Diamond => 1561,
        }
    }

    /// Get mining speed multiplier.
    pub fn mining_speed(&self) -> f32 {
        match self {
            ToolTier::Wood => 2.0,
            ToolTier::Stone => 4.0,
            ToolTier::Iron => 6.0,
            ToolTier::Gold => 12.0,
            ToolTier::Diamond => 8.0,
        }
    }

    /// Get attack damage bonus.
    pub fn attack_damage(&self) -> f32 {
        match self {
            ToolTier::Wood => 0.0,
            ToolTier::Stone => 1.0,
            ToolTier::Iron => 2.0,
            ToolTier::Gold => 0.0,
            ToolTier::Diamond => 3.0,
        }
    }

    /// Get harvest level (0=wood, 1=stone, 2=iron, 3=diamond).
    pub fn harvest_level(&self) -> u8 {
        match self {
            ToolTier::Wood => 0,
            ToolTier::Stone => 1,
            ToolTier::Iron => 2,
            ToolTier::Gold => 0,
            ToolTier::Diamond => 3,
        }
    }

    /// Get enchantability.
    pub fn enchantability(&self) -> u8 {
        match self {
            ToolTier::Wood => 15,
            ToolTier::Stone => 5,
            ToolTier::Iron => 14,
            ToolTier::Gold => 22,
            ToolTier::Diamond => 10,
        }
    }

    /// Get repair item for this tier.
    pub fn repair_item(&self) -> Identifier {
        match self {
            ToolTier::Wood => Identifier::parse("minecraft:planks").unwrap(),
            ToolTier::Stone => Identifier::parse("minecraft:cobblestone").unwrap(),
            ToolTier::Iron => Identifier::parse("minecraft:iron_ingot").unwrap(),
            ToolTier::Gold => Identifier::parse("minecraft:gold_ingot").unwrap(),
            ToolTier::Diamond => Identifier::parse("minecraft:diamond").unwrap(),
        }
    }
}

/// Armor properties.
#[derive(Debug, Clone, PartialEq)]
pub struct ArmorProperties {
    /// Armor type.
    pub armor_type: ArmorType,
    /// Maximum damage (durability).
    pub max_damage: u16,
    /// Protection amount (armor points).
    pub protection: u8,
    /// Toughness (reduces damage more effectively).
    pub toughness: f32,
    /// Knockback resistance.
    pub knockback_resistance: f32,
    /// Armor material (for repair).
    pub material: ArmorMaterial,
}

/// Armor type/slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmorType {
    Helmet,
    Chestplate,
    Leggings,
    Boots,
}

/// Armor material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmorMaterial {
    Leather,
    Chainmail,
    Iron,
    Gold,
    Diamond,
}

impl ArmorMaterial {
    /// Get base durability for this material.
    pub fn base_durability(&self, armor_type: ArmorType) -> u16 {
        let multiplier = match armor_type {
            ArmorType::Helmet => 11,
            ArmorType::Chestplate => 16,
            ArmorType::Leggings => 15,
            ArmorType::Boots => 13,
        };
        match self {
            ArmorMaterial::Leather => 5 * multiplier,
            ArmorMaterial::Chainmail => 15 * multiplier,
            ArmorMaterial::Iron => 19 * multiplier,
            ArmorMaterial::Gold => 7 * multiplier,
            ArmorMaterial::Diamond => 33 * multiplier,
        }
    }

    /// Get protection for this material and type.
    pub fn protection(&self, armor_type: ArmorType) -> u8 {
        match (self, armor_type) {
            (ArmorMaterial::Leather, ArmorType::Helmet) => 1,
            (ArmorMaterial::Leather, ArmorType::Chestplate) => 3,
            (ArmorMaterial::Leather, ArmorType::Leggings) => 2,
            (ArmorMaterial::Leather, ArmorType::Boots) => 1,
            (ArmorMaterial::Chainmail, ArmorType::Helmet) => 2,
            (ArmorMaterial::Chainmail, ArmorType::Chestplate) => 5,
            (ArmorMaterial::Chainmail, ArmorType::Leggings) => 4,
            (ArmorMaterial::Chainmail, ArmorType::Boots) => 1,
            (ArmorMaterial::Iron, ArmorType::Helmet) => 2,
            (ArmorMaterial::Iron, ArmorType::Chestplate) => 6,
            (ArmorMaterial::Iron, ArmorType::Leggings) => 5,
            (ArmorMaterial::Iron, ArmorType::Boots) => 2,
            (ArmorMaterial::Gold, ArmorType::Helmet) => 2,
            (ArmorMaterial::Gold, ArmorType::Chestplate) => 5,
            (ArmorMaterial::Gold, ArmorType::Leggings) => 3,
            (ArmorMaterial::Gold, ArmorType::Boots) => 1,
            (ArmorMaterial::Diamond, ArmorType::Helmet) => 3,
            (ArmorMaterial::Diamond, ArmorType::Chestplate) => 8,
            (ArmorMaterial::Diamond, ArmorType::Leggings) => 6,
            (ArmorMaterial::Diamond, ArmorType::Boots) => 3,
        }
    }

    /// Get toughness.
    pub fn toughness(&self) -> f32 {
        match self {
            ArmorMaterial::Leather => 0.0,
            ArmorMaterial::Chainmail => 0.0,
            ArmorMaterial::Iron => 0.0,
            ArmorMaterial::Gold => 0.0,
            ArmorMaterial::Diamond => 2.0,
        }
    }

    /// Get knockback resistance.
    pub fn knockback_resistance(&self) -> f32 {
        match self {
            ArmorMaterial::Leather => 0.0,
            ArmorMaterial::Chainmail => 0.0,
            ArmorMaterial::Iron => 0.0,
            ArmorMaterial::Gold => 0.0,
            ArmorMaterial::Diamond => 0.0,
        }
    }

    /// Get repair item for this material.
    pub fn repair_item(&self) -> Identifier {
        match self {
            ArmorMaterial::Leather => Identifier::parse("minecraft:leather").unwrap(),
            ArmorMaterial::Chainmail => Identifier::parse("minecraft:iron_ingot").unwrap(),
            ArmorMaterial::Iron => Identifier::parse("minecraft:iron_ingot").unwrap(),
            ArmorMaterial::Gold => Identifier::parse("minecraft:gold_ingot").unwrap(),
            ArmorMaterial::Diamond => Identifier::parse("minecraft:diamond").unwrap(),
        }
    }
}

/// The property sets of the M1 block set, in registration order.
///
/// Order matters: block indices and block-state ids are derived from it, so
/// entries are only ever appended, never reordered (SPEC.md 5.3).
fn block_table() -> Result<Vec<(&'static str, BlockProperties)>, ContentError> {
    // Facings/axes use the modern flattened order; these lists define the
    // canonical state order and must be treated as frozen once saves exist.
    let axis = || PropertyDefinition::enumeration("axis", ["x", "y", "z"]);
    let facing = || PropertyDefinition::enumeration("facing", ["north", "south", "west", "east"]);

    let entries: Vec<(&'static str, Result<BlockProperties, StateError>)> = vec![
        // Air must be index 0 so "empty" is a stable, cheap sentinel.
        ("minecraft:air", Ok(BlockProperties::none())),
        (
            "minecraft:stone",
            BlockProperties::with_gameplay(
                vec![],
                1.5,
                HarvestTier::Wood,
                HarvestTool::Pickaxe,
                0,
                false,
                false,
                SoundGroup::Stone,
                true,
                None,
            ),
        ),
        (
            "minecraft:dirt",
            BlockProperties::with_gameplay(
                vec![],
                0.5,
                HarvestTier::Wood,
                HarvestTool::Shovel,
                0,
                false,
                false,
                SoundGroup::Gravel,
                true,
                None,
            ),
        ),
        (
            "minecraft:grass_block",
            BlockProperties::with_gameplay(
                vec![PropertyDefinition::bool("snowy")],
                0.6,
                HarvestTier::Wood,
                HarvestTool::Shovel,
                0,
                false,
                false,
                SoundGroup::Gravel,
                true,
                None,
            ),
        ),
        (
            "minecraft:bedrock",
            BlockProperties::with_gameplay(
                vec![],
                -1.0,
                HarvestTier::Diamond,
                HarvestTool::Pickaxe,
                0,
                false,
                false,
                SoundGroup::Stone,
                false,
                None,
            ),
        ),
        (
            "minecraft:sand",
            BlockProperties::with_gameplay(
                vec![],
                0.5,
                HarvestTier::Wood,
                HarvestTool::Shovel,
                0,
                false,
                false,
                SoundGroup::Gravel,
                true,
                None,
            ),
        ),
        (
            "minecraft:water",
            BlockProperties::with_gameplay(
                vec![PropertyDefinition::int("level", 0, 15)],
                100.0,
                HarvestTier::Wood,
                HarvestTool::Hand,
                0,
                true,
                true,
                SoundGroup::Plant,
                false,
                None,
            ),
        ),
        (
            "minecraft:lava",
            BlockProperties::with_gameplay(
                vec![PropertyDefinition::int("level", 0, 15)],
                100.0,
                HarvestTier::Wood,
                HarvestTool::Hand,
                15,
                true,
                true,
                SoundGroup::Stone,
                false,
                None,
            ),
        ),
        (
            "minecraft:oak_log",
            BlockProperties::with_gameplay(
                vec![axis()],
                2.0,
                HarvestTier::Wood,
                HarvestTool::Axe,
                0,
                false,
                false,
                SoundGroup::Wood,
                true,
                None,
            ),
        ),
        (
            "minecraft:oak_leaves",
            BlockProperties::with_gameplay(
                vec![
                    PropertyDefinition::int("distance", 1, 7),
                    PropertyDefinition::bool("persistent"),
                ],
                0.2,
                HarvestTier::Wood,
                HarvestTool::Shears,
                0,
                true,
                false,
                SoundGroup::Plant,
                true,
                None,
            ),
        ),
        (
            "minecraft:oak_planks",
            BlockProperties::with_gameplay(
                vec![],
                2.0,
                HarvestTier::Wood,
                HarvestTool::Axe,
                0,
                false,
                false,
                SoundGroup::Wood,
                true,
                None,
            ),
        ),
        (
            "minecraft:cobblestone",
            BlockProperties::with_gameplay(
                vec![],
                2.0,
                HarvestTier::Wood,
                HarvestTool::Pickaxe,
                0,
                false,
                false,
                SoundGroup::Stone,
                true,
                None,
            ),
        ),
        (
            "minecraft:coal_ore",
            BlockProperties::with_gameplay(
                vec![],
                3.0,
                HarvestTier::Wood,
                HarvestTool::Pickaxe,
                0,
                false,
                false,
                SoundGroup::Stone,
                true,
                None,
            ),
        ),
        (
            "minecraft:iron_ore",
            BlockProperties::with_gameplay(
                vec![],
                3.0,
                HarvestTier::Stone,
                HarvestTool::Pickaxe,
                0,
                false,
                false,
                SoundGroup::Stone,
                true,
                None,
            ),
        ),
        (
            "minecraft:glass",
            BlockProperties::with_gameplay(
                vec![],
                0.3,
                HarvestTier::Wood,
                HarvestTool::Pickaxe,
                0,
                true,
                false,
                SoundGroup::Glass,
                false,
                None,
            ),
        ),
        (
            "minecraft:oak_stairs",
            // 4 facings x 2 halves x 5 shapes = 40 states.
            BlockProperties::with_gameplay(
                vec![
                    facing(),
                    PropertyDefinition::enumeration("half", ["bottom", "top"]),
                    PropertyDefinition::enumeration(
                        "shape",
                        [
                            "straight",
                            "inner_left",
                            "inner_right",
                            "outer_left",
                            "outer_right",
                        ],
                    ),
                ],
                2.0,
                HarvestTier::Wood,
                HarvestTool::Axe,
                0,
                false,
                false,
                SoundGroup::Wood,
                true,
                None,
            ),
        ),
        (
            "minecraft:stone_slab",
            // 3 states: bottom, top, double.
            BlockProperties::with_gameplay(
                vec![PropertyDefinition::enumeration(
                    "type",
                    ["bottom", "top", "double"],
                )],
                2.0,
                HarvestTier::Wood,
                HarvestTool::Pickaxe,
                0,
                false,
                false,
                SoundGroup::Stone,
                true,
                None,
            ),
        ),
        (
            "minecraft:crafting_table",
            BlockProperties::with_gameplay(
                vec![],
                2.5,
                HarvestTier::Wood,
                HarvestTool::Axe,
                0,
                false,
                false,
                SoundGroup::Wood,
                true,
                None,
            ),
        ),
        (
            "minecraft:furnace",
            // 4 facings x lit(false/true) = 8 states.
            BlockProperties::with_gameplay(
                vec![facing(), PropertyDefinition::bool("lit")],
                3.5,
                HarvestTier::Stone,
                HarvestTool::Pickaxe,
                0,
                false,
                false,
                SoundGroup::Stone,
                true,
                None,
            ),
        ),
        (
            "minecraft:torch",
            // 5 attach directions, `up` first (default).
            BlockProperties::with_gameplay(
                vec![PropertyDefinition::enumeration(
                    "facing",
                    ["up", "north", "south", "west", "east"],
                )],
                0.0,
                HarvestTier::Wood,
                HarvestTool::Hand,
                14,
                true,
                false,
                SoundGroup::Plant,
                false,
                None,
            ),
        ),
        (
            "minecraft:chest",
            BlockProperties::with_gameplay(
                vec![facing()],
                2.5,
                HarvestTier::Wood,
                HarvestTool::Axe,
                0,
                false,
                false,
                SoundGroup::Wood,
                true,
                None,
            ),
        ),
        (
            "minecraft:wheat",
            BlockProperties::with_gameplay(
                vec![PropertyDefinition::int("age", 0, 7)],
                0.0,
                HarvestTier::Wood,
                HarvestTool::Hand,
                0,
                true,
                true,
                SoundGroup::Plant,
                false,
                None,
            ),
        ),
    ];

    let mut out = Vec::with_capacity(entries.len());
    for (id, properties) in entries {
        out.push((id, properties?));
    }
    Ok(out)
}

/// Number of blocks in the M1 vanilla set.
pub const BLOCK_COUNT: usize = 22;

/// Registers every baseline block into `registry`, returning the number added.
///
/// Entries are registered in the frozen [`block_table`] order, so indices are
/// reproducible. Callers that add mod content afterwards get the next indices.
pub fn register(registry: &mut Registry<BlockProperties>) -> Result<u32, ContentError> {
    let table = block_table()?;
    debug_assert_eq!(table.len(), BLOCK_COUNT);
    let mut added = 0;
    for (id, properties) in table {
        registry.register(Identifier::parse(id)?, properties)?;
        added += 1;
    }
    Ok(added)
}

/// Builds a frozen [`BlockStateModel`] containing only vanilla blocks.
///
/// Convenience for tests, tools, and worldgen; real sessions start from a
/// shared registry that mods also populate.
pub fn block_states() -> Result<BlockStateModel, ContentError> {
    let mut registry = Registry::new("minecraft:block");
    register(&mut registry)?;
    Ok(BlockStateModel::freeze(registry)?)
}

/// Item table for baseline profile (M5).
/// Order matters: item indices are derived from registration order.
fn item_table() -> Vec<(&'static str, ItemProperties)> {
    vec![
        // Materials
        ("minecraft:iron_ingot", ItemProperties::simple()),
        ("minecraft:gold_ingot", ItemProperties::simple()),
        ("minecraft:diamond", ItemProperties::simple()),
        ("minecraft:coal", ItemProperties::simple()),
        ("minecraft:charcoal", ItemProperties::simple()),
        ("minecraft:redstone", ItemProperties::simple()),
        ("minecraft:lapis_lazuli", ItemProperties::simple()),
        ("minecraft:quartz", ItemProperties::simple()),
        ("minecraft:emerald", ItemProperties::simple()),
        ("minecraft:netherite_scrap", ItemProperties::simple()),
        ("minecraft:netherite_ingot", ItemProperties::simple()),
        ("minecraft:stick", ItemProperties::simple()),
        ("minecraft:bowl", ItemProperties::simple()),
        ("minecraft:string", ItemProperties::simple()),
        ("minecraft:feather", ItemProperties::simple()),
        ("minecraft:gunpowder", ItemProperties::simple()),
        ("minecraft:wheat_seeds", ItemProperties::simple()),
        ("minecraft:flint", ItemProperties::simple()),
        ("minecraft:leather", ItemProperties::simple()),
        ("minecraft:paper", ItemProperties::simple()),
        ("minecraft:book", ItemProperties::simple()),
        ("minecraft:slime_ball", ItemProperties::simple()),
        ("minecraft:ender_pearl", ItemProperties::simple()),
        ("minecraft:blaze_rod", ItemProperties::simple()),
        ("minecraft:ghast_tear", ItemProperties::simple()),
        ("minecraft:gold_nugget", ItemProperties::simple()),
        ("minecraft:clay_ball", ItemProperties::simple()),
        ("minecraft:brick", ItemProperties::simple()),
        ("minecraft:nether_brick", ItemProperties::simple()),
        ("minecraft:snowball", ItemProperties::simple()),
        ("minecraft:egg", ItemProperties::simple()),
        ("minecraft:ink_sac", ItemProperties::simple()),
        ("minecraft:bone", ItemProperties::simple()),
        ("minecraft:sugar", ItemProperties::simple()),
        ("minecraft:fermented_spider_eye", ItemProperties::simple()),
        ("minecraft:blaze_powder", ItemProperties::simple()),
        ("minecraft:magma_cream", ItemProperties::simple()),
        ("minecraft:brewing_stand", ItemProperties::simple()),
        ("minecraft:cauldron", ItemProperties::simple()),
        ("minecraft:eye_of_ender", ItemProperties::simple()),
        ("minecraft:glistering_melon_slice", ItemProperties::simple()),
        ("minecraft:spawn_egg", ItemProperties::simple()),
        ("minecraft:experience_bottle", ItemProperties::simple()),
        ("minecraft:fire_charge", ItemProperties::simple()),
        ("minecraft:writable_book", ItemProperties::simple()),
        ("minecraft:written_book", ItemProperties::simple()),
        ("minecraft:emerald_block", ItemProperties::simple()),
        ("minecraft:diamond_block", ItemProperties::simple()),
        ("minecraft:gold_block", ItemProperties::simple()),
        ("minecraft:iron_block", ItemProperties::simple()),
        ("minecraft:redstone_block", ItemProperties::simple()),
        ("minecraft:lapis_block", ItemProperties::simple()),
        ("minecraft:coal_block", ItemProperties::simple()),
        ("minecraft:quartz_block", ItemProperties::simple()),
        ("minecraft:netherite_block", ItemProperties::simple()),
        ("minecraft:sand", ItemProperties::simple()),
        ("minecraft:red_sand", ItemProperties::simple()),
        ("minecraft:gravel", ItemProperties::simple()),
        ("minecraft:clay", ItemProperties::simple()),
        ("minecraft:dirt", ItemProperties::simple()),
        ("minecraft:grass_block", ItemProperties::simple()),
        ("minecraft:mycelium", ItemProperties::simple()),
        ("minecraft:podzol", ItemProperties::simple()),
        ("minecraft:stone", ItemProperties::simple()),
        ("minecraft:cobblestone", ItemProperties::simple()),
        ("minecraft:mossy_cobblestone", ItemProperties::simple()),
        ("minecraft:oak_planks", ItemProperties::simple()),
        ("minecraft:spruce_planks", ItemProperties::simple()),
        ("minecraft:birch_planks", ItemProperties::simple()),
        ("minecraft:jungle_planks", ItemProperties::simple()),
        ("minecraft:acacia_planks", ItemProperties::simple()),
        ("minecraft:dark_oak_planks", ItemProperties::simple()),
        ("minecraft:oak_log", ItemProperties::simple()),
        ("minecraft:spruce_log", ItemProperties::simple()),
        ("minecraft:birch_log", ItemProperties::simple()),
        ("minecraft:jungle_log", ItemProperties::simple()),
        ("minecraft:acacia_log", ItemProperties::simple()),
        ("minecraft:dark_oak_log", ItemProperties::simple()),
        ("minecraft:oak_leaves", ItemProperties::simple()),
        ("minecraft:spruce_leaves", ItemProperties::simple()),
        ("minecraft:birch_leaves", ItemProperties::simple()),
        ("minecraft:jungle_leaves", ItemProperties::simple()),
        ("minecraft:acacia_leaves", ItemProperties::simple()),
        ("minecraft:dark_oak_leaves", ItemProperties::simple()),
        ("minecraft:sponge", ItemProperties::simple()),
        ("minecraft:wet_sponge", ItemProperties::simple()),
        ("minecraft:glass", ItemProperties::simple()),
        ("minecraft:tinted_glass", ItemProperties::simple()),
        ("minecraft:glass_pane", ItemProperties::simple()),
        ("minecraft:sea_lantern", ItemProperties::simple()),
        ("minecraft:glowstone", ItemProperties::simple()),
        ("minecraft:jack_o_lantern", ItemProperties::simple()),
        ("minecraft:torch", ItemProperties::simple()),
        ("minecraft:wall_torch", ItemProperties::simple()),
        ("minecraft:lantern", ItemProperties::simple()),
        ("minecraft:soul_lantern", ItemProperties::simple()),
        ("minecraft:campfire", ItemProperties::simple()),
        ("minecraft:soul_campfire", ItemProperties::simple()),
        ("minecraft:fire", ItemProperties::simple()),
        ("minecraft:soul_fire", ItemProperties::simple()),
        ("minecraft:lava_bucket", ItemProperties::simple()),
        ("minecraft:water_bucket", ItemProperties::simple()),
        ("minecraft:powder_snow_bucket", ItemProperties::simple()),
        ("minecraft:milk_bucket", ItemProperties::simple()),
        ("minecraft:bucket", ItemProperties::simple()),
        ("minecraft:compass", ItemProperties::simple()),
        ("minecraft:clock", ItemProperties::simple()),
        ("minecraft:fishing_rod", ItemProperties::damageable(64)),
        ("minecraft:carrot_on_a_stick", ItemProperties::damageable(25)),
        ("minecraft:warped_fungus_on_a_stick", ItemProperties::damageable(100)),
        ("minecraft:lead", ItemProperties::simple()),
        ("minecraft:name_tag", ItemProperties::simple()),
        ("minecraft:saddle", ItemProperties::simple()),
        ("minecraft:horse_armor_iron", ItemProperties::simple()),
        ("minecraft:horse_armor_gold", ItemProperties::simple()),
        ("minecraft:horse_armor_diamond", ItemProperties::simple()),
        ("minecraft:horse_armor_leather", ItemProperties::simple()),
        ("minecraft:shield", ItemProperties::damageable(336)),
        ("minecraft:totem_of_undying", ItemProperties::simple()),
        ("minecraft:phantom_membrane", ItemProperties::simple()),
        ("minecraft:nautilus_shell", ItemProperties::simple()),
        ("minecraft:heart_of_the_sea", ItemProperties::simple()),
        ("minecraft:trident", ItemProperties::damageable(250)),

        // Tools - Pickaxes
        ("minecraft:wooden_pickaxe", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Wood,
            max_damage: ToolTier::Wood.base_durability(),
            mining_speed: ToolTier::Wood.mining_speed(),
            attack_damage: ToolTier::Wood.attack_damage(),
            harvest_level: ToolTier::Wood.harvest_level(),
            effective_on: vec![
                Identifier::parse("minecraft:stone").unwrap(),
                Identifier::parse("minecraft:cobblestone").unwrap(),
                Identifier::parse("minecraft:coal_ore").unwrap(),
                Identifier::parse("minecraft:iron_ore").unwrap(),
            ],
        })),
        ("minecraft:stone_pickaxe", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Stone,
            max_damage: ToolTier::Stone.base_durability(),
            mining_speed: ToolTier::Stone.mining_speed(),
            attack_damage: ToolTier::Stone.attack_damage(),
            harvest_level: ToolTier::Stone.harvest_level(),
            effective_on: vec![
                Identifier::parse("minecraft:stone").unwrap(),
                Identifier::parse("minecraft:cobblestone").unwrap(),
                Identifier::parse("minecraft:coal_ore").unwrap(),
                Identifier::parse("minecraft:iron_ore").unwrap(),
                Identifier::parse("minecraft:gold_ore").unwrap(),
            ],
        })),
        ("minecraft:iron_pickaxe", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Iron,
            max_damage: ToolTier::Iron.base_durability(),
            mining_speed: ToolTier::Iron.mining_speed(),
            attack_damage: ToolTier::Iron.attack_damage(),
            harvest_level: ToolTier::Iron.harvest_level(),
            effective_on: vec![
                Identifier::parse("minecraft:stone").unwrap(),
                Identifier::parse("minecraft:cobblestone").unwrap(),
                Identifier::parse("minecraft:coal_ore").unwrap(),
                Identifier::parse("minecraft:iron_ore").unwrap(),
                Identifier::parse("minecraft:gold_ore").unwrap(),
                Identifier::parse("minecraft:diamond_ore").unwrap(),
                Identifier::parse("minecraft:emerald_ore").unwrap(),
                Identifier::parse("minecraft:redstone_ore").unwrap(),
                Identifier::parse("minecraft:lapis_ore").unwrap(),
            ],
        })),
        ("minecraft:golden_pickaxe", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Gold,
            max_damage: ToolTier::Gold.base_durability(),
            mining_speed: ToolTier::Gold.mining_speed(),
            attack_damage: ToolTier::Gold.attack_damage(),
            harvest_level: ToolTier::Gold.harvest_level(),
            effective_on: vec![
                Identifier::parse("minecraft:stone").unwrap(),
                Identifier::parse("minecraft:cobblestone").unwrap(),
                Identifier::parse("minecraft:coal_ore").unwrap(),
            ],
        })),
        ("minecraft:diamond_pickaxe", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Diamond,
            max_damage: ToolTier::Diamond.base_durability(),
            mining_speed: ToolTier::Diamond.mining_speed(),
            attack_damage: ToolTier::Diamond.attack_damage(),
            harvest_level: ToolTier::Diamond.harvest_level(),
            effective_on: vec![
                Identifier::parse("minecraft:stone").unwrap(),
                Identifier::parse("minecraft:cobblestone").unwrap(),
                Identifier::parse("minecraft:coal_ore").unwrap(),
                Identifier::parse("minecraft:iron_ore").unwrap(),
                Identifier::parse("minecraft:gold_ore").unwrap(),
                Identifier::parse("minecraft:diamond_ore").unwrap(),
                Identifier::parse("minecraft:emerald_ore").unwrap(),
                Identifier::parse("minecraft:redstone_ore").unwrap(),
                Identifier::parse("minecraft:lapis_ore").unwrap(),
                Identifier::parse("minecraft:obsidian").unwrap(),
                Identifier::parse("minecraft:netherrack").unwrap(),
            ],
        })),

        // Tools - Axes
        ("minecraft:wooden_axe", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Wood,
            max_damage: ToolTier::Wood.base_durability(),
            mining_speed: ToolTier::Wood.mining_speed(),
            attack_damage: ToolTier::Wood.attack_damage() + 3.0,
            harvest_level: ToolTier::Wood.harvest_level(),
            effective_on: vec![
                Identifier::parse("minecraft:oak_log").unwrap(),
                Identifier::parse("minecraft:spruce_log").unwrap(),
                Identifier::parse("minecraft:birch_log").unwrap(),
                Identifier::parse("minecraft:jungle_log").unwrap(),
                Identifier::parse("minecraft:acacia_log").unwrap(),
                Identifier::parse("minecraft:dark_oak_log").unwrap(),
                Identifier::parse("minecraft:oak_planks").unwrap(),
                Identifier::parse("minecraft:bookshelf").unwrap(),
            ],
        })),
        ("minecraft:stone_axe", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Stone,
            max_damage: ToolTier::Stone.base_durability(),
            mining_speed: ToolTier::Stone.mining_speed(),
            attack_damage: ToolTier::Stone.attack_damage() + 4.0,
            harvest_level: ToolTier::Stone.harvest_level(),
            effective_on: vec![
                Identifier::parse("minecraft:oak_log").unwrap(),
                Identifier::parse("minecraft:spruce_log").unwrap(),
                Identifier::parse("minecraft:birch_log").unwrap(),
                Identifier::parse("minecraft:jungle_log").unwrap(),
                Identifier::parse("minecraft:acacia_log").unwrap(),
                Identifier::parse("minecraft:dark_oak_log").unwrap(),
                Identifier::parse("minecraft:oak_planks").unwrap(),
                Identifier::parse("minecraft:bookshelf").unwrap(),
            ],
        })),
        ("minecraft:iron_axe", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Iron,
            max_damage: ToolTier::Iron.base_durability(),
            mining_speed: ToolTier::Iron.mining_speed(),
            attack_damage: ToolTier::Iron.attack_damage() + 5.0,
            harvest_level: ToolTier::Iron.harvest_level(),
            effective_on: vec![
                Identifier::parse("minecraft:oak_log").unwrap(),
                Identifier::parse("minecraft:spruce_log").unwrap(),
                Identifier::parse("minecraft:birch_log").unwrap(),
                Identifier::parse("minecraft:jungle_log").unwrap(),
                Identifier::parse("minecraft:acacia_log").unwrap(),
                Identifier::parse("minecraft:dark_oak_log").unwrap(),
                Identifier::parse("minecraft:oak_planks").unwrap(),
                Identifier::parse("minecraft:bookshelf").unwrap(),
            ],
        })),
        ("minecraft:golden_axe", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Gold,
            max_damage: ToolTier::Gold.base_durability(),
            mining_speed: ToolTier::Gold.mining_speed(),
            attack_damage: ToolTier::Gold.attack_damage() + 3.0,
            harvest_level: ToolTier::Gold.harvest_level(),
            effective_on: vec![
                Identifier::parse("minecraft:oak_log").unwrap(),
                Identifier::parse("minecraft:spruce_log").unwrap(),
                Identifier::parse("minecraft:birch_log").unwrap(),
                Identifier::parse("minecraft:jungle_log").unwrap(),
                Identifier::parse("minecraft:acacia_log").unwrap(),
                Identifier::parse("minecraft:dark_oak_log").unwrap(),
                Identifier::parse("minecraft:oak_planks").unwrap(),
                Identifier::parse("minecraft:bookshelf").unwrap(),
            ],
        })),
        ("minecraft:diamond_axe", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Diamond,
            max_damage: ToolTier::Diamond.base_durability(),
            mining_speed: ToolTier::Diamond.mining_speed(),
            attack_damage: ToolTier::Diamond.attack_damage() + 6.0,
            harvest_level: ToolTier::Diamond.harvest_level(),
            effective_on: vec![
                Identifier::parse("minecraft:oak_log").unwrap(),
                Identifier::parse("minecraft:spruce_log").unwrap(),
                Identifier::parse("minecraft:birch_log").unwrap(),
                Identifier::parse("minecraft:jungle_log").unwrap(),
                Identifier::parse("minecraft:acacia_log").unwrap(),
                Identifier::parse("minecraft:dark_oak_log").unwrap(),
                Identifier::parse("minecraft:oak_planks").unwrap(),
                Identifier::parse("minecraft:bookshelf").unwrap(),
            ],
        })),

        // Tools - Shovels
        ("minecraft:wooden_shovel", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Wood,
            max_damage: ToolTier::Wood.base_durability(),
            mining_speed: ToolTier::Wood.mining_speed(),
            attack_damage: ToolTier::Wood.attack_damage() + 1.5,
            harvest_level: ToolTier::Wood.harvest_level(),
            effective_on: vec![
                Identifier::parse("minecraft:dirt").unwrap(),
                Identifier::parse("minecraft:grass_block").unwrap(),
                Identifier::parse("minecraft:sand").unwrap(),
                Identifier::parse("minecraft:gravel").unwrap(),
                Identifier::parse("minecraft:clay").unwrap(),
                Identifier::parse("minecraft:snow").unwrap(),
            ],
        })),
        ("minecraft:stone_shovel", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Stone,
            max_damage: ToolTier::Stone.base_durability(),
            mining_speed: ToolTier::Stone.mining_speed(),
            attack_damage: ToolTier::Stone.attack_damage() + 2.5,
            harvest_level: ToolTier::Stone.harvest_level(),
            effective_on: vec![
                Identifier::parse("minecraft:dirt").unwrap(),
                Identifier::parse("minecraft:grass_block").unwrap(),
                Identifier::parse("minecraft:sand").unwrap(),
                Identifier::parse("minecraft:gravel").unwrap(),
                Identifier::parse("minecraft:clay").unwrap(),
                Identifier::parse("minecraft:snow").unwrap(),
            ],
        })),
        ("minecraft:iron_shovel", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Iron,
            max_damage: ToolTier::Iron.base_durability(),
            mining_speed: ToolTier::Iron.mining_speed(),
            attack_damage: ToolTier::Iron.attack_damage() + 3.5,
            harvest_level: ToolTier::Iron.harvest_level(),
            effective_on: vec![
                Identifier::parse("minecraft:dirt").unwrap(),
                Identifier::parse("minecraft:grass_block").unwrap(),
                Identifier::parse("minecraft:sand").unwrap(),
                Identifier::parse("minecraft:gravel").unwrap(),
                Identifier::parse("minecraft:clay").unwrap(),
                Identifier::parse("minecraft:snow").unwrap(),
            ],
        })),
        ("minecraft:golden_shovel", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Gold,
            max_damage: ToolTier::Gold.base_durability(),
            mining_speed: ToolTier::Gold.mining_speed(),
            attack_damage: ToolTier::Gold.attack_damage() + 1.5,
            harvest_level: ToolTier::Gold.harvest_level(),
            effective_on: vec![
                Identifier::parse("minecraft:dirt").unwrap(),
                Identifier::parse("minecraft:grass_block").unwrap(),
                Identifier::parse("minecraft:sand").unwrap(),
                Identifier::parse("minecraft:gravel").unwrap(),
                Identifier::parse("minecraft:clay").unwrap(),
                Identifier::parse("minecraft:snow").unwrap(),
            ],
        })),
        ("minecraft:diamond_shovel", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Diamond,
            max_damage: ToolTier::Diamond.base_durability(),
            mining_speed: ToolTier::Diamond.mining_speed(),
            attack_damage: ToolTier::Diamond.attack_damage() + 4.5,
            harvest_level: ToolTier::Diamond.harvest_level(),
            effective_on: vec![
                Identifier::parse("minecraft:dirt").unwrap(),
                Identifier::parse("minecraft:grass_block").unwrap(),
                Identifier::parse("minecraft:sand").unwrap(),
                Identifier::parse("minecraft:gravel").unwrap(),
                Identifier::parse("minecraft:clay").unwrap(),
                Identifier::parse("minecraft:snow").unwrap(),
            ],
        })),

        // Tools - Hoes
        ("minecraft:wooden_hoe", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Wood,
            max_damage: ToolTier::Wood.base_durability(),
            mining_speed: 1.0,
            attack_damage: ToolTier::Wood.attack_damage(),
            harvest_level: ToolTier::Wood.harvest_level(),
            effective_on: vec![
                Identifier::parse("minecraft:dirt").unwrap(),
                Identifier::parse("minecraft:grass_block").unwrap(),
            ],
        })),
        ("minecraft:stone_hoe", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Stone,
            max_damage: ToolTier::Stone.base_durability(),
            mining_speed: 1.0,
            attack_damage: ToolTier::Stone.attack_damage() + 1.0,
            harvest_level: ToolTier::Stone.harvest_level(),
            effective_on: vec![
                Identifier::parse("minecraft:dirt").unwrap(),
                Identifier::parse("minecraft:grass_block").unwrap(),
            ],
        })),
        ("minecraft:iron_hoe", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Iron,
            max_damage: ToolTier::Iron.base_durability(),
            mining_speed: 1.0,
            attack_damage: ToolTier::Iron.attack_damage() + 1.0,
            harvest_level: ToolTier::Iron.harvest_level(),
            effective_on: vec![
                Identifier::parse("minecraft:dirt").unwrap(),
                Identifier::parse("minecraft:grass_block").unwrap(),
            ],
        })),
        ("minecraft:golden_hoe", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Gold,
            max_damage: ToolTier::Gold.base_durability(),
            mining_speed: 1.0,
            attack_damage: ToolTier::Gold.attack_damage(),
            harvest_level: ToolTier::Gold.harvest_level(),
            effective_on: vec![
                Identifier::parse("minecraft:dirt").unwrap(),
                Identifier::parse("minecraft:grass_block").unwrap(),
            ],
        })),
        ("minecraft:diamond_hoe", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Diamond,
            max_damage: ToolTier::Diamond.base_durability(),
            mining_speed: 1.0,
            attack_damage: ToolTier::Diamond.attack_damage() + 1.0,
            harvest_level: ToolTier::Diamond.harvest_level(),
            effective_on: vec![
                Identifier::parse("minecraft:dirt").unwrap(),
                Identifier::parse("minecraft:grass_block").unwrap(),
            ],
        })),

        // Tools - Swords
        ("minecraft:wooden_sword", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Wood,
            max_damage: ToolTier::Wood.base_durability(),
            mining_speed: 1.0,
            attack_damage: 4.0,
            harvest_level: ToolTier::Wood.harvest_level(),
            effective_on: vec![],
        })),
        ("minecraft:stone_sword", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Stone,
            max_damage: ToolTier::Stone.base_durability(),
            mining_speed: 1.0,
            attack_damage: 5.0,
            harvest_level: ToolTier::Stone.harvest_level(),
            effective_on: vec![],
        })),
        ("minecraft:iron_sword", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Iron,
            max_damage: ToolTier::Iron.base_durability(),
            mining_speed: 1.0,
            attack_damage: 6.0,
            harvest_level: ToolTier::Iron.harvest_level(),
            effective_on: vec![],
        })),
        ("minecraft:golden_sword", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Gold,
            max_damage: ToolTier::Gold.base_durability(),
            mining_speed: 1.0,
            attack_damage: 4.0,
            harvest_level: ToolTier::Gold.harvest_level(),
            effective_on: vec![],
        })),
        ("minecraft:diamond_sword", ItemProperties::tool(ToolProperties {
            tier: ToolTier::Diamond,
            max_damage: ToolTier::Diamond.base_durability(),
            mining_speed: 1.0,
            attack_damage: 7.0,
            harvest_level: ToolTier::Diamond.harvest_level(),
            effective_on: vec![],
        })),

        // Armor - Leather
        ("minecraft:leather_helmet", ItemProperties::armor(ArmorProperties {
            armor_type: ArmorType::Helmet,
            max_damage: ArmorMaterial::Leather.base_durability(ArmorType::Helmet),
            protection: ArmorMaterial::Leather.protection(ArmorType::Helmet),
            toughness: ArmorMaterial::Leather.toughness(),
            knockback_resistance: ArmorMaterial::Leather.knockback_resistance(),
            material: ArmorMaterial::Leather,
        })),
        ("minecraft:leather_chestplate", ItemProperties::armor(ArmorProperties {
            armor_type: ArmorType::Chestplate,
            max_damage: ArmorMaterial::Leather.base_durability(ArmorType::Chestplate),
            protection: ArmorMaterial::Leather.protection(ArmorType::Chestplate),
            toughness: ArmorMaterial::Leather.toughness(),
            knockback_resistance: ArmorMaterial::Leather.knockback_resistance(),
            material: ArmorMaterial::Leather,
        })),
        ("minecraft:leather_leggings", ItemProperties::armor(ArmorProperties {
            armor_type: ArmorType::Leggings,
            max_damage: ArmorMaterial::Leather.base_durability(ArmorType::Leggings),
            protection: ArmorMaterial::Leather.protection(ArmorType::Leggings),
            toughness: ArmorMaterial::Leather.toughness(),
            knockback_resistance: ArmorMaterial::Leather.knockback_resistance(),
            material: ArmorMaterial::Leather,
        })),
        ("minecraft:leather_boots", ItemProperties::armor(ArmorProperties {
            armor_type: ArmorType::Boots,
            max_damage: ArmorMaterial::Leather.base_durability(ArmorType::Boots),
            protection: ArmorMaterial::Leather.protection(ArmorType::Boots),
            toughness: ArmorMaterial::Leather.toughness(),
            knockback_resistance: ArmorMaterial::Leather.knockback_resistance(),
            material: ArmorMaterial::Leather,
        })),

        // Armor - Chainmail
        ("minecraft:chainmail_helmet", ItemProperties::armor(ArmorProperties {
            armor_type: ArmorType::Helmet,
            max_damage: ArmorMaterial::Chainmail.base_durability(ArmorType::Helmet),
            protection: ArmorMaterial::Chainmail.protection(ArmorType::Helmet),
            toughness: ArmorMaterial::Chainmail.toughness(),
            knockback_resistance: ArmorMaterial::Chainmail.knockback_resistance(),
            material: ArmorMaterial::Chainmail,
        })),
        ("minecraft:chainmail_chestplate", ItemProperties::armor(ArmorProperties {
            armor_type: ArmorType::Chestplate,
            max_damage: ArmorMaterial::Chainmail.base_durability(ArmorType::Chestplate),
            protection: ArmorMaterial::Chainmail.protection(ArmorType::Chestplate),
            toughness: ArmorMaterial::Chainmail.toughness(),
            knockback_resistance: ArmorMaterial::Chainmail.knockback_resistance(),
            material: ArmorMaterial::Chainmail,
        })),
        ("minecraft:chainmail_leggings", ItemProperties::armor(ArmorProperties {
            armor_type: ArmorType::Leggings,
            max_damage: ArmorMaterial::Chainmail.base_durability(ArmorType::Leggings),
            protection: ArmorMaterial::Chainmail.protection(ArmorType::Leggings),
            toughness: ArmorMaterial::Chainmail.toughness(),
            knockback_resistance: ArmorMaterial::Chainmail.knockback_resistance(),
            material: ArmorMaterial::Chainmail,
        })),
        ("minecraft:chainmail_boots", ItemProperties::armor(ArmorProperties {
            armor_type: ArmorType::Boots,
            max_damage: ArmorMaterial::Chainmail.base_durability(ArmorType::Boots),
            protection: ArmorMaterial::Chainmail.protection(ArmorType::Boots),
            toughness: ArmorMaterial::Chainmail.toughness(),
            knockback_resistance: ArmorMaterial::Chainmail.knockback_resistance(),
            material: ArmorMaterial::Chainmail,
        })),

        // Armor - Iron
        ("minecraft:iron_helmet", ItemProperties::armor(ArmorProperties {
            armor_type: ArmorType::Helmet,
            max_damage: ArmorMaterial::Iron.base_durability(ArmorType::Helmet),
            protection: ArmorMaterial::Iron.protection(ArmorType::Helmet),
            toughness: ArmorMaterial::Iron.toughness(),
            knockback_resistance: ArmorMaterial::Iron.knockback_resistance(),
            material: ArmorMaterial::Iron,
        })),
        ("minecraft:iron_chestplate", ItemProperties::armor(ArmorProperties {
            armor_type: ArmorType::Chestplate,
            max_damage: ArmorMaterial::Iron.base_durability(ArmorType::Chestplate),
            protection: ArmorMaterial::Iron.protection(ArmorType::Chestplate),
            toughness: ArmorMaterial::Iron.toughness(),
            knockback_resistance: ArmorMaterial::Iron.knockback_resistance(),
            material: ArmorMaterial::Iron,
        })),
        ("minecraft:iron_leggings", ItemProperties::armor(ArmorProperties {
            armor_type: ArmorType::Leggings,
            max_damage: ArmorMaterial::Iron.base_durability(ArmorType::Leggings),
            protection: ArmorMaterial::Iron.protection(ArmorType::Leggings),
            toughness: ArmorMaterial::Iron.toughness(),
            knockback_resistance: ArmorMaterial::Iron.knockback_resistance(),
            material: ArmorMaterial::Iron,
        })),
        ("minecraft:iron_boots", ItemProperties::armor(ArmorProperties {
            armor_type: ArmorType::Boots,
            max_damage: ArmorMaterial::Iron.base_durability(ArmorType::Boots),
            protection: ArmorMaterial::Iron.protection(ArmorType::Boots),
            toughness: ArmorMaterial::Iron.toughness(),
            knockback_resistance: ArmorMaterial::Iron.knockback_resistance(),
            material: ArmorMaterial::Iron,
        })),

        // Armor - Gold
        ("minecraft:golden_helmet", ItemProperties::armor(ArmorProperties {
            armor_type: ArmorType::Helmet,
            max_damage: ArmorMaterial::Gold.base_durability(ArmorType::Helmet),
            protection: ArmorMaterial::Gold.protection(ArmorType::Helmet),
            toughness: ArmorMaterial::Gold.toughness(),
            knockback_resistance: ArmorMaterial::Gold.knockback_resistance(),
            material: ArmorMaterial::Gold,
        })),
        ("minecraft:golden_chestplate", ItemProperties::armor(ArmorProperties {
            armor_type: ArmorType::Chestplate,
            max_damage: ArmorMaterial::Gold.base_durability(ArmorType::Chestplate),
            protection: ArmorMaterial::Gold.protection(ArmorType::Chestplate),
            toughness: ArmorMaterial::Gold.toughness(),
            knockback_resistance: ArmorMaterial::Gold.knockback_resistance(),
            material: ArmorMaterial::Gold,
        })),
        ("minecraft:golden_leggings", ItemProperties::armor(ArmorProperties {
            armor_type: ArmorType::Leggings,
            max_damage: ArmorMaterial::Gold.base_durability(ArmorType::Leggings),
            protection: ArmorMaterial::Gold.protection(ArmorType::Leggings),
            toughness: ArmorMaterial::Gold.toughness(),
            knockback_resistance: ArmorMaterial::Gold.knockback_resistance(),
            material: ArmorMaterial::Gold,
        })),
        ("minecraft:golden_boots", ItemProperties::armor(ArmorProperties {
            armor_type: ArmorType::Boots,
            max_damage: ArmorMaterial::Gold.base_durability(ArmorType::Boots),
            protection: ArmorMaterial::Gold.protection(ArmorType::Boots),
            toughness: ArmorMaterial::Gold.toughness(),
            knockback_resistance: ArmorMaterial::Gold.knockback_resistance(),
            material: ArmorMaterial::Gold,
        })),

        // Armor - Diamond
        ("minecraft:diamond_helmet", ItemProperties::armor(ArmorProperties {
            armor_type: ArmorType::Helmet,
            max_damage: ArmorMaterial::Diamond.base_durability(ArmorType::Helmet),
            protection: ArmorMaterial::Diamond.protection(ArmorType::Helmet),
            toughness: ArmorMaterial::Diamond.toughness(),
            knockback_resistance: ArmorMaterial::Diamond.knockback_resistance(),
            material: ArmorMaterial::Diamond,
        })),
        ("minecraft:diamond_chestplate", ItemProperties::armor(ArmorProperties {
            armor_type: ArmorType::Chestplate,
            max_damage: ArmorMaterial::Diamond.base_durability(ArmorType::Chestplate),
            protection: ArmorMaterial::Diamond.protection(ArmorType::Chestplate),
            toughness: ArmorMaterial::Diamond.toughness(),
            knockback_resistance: ArmorMaterial::Diamond.knockback_resistance(),
            material: ArmorMaterial::Diamond,
        })),
        ("minecraft:diamond_leggings", ItemProperties::armor(ArmorProperties {
            armor_type: ArmorType::Leggings,
            max_damage: ArmorMaterial::Diamond.base_durability(ArmorType::Leggings),
            protection: ArmorMaterial::Diamond.protection(ArmorType::Leggings),
            toughness: ArmorMaterial::Diamond.toughness(),
            knockback_resistance: ArmorMaterial::Diamond.knockback_resistance(),
            material: ArmorMaterial::Diamond,
        })),
        ("minecraft:diamond_boots", ItemProperties::armor(ArmorProperties {
            armor_type: ArmorType::Boots,
            max_damage: ArmorMaterial::Diamond.base_durability(ArmorType::Boots),
            protection: ArmorMaterial::Diamond.protection(ArmorType::Boots),
            toughness: ArmorMaterial::Diamond.toughness(),
            knockback_resistance: ArmorMaterial::Diamond.knockback_resistance(),
            material: ArmorMaterial::Diamond,
        })),

        // Food items
        ("minecraft:apple", ItemProperties::food(FoodProperties {
            nutrition: 4,
            saturation: 2.4,
            fast_eat: false,
            meat: false,
            always_edible: false,
        })),
        ("minecraft:bread", ItemProperties::food(FoodProperties {
            nutrition: 5,
            saturation: 6.0,
            fast_eat: false,
            meat: false,
            always_edible: false,
        })),
        ("minecraft:cooked_beef", ItemProperties::food(FoodProperties {
            nutrition: 8,
            saturation: 12.8,
            fast_eat: false,
            meat: true,
            always_edible: false,
        })),
        ("minecraft:cooked_chicken", ItemProperties::food(FoodProperties {
            nutrition: 6,
            saturation: 7.2,
            fast_eat: false,
            meat: true,
            always_edible: false,
        })),
        ("minecraft:cooked_cod", ItemProperties::food(FoodProperties {
            nutrition: 5,
            saturation: 6.0,
            fast_eat: false,
            meat: true,
            always_edible: false,
        })),
        ("minecraft:cooked_mutton", ItemProperties::food(FoodProperties {
            nutrition: 6,
            saturation: 9.6,
            fast_eat: false,
            meat: true,
            always_edible: false,
        })),
        ("minecraft:cooked_porkchop", ItemProperties::food(FoodProperties {
            nutrition: 8,
            saturation: 12.8,
            fast_eat: false,
            meat: true,
            always_edible: false,
        })),
        ("minecraft:cooked_rabbit", ItemProperties::food(FoodProperties {
            nutrition: 5,
            saturation: 6.0,
            fast_eat: false,
            meat: true,
            always_edible: false,
        })),
        ("minecraft:cooked_salmon", ItemProperties::food(FoodProperties {
            nutrition: 6,
            saturation: 9.6,
            fast_eat: false,
            meat: true,
            always_edible: false,
        })),
        ("minecraft:golden_apple", ItemProperties::food(FoodProperties {
            nutrition: 4,
            saturation: 9.6,
            fast_eat: false,
            meat: false,
            always_edible: true,
        })),
        ("minecraft:enchanting_golden_apple", ItemProperties::food(FoodProperties {
            nutrition: 4,
            saturation: 9.6,
            fast_eat: false,
            meat: false,
            always_edible: true,
        })),
        ("minecraft:golden_carrot", ItemProperties::food(FoodProperties {
            nutrition: 6,
            saturation: 14.4,
            fast_eat: false,
            meat: false,
            always_edible: false,
        })),
        ("minecraft:honey_bottle", ItemProperties::food(FoodProperties {
            nutrition: 6,
            saturation: 1.2,
            fast_eat: true,
            meat: false,
            always_edible: false,
        })),
        ("minecraft:melon_slice", ItemProperties::food(FoodProperties {
            nutrition: 2,
            saturation: 1.2,
            fast_eat: false,
            meat: false,
            always_edible: false,
        })),
        ("minecraft:cooked_potato", ItemProperties::food(FoodProperties {
            nutrition: 5,
            saturation: 6.0,
            fast_eat: false,
            meat: false,
            always_edible: false,
        })),
        ("minecraft:rabbit_stew", ItemProperties::food(FoodProperties {
            nutrition: 10,
            saturation: 12.0,
            fast_eat: false,
            meat: false,
            always_edible: false,
        })),
        ("minecraft:beetroot_soup", ItemProperties::food(FoodProperties {
            nutrition: 6,
            saturation: 7.2,
            fast_eat: false,
            meat: false,
            always_edible: false,
        })),
        ("minecraft:mushroom_stew", ItemProperties::food(FoodProperties {
            nutrition: 6,
            saturation: 7.2,
            fast_eat: false,
            meat: false,
            always_edible: false,
        })),
        ("minecraft:suspicious_stew", ItemProperties::food(FoodProperties {
            nutrition: 6,
            saturation: 7.2,
            fast_eat: false,
            meat: false,
            always_edible: false,
        })),
        ("minecraft:cookie", ItemProperties::food(FoodProperties {
            nutrition: 2,
            saturation: 0.4,
            fast_eat: true,
            meat: false,
            always_edible: false,
        })),
        ("minecraft:beef", ItemProperties::food(FoodProperties {
            nutrition: 3,
            saturation: 1.8,
            fast_eat: false,
            meat: true,
            always_edible: false,
        })),
        ("minecraft:chicken", ItemProperties::food(FoodProperties {
            nutrition: 2,
            saturation: 1.2,
            fast_eat: false,
            meat: true,
            always_edible: false,
        })),
        ("minecraft:cod", ItemProperties::food(FoodProperties {
            nutrition: 2,
            saturation: 0.2,
            fast_eat: false,
            meat: true,
            always_edible: false,
        })),
        ("minecraft:mutton", ItemProperties::food(FoodProperties {
            nutrition: 2,
            saturation: 1.2,
            fast_eat: false,
            meat: true,
            always_edible: false,
        })),
        ("minecraft:porkchop", ItemProperties::food(FoodProperties {
            nutrition: 3,
            saturation: 1.8,
            fast_eat: false,
            meat: true,
            always_edible: false,
        })),
        ("minecraft:rabbit", ItemProperties::food(FoodProperties {
            nutrition: 3,
            saturation: 1.8,
            fast_eat: false,
            meat: true,
            always_edible: false,
        })),
        ("minecraft:salmon", ItemProperties::food(FoodProperties {
            nutrition: 2,
            saturation: 0.2,
            fast_eat: false,
            meat: true,
            always_edible: false,
        })),
        ("minecraft:tropical_fish", ItemProperties::food(FoodProperties {
            nutrition: 1,
            saturation: 0.2,
            fast_eat: false,
            meat: true,
            always_edible: false,
        })),
        ("minecraft:pufferfish", ItemProperties::food(FoodProperties {
            nutrition: 1,
            saturation: 0.2,
            fast_eat: false,
            meat: true,
            always_edible: false,
        })),
        ("minecraft:rotten_flesh", ItemProperties::food(FoodProperties {
            nutrition: 4,
            saturation: 0.8,
            fast_eat: false,
            meat: true,
            always_edible: false,
        })),
        ("minecraft:spider_eye", ItemProperties::food(FoodProperties {
            nutrition: 2,
            saturation: 3.2,
            fast_eat: false,
            meat: false,
            always_edible: false,
        })),
        ("minecraft:carrot", ItemProperties::food(FoodProperties {
            nutrition: 3,
            saturation: 3.6,
            fast_eat: false,
            meat: false,
            always_edible: false,
        })),
        ("minecraft:potato", ItemProperties::food(FoodProperties {
            nutrition: 1,
            saturation: 0.6,
            fast_eat: false,
            meat: false,
            always_edible: false,
        })),
        ("minecraft:baked_potato", ItemProperties::food(FoodProperties {
            nutrition: 5,
            saturation: 6.0,
            fast_eat: false,
            meat: false,
            always_edible: false,
        })),
        ("minecraft:poisonous_potato", ItemProperties::food(FoodProperties {
            nutrition: 2,
            saturation: 1.2,
            fast_eat: false,
            meat: false,
            always_edible: false,
        })),
        ("minecraft:beetroot", ItemProperties::food(FoodProperties {
            nutrition: 1,
            saturation: 1.2,
            fast_eat: false,
            meat: false,
            always_edible: false,
        })),
        ("minecraft:dried_kelp", ItemProperties::food(FoodProperties {
            nutrition: 1,
            saturation: 0.6,
            fast_eat: true,
            meat: false,
            always_edible: false,
        })),
        ("minecraft:sweet_berries", ItemProperties::food(FoodProperties {
            nutrition: 2,
            saturation: 0.4,
            fast_eat: true,
            meat: false,
            always_edible: false,
        })),
        ("minecraft:glow_berries", ItemProperties::food(FoodProperties {
            nutrition: 2,
            saturation: 0.4,
            fast_eat: true,
            meat: false,
            always_edible: false,
        })),
        ("minecraft:chorus_fruit", ItemProperties::food(FoodProperties {
            nutrition: 4,
            saturation: 2.4,
            fast_eat: true,
            meat: false,
            always_edible: false,
        })),
        ("minecraft:honeycomb", ItemProperties::simple()),
        ("minecraft:kelp", ItemProperties::food(FoodProperties {
            nutrition: 1,
            saturation: 0.6,
            fast_eat: true,
            meat: false,
            always_edible: false,
        })),
    ]
}

/// Number of items in the baseline vanilla set.
pub const ITEM_COUNT: usize = 210;

/// Registers every baseline item into `registry`, returning the number added.
///
/// Entries are registered in the frozen [`item_table`] order, so indices are
/// reproducible. Callers that add mod content afterwards get the next indices.
pub fn register_items(registry: &mut Registry<ItemProperties>) -> Result<u32, ContentError> {
    let table = item_table();
    debug_assert_eq!(table.len(), ITEM_COUNT);
    let mut added = 0;
    for (id, properties) in table {
        registry.register(Identifier::parse(id)?, properties)?;
        added += 1;
    }
    Ok(added)
}

/// Builds a frozen item registry containing only vanilla items.
pub fn item_registry() -> Result<mclre_core::registry::FrozenRegistry<ItemProperties>, ContentError> {
    let mut registry = Registry::new("minecraft:item");
    register_items(&mut registry)?;
    registry.freeze().map_err(ContentError::Registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mclre_core::{BlockId, BlockStateId, PropertyValue};

    fn states() -> BlockStateModel {
        block_states().expect("vanilla content freezes")
    }

    #[test]
    fn registers_the_full_block_set() {
        let model = states();
        assert_eq!(
            model.block_count(),
            u32::try_from(BLOCK_COUNT).expect("fits")
        );
        let ids = model.registry().ids();
        assert_eq!(ids[0].to_string_form(), "minecraft:air");
        assert_eq!(ids.len(), BLOCK_COUNT);
        // Registration order is the table order.
        assert_eq!(ids[1].to_string_form(), "minecraft:stone");
        assert_eq!(ids[BLOCK_COUNT - 1].to_string_form(), "minecraft:wheat");
    }

    #[test]
    fn per_block_state_counts_match_property_products() {
        let model = states();
        let registry = model.registry();
        let expected = [
            ("minecraft:air", 1),
            ("minecraft:grass_block", 2),
            ("minecraft:water", 16),
            ("minecraft:lava", 16),
            ("minecraft:oak_log", 3),
            ("minecraft:oak_leaves", 14), // 7 distances x 2 persistent
            ("minecraft:oak_stairs", 40),
            ("minecraft:stone_slab", 3),
            ("minecraft:furnace", 8),
            ("minecraft:torch", 5),
            ("minecraft:chest", 4),
            ("minecraft:wheat", 8),
        ];
        for (id, count) in expected {
            let identifier = Identifier::parse(id).expect("valid literal");
            let (properties, index) = registry.get(&identifier).expect("registered");
            assert_eq!(
                properties.state_count(),
                count,
                "wrong state count for {id}"
            );
            let first = model.default_state(BlockId(index)).expect("has states");
            let last = BlockStateId(first.0 + u32::try_from(count).expect("fits") - 1);
            assert_eq!(model.block_of(first), Some(BlockId(index)));
            assert_eq!(model.block_of(last), Some(BlockId(index)));
        }
    }

    #[test]
    fn state_ids_are_contiguous_and_cover_every_block() {
        let model = states();
        let total: u32 = model
            .registry()
            .values()
            .iter()
            .map(|p| u32::try_from(p.state_count()).expect("fits"))
            .sum();
        assert_eq!(model.state_count(), total);
        for state in 0..model.state_count() {
            assert!(
                model.block_of(BlockStateId(state)).is_some(),
                "state {state} has no block"
            );
        }
        assert_eq!(model.block_of(BlockStateId(total)), None);
    }

    #[test]
    fn every_state_round_trips_through_property_values() {
        let model = states();
        for state in 0..model.state_count() {
            let state = BlockStateId(state);
            let block = model.block_of(state).expect("state has a block");
            let values = model.values_of(state).expect("state has values");
            let recomputed = model
                .state_from_values(block, &values)
                .expect("values are valid for their block");
            assert_eq!(recomputed, state, "round trip failed for {state:?}");
        }
    }

    #[test]
    fn default_states_use_first_canonical_values() {
        let model = states();
        let registry = model.registry();
        let oak_log = Identifier::parse("minecraft:oak_log").expect("valid");
        let (_, index) = registry.get(&oak_log).expect("registered");
        let default = model.default_state(BlockId(index)).expect("has states");
        assert_eq!(
            model.values_of(default),
            Some(vec![PropertyValue::Enum("x".into())])
        );

        let furnace = Identifier::parse("minecraft:furnace").expect("valid");
        let (_, index) = registry.get(&furnace).expect("registered");
        let default = model.default_state(BlockId(index)).expect("has states");
        assert_eq!(
            model.values_of(default),
            Some(vec![
                PropertyValue::Enum("north".into()),
                PropertyValue::Bool(false)
            ])
        );
    }

    #[test]
    fn declaring_states_is_idempotent_across_builds() {
        // State ids must be reproducible run to run and platform to platform.
        let first = states();
        let second = states();
        assert_eq!(first.state_count(), second.state_count());
        for state in 0..first.state_count() {
            let state = BlockStateId(state);
            assert_eq!(first.block_of(state), second.block_of(state));
            assert_eq!(first.values_of(state), second.values_of(state));
        }
    }

    #[test]
    fn registering_into_an_existing_registry_appends_after_mod_content() {
        let mut registry = Registry::new("minecraft:block");
        registry
            .register(
                Identifier::parse("testmod:first").expect("valid"),
                BlockProperties::none(),
            )
            .expect("register mod block");
        let added = register(&mut registry).expect("vanilla registers");
        assert_eq!(added as usize, BLOCK_COUNT);
        let model = BlockStateModel::freeze(registry).expect("freeze");
        assert_eq!(model.registry().ids()[0].to_string_form(), "testmod:first");
        assert_eq!(model.registry().ids()[1].to_string_form(), "minecraft:air");
    }

    #[test]
    fn registers_all_baseline_items() {
        let registry = item_registry().expect("items freeze");
        assert_eq!(registry.len(), ITEM_COUNT);
        let ids = registry.ids();
        assert_eq!(ids[0].to_string_form(), "minecraft:iron_ingot");
        assert_eq!(ids[ITEM_COUNT - 1].to_string_form(), "minecraft:kelp");
    }

    #[test]
    fn item_properties_are_correct() {
        let registry = item_registry().expect("items freeze");
        
        // Test a few key items
        let diamond_sword = Identifier::parse("minecraft:diamond_sword").expect("valid");
        let (props, _) = registry.get(&diamond_sword).expect("registered");
        assert!(props.tool.is_some());
        let tool = props.tool.as_ref().expect("is tool");
        assert_eq!(tool.tier, ToolTier::Diamond);
        assert_eq!(tool.max_damage, 1561);
        assert_eq!(tool.attack_damage, 7.0);
        
        let iron_chestplate = Identifier::parse("minecraft:iron_chestplate").expect("valid");
        let (props, _) = registry.get(&iron_chestplate).expect("registered");
        assert!(props.armor.is_some());
        let armor = props.armor.as_ref().expect("is armor");
        assert_eq!(armor.armor_type, ArmorType::Chestplate);
        assert_eq!(armor.protection, 6);
        assert_eq!(armor.material, ArmorMaterial::Iron);
        
        let apple = Identifier::parse("minecraft:apple").expect("valid");
        let (props, _) = registry.get(&apple).expect("registered");
        assert!(props.food.is_some());
        let food = props.food.as_ref().expect("is food");
        assert_eq!(food.nutrition, 4);
        assert_eq!(food.saturation, 2.4);
    }

    #[test]
    fn declaring_items_is_idempotent_across_builds() {
        let first = item_registry().expect("items freeze");
        let second = item_registry().expect("items freeze");
        assert_eq!(first.len(), second.len());
        for (a, b) in first.ids().iter().zip(second.ids().iter()) {
            assert_eq!(a, b);
        }
    }
}
