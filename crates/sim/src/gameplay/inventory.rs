//! Inventory and container system for MCLRE baseline profile.

use crate::{SimWorld, SimError, SimPlayer};
use mclre_core::{Identifier, BlockId, BlockStateId};
use mclre_world::{BlockPos, ChunkPos};
use std::collections::HashMap;

/// Maximum stack size for items (can be overridden per-item).
pub const DEFAULT_MAX_STACK_SIZE: u8 = 64;

/// Inventory slot containing an item stack.
#[derive(Debug, Clone, Default)]
pub struct ItemStack {
    /// Item identifier.
    pub item: Option<Identifier>,
    /// Number of items in this stack.
    pub count: u8,
    /// Item damage (for tools/armor) or metadata.
    pub damage: u16,
    /// NBT data for complex items (enchanted books, written books, etc.).
    pub nbt: Option<Vec<u8>>,
}

impl ItemStack {
    /// Create an empty item stack.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Create a new item stack.
    pub fn new(item: Identifier, count: u8, damage: u16) -> Self {
        Self {
            item: Some(item),
            count,
            damage,
            nbt: None,
        }
    }

    /// Check if the stack is empty.
    pub fn is_empty(&self) -> bool {
        self.item.is_none() || self.count == 0
    }

    /// Get the maximum stack size for this item.
    pub fn max_stack_size(&self) -> u8 {
        if let Some(item) = self.item {
            return get_max_stack_size(item);
        }
        DEFAULT_MAX_STACK_SIZE
    }

    /// Try to add items to this stack.
    /// Returns the number of items that couldn't fit.
    pub fn add(&mut self, count: u8) -> u8 {
        if self.is_empty() {
            return count;
        }
        
        let space = self.max_stack_size().saturating_sub(self.count);
        let added = count.min(space);
        self.count += added;
        count - added
    }

    /// Try to remove items from this stack.
    /// Returns the number of items actually removed.
    pub fn remove(&mut self, count: u8) -> u8 {
        let removed = count.min(self.count);
        self.count -= removed;
        if self.count == 0 {
            self.item = None;
            self.damage = 0;
            self.nbt = None;
        }
        removed
    }

    /// Check if this stack can stack with another.
    pub fn can_stack_with(&self, other: &ItemStack) -> bool {
        if self.is_empty() || other.is_empty() {
            return false;
        }
        
        if self.item != other.item {
            return false;
        }
        
        if self.damage != other.damage {
            return false;
        }
        
        // TODO: Compare NBT data
        
        true
    }

    /// Split the stack, returning a new stack with `count` items.
    pub fn split(&mut self, count: u8) -> Option<ItemStack> {
        if count == 0 || count >= self.count {
            return None;
        }
        
        self.count -= count;
        Some(ItemStack {
            item: self.item,
            count,
            damage: self.damage,
            nbt: self.nbt.clone(),
        })
    }

    /// Get the item identifier, or None if empty.
    pub fn item_id(&self) -> Option<Identifier> {
        self.item
    }
}

/// Armor slot indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmorSlot {
    Feet = 0,
    Legs = 1,
    Chest = 2,
    Head = 3,
}

impl ArmorSlot {
    /// Get all armor slots in order (feet, legs, chest, head).
    pub fn all() -> [ArmorSlot; 4] {
        [ArmorSlot::Feet, ArmorSlot::Legs, ArmorSlot::Chest, ArmorSlot::Head]
    }
}

/// Player inventory (36 slots + 4 armor + offhand + crafting grid).
#[derive(Debug, Clone)]
pub struct PlayerInventory {
    /// Main inventory (36 slots: 9 hotbar + 27 main).
    pub main: [ItemStack; 36],
    /// Armor slots: [feet, legs, chest, head].
    pub armor: [ItemStack; 4],
    /// Offhand slot.
    pub offhand: ItemStack,
    /// Crafting input grid (4 slots for 2x2).
    pub crafting_input: [ItemStack; 4],
    /// Crafting output slot.
    pub crafting_output: ItemStack,
    /// Currently selected hotbar slot (0-8).
    pub selected_slot: u8,
}

impl Default for PlayerInventory {
    fn default() -> Self {
        Self {
            main: std::array::from_fn(|_| ItemStack::empty()),
            armor: std::array::from_fn(|_| ItemStack::empty()),
            offhand: ItemStack::empty(),
            crafting_input: std::array::from_fn(|_| ItemStack::empty()),
            crafting_output: ItemStack::empty(),
            selected_slot: 0,
        }
    }
}

impl PlayerInventory {
    /// Get the item stack in the selected hotbar slot.
    pub fn selected_item(&self) -> &ItemStack {
        &self.main[self.selected_slot as usize]
    }

    /// Get mutable reference to selected hotbar slot.
    pub fn selected_item_mut(&mut self) -> &mut ItemStack {
        &mut self.main[self.selected_slot as usize]
    }

    /// Get the item in the main hand.
    pub fn main_hand(&self) -> &ItemStack {
        self.selected_item()
    }

    /// Get the item in the off hand.
    pub fn off_hand(&self) -> &ItemStack {
        &self.offhand
    }

    /// Set the selected hotbar slot (0-8).
    pub fn set_selected_slot(&mut self, slot: u8) {
        self.selected_slot = slot.min(8);
    }

    /// Get armor in a specific slot.
    pub fn get_armor(&self, slot: ArmorSlot) -> &ItemStack {
        &self.armor[slot as usize]
    }

    /// Get mutable armor in a specific slot.
    pub fn get_armor_mut(&mut self, slot: ArmorSlot) -> &mut ItemStack {
        &mut self.armor[slot as usize]
    }

    /// Set armor in a specific slot.
    pub fn set_armor(&mut self, slot: ArmorSlot, stack: ItemStack) {
        self.armor[slot as usize] = stack;
    }

    /// Calculate total armor points (0-20).
    /// Each armor piece provides different protection.
    pub fn total_armor_points(&self) -> u8 {
        let mut total = 0u8;
        for armor in &self.armor {
            if let Some(item) = armor.item {
                // TODO: Look up armor protection value from item registry
                // For now, use placeholder values based on material
                total = total.saturating_add(get_armor_protection(item));
            }
        }
        total.min(20)
    }

    /// Calculate armor toughness.
    pub fn total_armor_toughness(&self) -> f32 {
        let mut total = 0.0f32;
        for armor in &self.armor {
            if let Some(item) = armor.item {
                // TODO: Look up armor toughness from item registry
                total += get_armor_toughness(item);
            }
        }
        total
    }

    /// Calculate knockback resistance.
    pub fn knockback_resistance(&self) -> f32 {
        let mut total = 0.0f32;
        for armor in &self.armor {
            if let Some(item) = armor.item {
                total += get_knockback_resistance(item);
            }
        }
        total
    }

    /// Try to add an item stack to the inventory.
    /// Returns the remaining items that couldn't fit.
    pub fn add_item(&mut self, mut stack: ItemStack) -> ItemStack {
        if stack.is_empty() {
            return ItemStack::empty();
        }

        // In creative mode, items are infinite
        // (This is handled by the caller checking gamemode)

        // First try to stack with existing items
        for slot in &mut self.main {
            if slot.can_stack_with(&stack) {
                let remaining = slot.add(stack.count);
                stack.count = remaining;
                if stack.is_empty() {
                    return ItemStack::empty();
                }
            }
        }
        
        // Then find empty slot
        for slot in &mut self.main {
            if slot.is_empty() {
                *slot = stack;
                return ItemStack::empty();
            }
        }
        
        // No space
        stack
    }

    /// Try to add an item stack to a specific slot.
    /// Returns the remaining items.
    pub fn add_item_to_slot(&mut self, slot: usize, mut stack: ItemStack) -> ItemStack {
        if slot >= self.main.len() {
            return stack;
        }
        
        if self.main[slot].can_stack_with(&stack) {
            let remaining = self.main[slot].add(stack.count);
            stack.count = remaining;
            if stack.is_empty() {
                return ItemStack::empty();
            }
        }
        
        if self.main[slot].is_empty() {
            self.main[slot] = stack;
            return ItemStack::empty();
        }
        
        stack
    }

    /// Remove items from a specific slot.
    pub fn remove_from_slot(&mut self, slot: usize, count: u8) -> u8 {
        if slot < self.main.len() {
            self.main[slot].remove(count)
        } else {
            0
        }
    }

    /// Swap two slots.
    pub fn swap_slots(&mut self, a: usize, b: usize) {
        if a < self.main.len() && b < self.main.len() {
            self.main.swap(a, b);
        }
    }

    /// Move items between inventory and container (shift-click behavior).
    /// Returns items that couldn't be moved.
    pub fn shift_click_move(
        &mut self,
        container: &mut ContainerInventory,
        slot: usize,
    ) -> ItemStack {
        if slot >= container.size() {
            return ItemStack::empty();
        }
        
        let mut stack = container.slots[slot].clone();
        if stack.is_empty() {
            return ItemStack::empty();
        }
        
        container.slots[slot] = ItemStack::empty();
        
        // Try to add to player inventory
        stack = self.add_item(stack);
        
        // If couldn't fit all, put remainder back
        if !stack.is_empty() {
            container.slots[slot] = stack.clone();
        }
        
        stack
    }

    /// Get all non-empty slots for serialization.
    pub fn get_non_empty_slots(&self) -> Vec<(usize, &ItemStack)> {
        self.main.iter().enumerate()
            .filter(|(_, s)| !s.is_empty())
            .collect()
    }
}

/// Container types for block entities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerType {
    Chest,
    DoubleChest,
    Furnace,
    Dispenser,
    Dropper,
    CraftingTable,
    EnchantingTable,
    BrewingStand,
    Anvil,
    Hopper,
    /// Custom container with specific slot count.
    Custom(usize),
}

/// Generic container inventory.
#[derive(Debug, Clone)]
pub struct ContainerInventory {
    /// Container type.
    pub container_type: ContainerType,
    /// Slots in the container.
    pub slots: Vec<ItemStack>,
    /// Custom name (for named containers).
    pub custom_name: Option<String>,
    /// Lock (for locked containers).
    pub lock: Option<String>,
}

impl ContainerInventory {
    /// Create a new container with the given number of slots.
    pub fn new(container_type: ContainerType, slots: usize) -> Self {
        Self {
            container_type,
            slots: vec![ItemStack::empty(); slots],
            custom_name: None,
            lock: None,
        }
    }

    /// Get the number of slots.
    pub fn size(&self) -> usize {
        self.slots.len()
    }

    /// Get a slot.
    pub fn get_slot(&self, index: usize) -> Option<&ItemStack> {
        self.slots.get(index)
    }

    /// Get a mutable slot.
    pub fn get_slot_mut(&mut self, index: usize) -> Option<&mut ItemStack> {
        self.slots.get_mut(index)
    }

    /// Set a slot.
    pub fn set_slot(&mut self, index: usize, stack: ItemStack) -> bool {
        if let Some(slot) = self.slots.get_mut(index) {
            *slot = stack;
            true
        } else {
            false
        }
    }

    /// Try to add an item to the container.
    pub fn add_item(&mut self, mut stack: ItemStack) -> ItemStack {
        if stack.is_empty() {
            return ItemStack::empty();
        }
        
        // Try to stack first
        for slot in &mut self.slots {
            if slot.can_stack_with(&stack) {
                let remaining = slot.add(stack.count);
                stack.count = remaining;
                if stack.is_empty() {
                    return ItemStack::empty();
                }
            }
        }
        
        // Find empty slot
        for slot in &mut self.slots {
            if slot.is_empty() {
                *slot = stack;
                return ItemStack::empty();
            }
        }
        
        stack
    }

    /// Try to add an item to a specific slot.
    pub fn add_item_to_slot(&mut self, slot: usize, mut stack: ItemStack) -> ItemStack {
        if slot >= self.slots.len() {
            return stack;
        }
        
        if self.slots[slot].can_stack_with(&stack) {
            let remaining = self.slots[slot].add(stack.count);
            stack.count = remaining;
            if stack.is_empty() {
                return ItemStack::empty();
            }
        }
        
        if self.slots[slot].is_empty() {
            self.slots[slot] = stack;
            return ItemStack::empty();
        }
        
        stack
    }

    /// Remove items from a slot.
    pub fn remove_from_slot(&mut self, slot: usize, count: u8) -> u8 {
        if let Some(s) = self.slots.get_mut(slot) {
            s.remove(count)
        } else {
            0
        }
    }

    /// Swap two slots within the container.
    pub fn swap_slots(&mut self, a: usize, b: usize) {
        if a < self.slots.len() && b < self.slots.len() {
            self.slots.swap(a, b);
        }
    }

    /// Get all non-empty slots for serialization.
    pub fn get_non_empty_slots(&self) -> Vec<(usize, &ItemStack)> {
        self.slots.iter().enumerate()
            .filter(|(_, s)| !s.is_empty())
            .collect()
    }
}

/// Chest container (27 slots for single, 54 for double).
pub fn create_chest(is_double: bool) -> ContainerInventory {
    let slots = if is_double { 54 } else { 27 };
    ContainerInventory::new(
        if is_double { ContainerType::DoubleChest } else { ContainerType::Chest },
        slots,
    )
}

/// Furnace container: [input, fuel, output] = 3 slots.
pub fn create_furnace() -> ContainerInventory {
    ContainerInventory::new(ContainerType::Furnace, 3)
}

/// Furnace slot indices.
pub mod furnace_slots {
    pub const INPUT: usize = 0;
    pub const FUEL: usize = 1;
    pub const OUTPUT: usize = 2;
}

/// Crafting table container: 9 input + 1 output = 10 slots.
pub fn create_crafting_table() -> ContainerInventory {
    ContainerInventory::new(ContainerType::CraftingTable, 10)
}

/// Dispenser/Dropper container: 9 slots.
pub fn create_dispenser() -> ContainerInventory {
    ContainerInventory::new(ContainerType::Dispenser, 9)
}

/// Brewing stand container: 3 potion slots + 1 ingredient + 1 fuel = 5 slots.
pub fn create_brewing_stand() -> ContainerInventory {
    ContainerInventory::new(ContainerType::BrewingStand, 5)
}

/// Brewing stand slot indices.
pub mod brewing_slots {
    pub const POTION_1: usize = 0;
    pub const POTION_2: usize = 1;
    pub const POTION_3: usize = 2;
    pub const INGREDIENT: usize = 3;
    pub const FUEL: usize = 4; // Blaze powder
}

/// Anvil container: 2 input + 1 output = 3 slots.
pub fn create_anvil() -> ContainerInventory {
    ContainerInventory::new(ContainerType::Anvil, 3)
}

/// Anvil slot indices.
pub mod anvil_slots {
    pub const INPUT_1: usize = 0;
    pub const INPUT_2: usize = 1;
    pub const OUTPUT: usize = 2;
}

/// Enchanting table container: 1 input + 1 lapis + 1 output = 3 slots (virtual).
pub fn create_enchanting_table() -> ContainerInventory {
    ContainerInventory::new(ContainerType::EnchantingTable, 3)
}

/// Hopper container: 5 slots.
pub fn create_hopper() -> ContainerInventory {
    ContainerInventory::new(ContainerType::Hopper, 5)
}

/// Get armor protection value for an item.
fn get_armor_protection(item: Identifier) -> u8 {
    // Placeholder values based on material
    // TODO: Look up from item registry
    let name = item.to_string();
    if name.contains("diamond") {
        match name.as_str() {
            s if s.contains("helmet") => 3,
            s if s.contains("chestplate") => 8,
            s if s.contains("leggings") => 6,
            s if s.contains("boots") => 3,
            _ => 0,
        }
    } else if name.contains("netherite") {
        match name.as_str() {
            s if s.contains("helmet") => 3,
            s if s.contains("chestplate") => 8,
            s if s.contains("leggings") => 6,
            s if s.contains("boots") => 3,
            _ => 0,
        }
    } else if name.contains("iron") {
        match name.as_str() {
            s if s.contains("helmet") => 2,
            s if s.contains("chestplate") => 6,
            s if s.contains("leggings") => 5,
            s if s.contains("boots") => 2,
            _ => 0,
        }
    } else if name.contains("gold") {
        match name.as_str() {
            s if s.contains("helmet") => 2,
            s if s.contains("chestplate") => 5,
            s if s.contains("leggings") => 3,
            s if s.contains("boots") => 1,
            _ => 0,
        }
    } else if name.contains("leather") {
        match name.as_str() {
            s if s.contains("helmet") => 1,
            s if s.contains("chestplate") => 3,
            s if s.contains("leggings") => 2,
            s if s.contains("boots") => 1,
            _ => 0,
        }
    } else if name.contains("chainmail") {
        match name.as_str() {
            s if s.contains("helmet") => 2,
            s if s.contains("chestplate") => 5,
            s if s.contains("leggings") => 4,
            s if s.contains("boots") => 1,
            _ => 0,
        }
    } else if name.contains("turtle") {
        match name.as_str() {
            s if s.contains("helmet") => 2,
            _ => 0,
        }
    } else {
        0
    }
}

/// Get armor toughness for an item.
fn get_armor_toughness(item: Identifier) -> f32 {
    let name = item.to_string();
    if name.contains("diamond") || name.contains("netherite") {
        2.0
    } else {
        0.0
    }
}

/// Get knockback resistance for an item.
fn get_knockback_resistance(item: Identifier) -> f32 {
    let name = item.to_string();
    if name.contains("netherite") {
        0.1 // Netherite provides knockback resistance
    } else {
        0.0
    }
}

/// Get max stack size for an item.
pub fn get_max_stack_size(item: Identifier) -> u8 {
    let name = item.to_string();
    // Items that stack to 16
    if name.contains("ender_pearl") || name.contains("snowball") || name.contains("egg") ||
       name.contains("honey_bottle") || name.contains("bucket") || name.contains("sign") ||
       name.contains("banner") || name.contains("door") || name.contains("bed") ||
       name.contains("boat") || name.contains("minecart") || name.contains("saddle") ||
       name.contains("horse_armor") || name.contains("elytra") || name.contains("totem") {
        return 16;
    }
    // Items that stack to 1 (tools, armor, weapons, etc.)
    if name.contains("_sword") || name.contains("_pickaxe") || name.contains("_axe") ||
       name.contains("_shovel") || name.contains("_hoe") || name.contains("_helmet") ||
       name.contains("_chestplate") || name.contains("_leggings") || name.contains("_boots") ||
       name.contains("_horse_armor") || name.contains("shield") || name.contains("bow") ||
       name.contains("crossbow") || name.contains("trident") || name.contains("fishing_rod") ||
       name.contains("carrot_on_a_stick") || name.contains("warped_fungus_on_a_stick") ||
       name.contains("shears") || name.contains("flint_and_steel") || name.contains("clock") ||
       name.contains("compass") || name.contains("recovery_compass") || name.contains("map") ||
       name.contains("written_book") || name.contains("enchanted_book") || name.contains("music_disc") {
        return 1;
    }
    DEFAULT_MAX_STACK_SIZE
}

/// Item drop on the ground.
#[derive(Debug, Clone)]
pub struct ItemEntity {
    /// The item stack.
    pub stack: ItemStack,
    /// Position in world.
    pub position: mclre_core::mth::Vec3<f32>,
    /// Velocity.
    pub velocity: mclre_core::mth::Vec3<f32>,
    /// Age in ticks (despawns after 6000 ticks = 5 minutes).
    pub age: u32,
    /// Pickup delay (ticks before can be picked up).
    pub pickup_delay: u16,
    /// Unique entity ID.
    pub entity_id: i32,
}

impl ItemEntity {
    pub fn new(stack: ItemStack, position: mclre_core::mth::Vec3<f32>, entity_id: i32) -> Self {
        Self {
            stack,
            position,
            velocity: mclre_core::mth::Vec3::new(0.0, 0.0, 0.0),
            age: 0,
            pickup_delay: 10, // 0.5 seconds
            entity_id,
        }
    }

    /// Tick the item entity.
    pub fn tick(&mut self, world: &SimWorld) -> bool {
        self.age += 1;
        if self.age >= 6000 {
            return true; // Mark for despawn
        }
        
        if self.pickup_delay > 0 {
            self.pickup_delay -= 1;
        }
        
        // Apply gravity
        self.velocity.y -= crate::physics::GRAVITY;
        if self.velocity.y < -crate::physics::TERMINAL_VELOCITY {
            self.velocity.y = -crate::physics::TERMINAL_VELOCITY;
        }
        
        // Apply velocity
        self.position += self.velocity;
        
        // TODO: Collision detection with blocks
        // Simple ground check
        if self.position.y <= 64.0 && self.velocity.y <= 0.0 {
            self.position.y = 64.0;
            self.velocity.y = 0.0;
            self.velocity.x *= 0.98;
            self.velocity.z *= 0.98;
        }
        
        false
    }
}

/// Spawn item drops at a position.
pub fn spawn_item_drops(
    world: &mut SimWorld,
    position: mclre_core::mth::Vec3<f32>,
    stacks: Vec<ItemStack>,
) -> Vec<ItemEntity> {
    let mut entities = Vec::new();
    let mut entity_id = world.next_entity_id();
    
    for stack in stacks {
        if !stack.is_empty() {
            entities.push(ItemEntity::new(stack, position, entity_id));
            entity_id += 1;
        }
    }
    
    entities
}

/// Drop all items from an inventory at a position (e.g., on death).
pub fn drop_inventory_contents(
    world: &mut SimWorld,
    position: mclre_core::mth::Vec3<f32>,
    inventory: &mut PlayerInventory,
) -> Vec<ItemEntity> {
    let mut all_stacks = Vec::new();
    
    // Main inventory
    for slot in &mut inventory.main {
        if !slot.is_empty() {
            all_stacks.push(std::mem::take(slot));
        }
    }
    
    // Armor
    for slot in &mut inventory.armor {
        if !slot.is_empty() {
            all_stacks.push(std::mem::take(slot));
        }
    }
    
    // Offhand
    if !inventory.offhand.is_empty() {
        all_stacks.push(std::mem::take(&mut inventory.offhand));
    }
    
    // Crafting grid
    for slot in &mut inventory.crafting_input {
        if !slot.is_empty() {
            all_stacks.push(std::mem::take(slot));
        }
    }
    if !inventory.crafting_output.is_empty() {
        all_stacks.push(std::mem::take(&mut inventory.crafting_output));
    }
    
    spawn_item_drops(world, position, all_stacks)
}