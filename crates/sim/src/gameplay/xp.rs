//! Experience points and levels for MCLRE baseline profile.

use crate::{SimError, SimPlayer, SimWorld};
use mclre_core::GameRandom;
use mclre_core::mth::Vec3;
use std::collections::HashMap;

/// XP curve for baseline profile (pre-1.9, no XP curve changes).
/// Level N requires total XP = sum of level costs up to N.
/// Formula from reference (Player.cpp getXpForLevel):
/// All levels >= 1: cost = 2*level + 5
/// (Matches explicit table for levels 1-20 in docs/reference/xp.md)

/// XP required to reach a specific level (total XP).
pub fn xp_for_level(level: u32) -> u32 {
    if level == 0 {
        return 0;
    }

    let mut total = 0u32;
    for l in 1..=level {
        total += xp_cost_for_level(l);
    }
    total
}

/// XP cost to go from level-1 to level.
pub fn xp_cost_for_level(level: u32) -> u32 {
    if level >= 1 { 2 * level + 5 } else { 0 }
}

/// Calculate current level from total XP.
pub fn level_from_xp(total_xp: u32) -> u32 {
    let mut level = 0u32;
    let mut remaining = total_xp;

    loop {
        let cost = xp_cost_for_level(level + 1);
        if remaining >= cost {
            remaining -= cost;
            level += 1;
        } else {
            break;
        }
    }

    level
}

/// Get progress to next level (0.0 to 1.0).
pub fn progress_to_next_level(total_xp: u32) -> f32 {
    let level = level_from_xp(total_xp);
    let xp_for_current = xp_for_level(level);
    let xp_for_next = xp_for_level(level + 1);

    if xp_for_next == xp_for_current {
        return 1.0;
    }

    (total_xp - xp_for_current) as f32 / (xp_for_next - xp_for_current) as f32
}

/// XP orb values (dropped by mobs, mining, smelting, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum XpOrbValue {
    Tiny = 1,       // Green
    Small = 3,      // Yellow
    Medium = 7,     // Orange
    Large = 17,     // Red
    Huge = 37,      // Blue
    Massive = 73,   // Purple
    Enormous = 149, // White
}

impl XpOrbValue {
    /// Get the XP value of this orb type.
    pub fn value(self) -> u32 {
        self as u32
    }

    /// Get the color index for rendering (0-6).
    pub fn color_index(self) -> u8 {
        match self {
            XpOrbValue::Tiny => 0,     // Green
            XpOrbValue::Small => 1,    // Yellow
            XpOrbValue::Medium => 2,   // Orange
            XpOrbValue::Large => 3,    // Red
            XpOrbValue::Huge => 4,     // Blue
            XpOrbValue::Massive => 5,  // Purple
            XpOrbValue::Enormous => 6, // White
        }
    }
}

/// Convert XP amount to optimal orb split (greedy algorithm).
pub fn split_xp_into_orbs(mut xp: u32) -> Vec<XpOrbValue> {
    let mut orbs = Vec::new();

    let values = [
        (XpOrbValue::Enormous, 149),
        (XpOrbValue::Massive, 73),
        (XpOrbValue::Huge, 37),
        (XpOrbValue::Large, 17),
        (XpOrbValue::Medium, 7),
        (XpOrbValue::Small, 3),
        (XpOrbValue::Tiny, 1),
    ];

    for (orb, value) in values {
        while xp >= value {
            orbs.push(orb);
            xp -= value;
        }
    }

    orbs
}

/// XP sources for baseline profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XpSource {
    /// Killing mobs.
    MobKill,
    /// Mining ores.
    Mining,
    /// Smelting.
    Smelting,
    /// Breeding animals.
    Breeding,
    /// Fishing.
    Fishing,
    /// Trading with villagers.
    Trading,
    /// Using bottles o' enchanting.
    Bottle,
    /// Breaking blocks (diamond ore, etc.).
    BlockBreak,
    /// Using grindstone (later TU).
    Grindstone,
}

/// XP reward amounts for baseline activities.
pub fn xp_reward(source: XpSource, rng: &mut crate::GameRandom) -> u32 {
    match source {
        // Mobs (baseline values)
        XpSource::MobKill => {
            // Varies by mob type - placeholder
            (rng.next_int_bound(5) + 1) as u32
        }
        XpSource::Mining => {
            // Ores: coal=0, iron=3, gold=3, diamond=7, emerald=7, redstone=3, lapis=3
            (rng.next_int_bound(4) + 2) as u32
        }
        XpSource::Smelting => {
            // 0.1-1.0 per item typically
            (rng.next_int_bound(2) + 1) as u32
        }
        XpSource::Breeding => (rng.next_int_bound(3) + 1) as u32,
        XpSource::Fishing => (rng.next_int_bound(4) + 1) as u32,
        XpSource::Trading => {
            (3 + rng.next_int_bound(4)) as u32 // 3-6
        }
        XpSource::Bottle => {
            (3 + rng.next_int_bound(9)) as u32 // 3-11
        }
        XpSource::BlockBreak => 0, // Only specific blocks
        XpSource::Grindstone => 0, // Not in baseline
    }
}

/// Add XP to a player.
pub fn add_xp(player: &mut crate::SimPlayer, amount: u32) {
    player.xp_total += amount;

    // Check for level ups
    let new_level = level_from_xp(player.xp_total);
    if new_level > player.level {
        player.level = new_level;
        // TODO: Trigger level up effects (particles, sound)
    }
}

/// Remove XP from a player (for enchanting, anvil, etc.).
/// Returns true if player had enough XP.
pub fn remove_xp(player: &mut crate::SimPlayer, amount: u32) -> bool {
    if player.xp_total >= amount {
        player.xp_total -= amount;
        player.level = level_from_xp(player.xp_total);
        true
    } else {
        false
    }
}

/// XP orb entity in the world.
#[derive(Debug, Clone)]
pub struct XpOrbEntity {
    /// XP value of this orb.
    pub value: u32,
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

impl XpOrbEntity {
    pub fn new(value: u32, position: Vec3<f32>, entity_id: i32) -> Self {
        Self {
            value,
            position,
            velocity: Vec3::new(0.0, 0.0, 0.0),
            age: 0,
            pickup_delay: 10, // 0.5 seconds
            entity_id,
        }
    }

    /// Get the orb type based on value.
    pub fn orb_type(&self) -> XpOrbValue {
        match self.value {
            1 => XpOrbValue::Tiny,
            3 => XpOrbValue::Small,
            7 => XpOrbValue::Medium,
            17 => XpOrbValue::Large,
            37 => XpOrbValue::Huge,
            73 => XpOrbValue::Massive,
            149 => XpOrbValue::Enormous,
            _ => XpOrbValue::Tiny, // Fallback
        }
    }

    /// Get the color index for rendering.
    pub fn color_index(&self) -> u8 {
        self.orb_type().color_index()
    }

    /// Tick the XP orb entity.
    /// Returns true if the orb should despawn.
    pub fn tick(&mut self, player_positions: &[mclre_core::mth::Vec3<f32>]) -> bool {
        self.age += 1;
        if self.age >= 6000 {
            return true; // Mark for despawn after 5 minutes
        }

        if self.pickup_delay > 0 {
            self.pickup_delay -= 1;
        }

        // Apply gravity
        self.velocity.y -= crate::physics::GRAVITY;
        if self.velocity.y < -crate::physics::TERMINAL_VELOCITY {
            self.velocity.y = -crate::physics::TERMINAL_VELOCITY;
        }

        // Attraction to nearby players (within 3 blocks)
        if self.pickup_delay == 0 {
            let mut nearest_pos: Option<mclre_core::mth::Vec3<f32>> = None;
            let mut nearest_dist_sq = 9.0; // 3 blocks squared

            for pos in player_positions {
                let dist_sq = (self.position - *pos).length_squared();
                if dist_sq < nearest_dist_sq {
                    nearest_dist_sq = dist_sq;
                    nearest_pos = Some(*pos);
                }
            }

            if let Some(pos) = nearest_pos {
                // Attract towards player
                let to_player = pos - self.position;
                let dist = to_player.length();
                if dist > 0.1 {
                    let force = 0.3 * (1.0 - dist / 3.0).max(0.0);
                    self.velocity += to_player.normalize() * force;
                }
            }
        }

        // Apply velocity
        self.position += self.velocity;

        // Simple ground check
        if self.position.y <= 64.0 && self.velocity.y <= 0.0 {
            self.position.y = 64.0;
            self.velocity.y = 0.0;
            self.velocity.x *= 0.98;
            self.velocity.z *= 0.98;
        }

        // Merge with nearby orbs of same type
        // (This is handled at the system level)

        false
    }
}

/// XP orb system state for tracking orbs across ticks.
#[derive(Debug, Default)]
pub struct XpOrbSystem {
    /// All active XP orbs.
    pub orbs: HashMap<i32, XpOrbEntity>,
    /// Next entity ID to assign.
    next_entity_id: i32,
}

impl XpOrbSystem {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new orb.
    pub fn add_orb(&mut self, orb: XpOrbEntity) {
        self.orbs.insert(orb.entity_id, orb);
    }

    /// Spawn XP orbs at a position.
    pub fn spawn_orbs(&mut self, position: Vec3<f32>, xp: u32, rng: &mut GameRandom) {
        let orbs = split_xp_into_orbs(xp);

        for orb_type in orbs {
            let value = orb_type as u32;
            let mut pos = Vec3::new(
                position.x + (rng.next_float() - 0.5) * 0.5,
                position.y + rng.next_float() * 0.25,
                position.z + (rng.next_float() - 0.5) * 0.5,
            );
            let entity_id = self.next_entity_id;
            self.next_entity_id += 1;
            let orb = XpOrbEntity::new(value, pos, entity_id);
            self.add_orb(orb);
        }
    }

    /// Tick all orbs.
    pub fn tick(&mut self, player_positions: &[mclre_core::mth::Vec3<f32>]) -> Vec<i32> {
        let mut to_remove = Vec::new();
        let mut orbs_by_value: HashMap<u32, Vec<i32>> = HashMap::new();

        // Tick all orbs
        for (id, orb) in self.orbs.iter_mut() {
            if orb.tick(player_positions) {
                to_remove.push(*id);
            }

            // Track orbs by value for merging
            if orb.pickup_delay == 0 {
                orbs_by_value.entry(orb.value).or_default().push(*id);
            }
        }

        // Remove dead orbs
        for id in &to_remove {
            self.orbs.remove(id);
        }

        // Merge nearby orbs of same value (within 0.5 blocks)
        // Based on reference: ExperienceOrb::merge - orbs within 0.5 blocks merge (sum values, recalc size)
        let mut merged_orb_ids = Vec::new();
        for (value, orb_ids) in orbs_by_value {
            if orb_ids.len() < 2 {
                continue;
            }

            // Check all pairs for merging
            let mut merged = vec![false; orb_ids.len()];
            for i in 0..orb_ids.len() {
                if merged[i] {
                    continue;
                }
                let id_i = orb_ids[i];
                let Some(orb_i) = self.orbs.get(&id_i) else {
                    continue;
                };
                let mut merged_value = orb_i.value;
                let mut merged_position = orb_i.position;
                let mut merged_count = 1;

                for j in (i + 1)..orb_ids.len() {
                    if merged[j] {
                        continue;
                    }
                    let id_j = orb_ids[j];
                    let Some(orb_j) = self.orbs.get(&id_j) else {
                        continue;
                    };

                    // Check distance (0.5 blocks = 0.5^2 = 0.25 distance squared)
                    let dist_sq = (orb_i.position - orb_j.position).length_squared();
                    if dist_sq <= 0.25 {
                        merged[j] = true;
                        merged_value += orb_j.value;
                        // Weighted average position
                        let total = merged_count + 1;
                        merged_position = (merged_position * (merged_count as f32)
                            + orb_j.position)
                            / (total as f32);
                        merged_count = total;
                    }
                }

                if merged_count > 1 {
                    // Update the first orb with merged values
                    if let Some(orb_i) = self.orbs.get_mut(&id_i) {
                        orb_i.value = merged_value;
                        orb_i.position = merged_position;
                    }
                    // Mark others for removal
                    for (j, &id_j) in orb_ids.iter().enumerate() {
                        if merged[j] {
                            merged_orb_ids.push(id_j);
                        }
                    }
                }
            }
        }

        // Remove merged orbs
        for id in &merged_orb_ids {
            self.orbs.remove(id);
        }
        to_remove.extend(merged_orb_ids);

        to_remove
    }
}

/// Spawn XP orbs at a position using the orb system.
pub fn spawn_xp_orbs(
    orb_system: &mut XpOrbSystem,
    position: Vec3<f32>,
    xp: u32,
    rng: &mut GameRandom,
) {
    let orbs = split_xp_into_orbs(xp);

    for orb_type in orbs {
        let value = orb_type as u32;
        let mut pos = Vec3::new(
            position.x + (rng.next_float() - 0.5) * 0.5,
            position.y + rng.next_float() * 0.25,
            position.z + (rng.next_float() - 0.5) * 0.5,
        );
        let orb = XpOrbEntity::new(value, pos, 0); // ID will be assigned by system
        orb_system.add_orb(orb);
    }
}

/// Tick the XP system - processes all XP orbs in the world.
pub fn tick(world: &mut SimWorld) -> Result<(), SimError> {
    // First, collect player positions for attraction logic (immutable borrow of ecs_world)
    let player_positions: Vec<mclre_core::mth::Vec3<f32>> = {
        let mut query = world.ecs_world.query::<&SimPlayer>();
        query.iter(&world.ecs_world).map(|p| p.position).collect()
    };

    // Also collect player entity IDs and positions for collection phase
    let player_data: Vec<(i32, mclre_core::mth::Vec3<f32>)> = {
        let mut query = world.ecs_world.query::<&SimPlayer>();
        query
            .iter(&world.ecs_world)
            .map(|p| (p.entity_id, p.position))
            .collect()
    };

    // Tick all orbs - get orb_system and tick it with player positions
    {
        let mut orb_system = world.get_xp_orb_system();
        orb_system.tick(&player_positions);
    }

    // Handle orb collection: first collect all (player_entity_id, orb_id) pairs to collect
    let collections: Vec<(i32, i32, u32)> = {
        let mut orb_system = world.get_xp_orb_system();
        let mut collections = Vec::new();

        for (player_entity_id, player_pos) in &player_data {
            for (orb_id, orb) in &orb_system.orbs {
                if orb.pickup_delay == 0 && (orb.position - *player_pos).length_squared() < 4.0 {
                    collections.push((*player_entity_id, *orb_id, orb.value));
                }
            }
        }
        collections
    };

    // Apply collections: remove orbs and add XP to players
    for (player_entity_id, orb_id, xp_value) in collections {
        // Remove orb
        {
            let mut orb_system = world.get_xp_orb_system();
            orb_system.orbs.remove(&orb_id);
        }
        // Add XP to player
        if let Some(mut player) = world.get_player_by_entity_id(player_entity_id) {
            add_xp(&mut player, xp_value);
        }
    }

    Ok(())
}

/// Add XP from a specific source.
pub fn add_xp_from_source(player: &mut crate::SimPlayer, source: XpSource, rng: &mut GameRandom) {
    let amount = xp_reward(source, rng);
    add_xp(player, amount);
}

/// XP sources for specific block breaking (only certain blocks give XP).
pub fn xp_from_block_break(block_id: u32, rng: &mut GameRandom) -> u32 {
    // XP from specific blocks (baseline values)
    match block_id {
        56 => (rng.next_int_bound(3) + 3) as u32, // Diamond ore: 3-5
        57 => (rng.next_int_bound(3) + 3) as u32, // Emerald ore: 3-5
        58 => (rng.next_int_bound(3) + 3) as u32, // Lapis ore: 3-5
        59 => (rng.next_int_bound(3) + 3) as u32, // Redstone ore: 3-5
        60 => (rng.next_int_bound(3) + 3) as u32, // Coal ore: 0-2 (but we give 1-3)
        61 => (rng.next_int_bound(3) + 3) as u32, // Nether quartz: 3-5
        _ => 0,
    }
}

/// Enchantment table slot costs (levels and lapis).
/// Based on reference: EnchantmentTableMenu.cpp, EnchantmentHelper.cpp
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnchantmentCost {
    /// Minimum level cost for this slot.
    pub min_level_cost: u32,
    /// Maximum level cost for this slot.
    pub max_level_cost: u32,
    /// Lapis lazuli cost for this slot.
    pub lapis_cost: u32,
    /// Minimum player level required to use this slot.
    pub min_player_level: u32,
}

/// Get enchantment cost for a specific slot (0=top, 1=middle, 2=bottom).
/// Based on LCE baseline: top=1-8/1lapis/level1, middle=9-21/2lapis/level15, bottom=22-30/3lapis/level30
pub fn enchantment_slot_cost(slot: u8) -> EnchantmentCost {
    match slot {
        0 => EnchantmentCost {
            min_level_cost: 1,
            max_level_cost: 8,
            lapis_cost: 1,
            min_player_level: 1,
        },
        1 => EnchantmentCost {
            min_level_cost: 9,
            max_level_cost: 21,
            lapis_cost: 2,
            min_player_level: 15,
        },
        2 => EnchantmentCost {
            min_level_cost: 22,
            max_level_cost: 30,
            lapis_cost: 3,
            min_player_level: 30,
        },
        _ => EnchantmentCost {
            min_level_cost: 1,
            max_level_cost: 8,
            lapis_cost: 1,
            min_player_level: 1,
        },
    }
}

/// Calculate the random enchantment level cost for a slot.
/// Formula from reference (EnchantmentHelper.cpp):
/// base = Random.nextInt(8) + 1 + (level/2) + Random.nextInt(level/2)
/// modified by bookshelves (0-15): base + Random.nextInt(bookshelves+1)
/// clamped to slot range
pub fn calculate_enchantment_cost(
    player_level: u32,
    slot: u8,
    bookshelves: u8,
    rng: &mut GameRandom,
) -> u32 {
    let slot_cost = enchantment_slot_cost(slot);

    // Base cost calculation (matches Java EnchantmentHelper.getEnchantmentLevel)
    let base = rng.next_int_bound(8)
        + 1
        + (player_level / 2) as i32
        + rng.next_int_bound((player_level / 2) as i32).max(1);
    let modified = base + rng.next_int_bound(bookshelves as i32 + 1);

    // Clamp to slot range
    modified.clamp(
        slot_cost.min_level_cost as i32,
        slot_cost.max_level_cost as i32,
    ) as u32
}

/// Anvil operation costs (levels).
/// Based on reference: AnvilMenu.cpp, AnvilTile.cpp
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnvilOperation {
    /// Rename item (1 level + prior work penalty)
    Rename,
    /// Repair with material (material cost × count, each unit = 1 level)
    Repair { material_cost: u32, count: u32 },
    /// Combine enchantments (sum of enchantment costs + prior work penalty)
    Combine { enchantment_cost: u32 },
}

/// Calculate prior work penalty: 2^n - 1 levels (n = times worked on anvil)
/// Baseline max anvil cost: 39 levels
pub fn anvil_prior_work_penalty(times_worked: u32) -> u32 {
    if times_worked == 0 {
        0
    } else {
        2u32.saturating_pow(times_worked).saturating_sub(1)
    }
}

/// Calculate anvil operation cost in levels.
/// Returns None if cost exceeds 39 (baseline max).
pub fn anvil_operation_cost(operation: AnvilOperation, times_worked: u32) -> Option<u32> {
    let base_cost = match operation {
        AnvilOperation::Rename => 1,
        AnvilOperation::Repair {
            material_cost,
            count,
        } => material_cost * count,
        AnvilOperation::Combine { enchantment_cost } => enchantment_cost,
    };

    let penalty = anvil_prior_work_penalty(times_worked);
    let total = base_cost.saturating_add(penalty);

    if total > 39 {
        None // Too expensive
    } else {
        Some(total)
    }
}

/// Check if player can afford enchantment (levels + lapis).
pub fn can_enchant(player: &crate::SimPlayer, slot: u8, lapis_count: u32) -> bool {
    let cost = enchantment_slot_cost(slot);
    player.level >= cost.min_player_level
        && player.xp_total >= cost.min_level_cost
        && lapis_count >= cost.lapis_cost
}

/// Check if player can afford anvil operation.
pub fn can_anvil(player: &crate::SimPlayer, cost: u32) -> bool {
    player.xp_total >= cost
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GameRandom;

    #[test]
    fn test_xp_level_curve() {
        // Test level curve matches reference (Player.cpp getXpForLevel)
        // Formula: cost = 2*level + 5 for all levels >= 1
        // (Matches explicit table for levels 1-20 in docs/reference/xp.md)

        // Level 0
        assert_eq!(xp_for_level(0), 0);

        // Cost per level (from reference table)
        assert_eq!(xp_cost_for_level(1), 7); // 2*1 + 5 = 7
        assert_eq!(xp_cost_for_level(2), 9); // 2*2 + 5 = 9
        assert_eq!(xp_cost_for_level(3), 11); // 2*3 + 5 = 11
        assert_eq!(xp_cost_for_level(15), 35); // 2*15 + 5 = 35
        assert_eq!(xp_cost_for_level(16), 37); // 2*16 + 5 = 37
        assert_eq!(xp_cost_for_level(17), 39); // 2*17 + 5 = 39
        assert_eq!(xp_cost_for_level(20), 45); // 2*20 + 5 = 45

        // Cumulative XP (from reference table)
        assert_eq!(xp_for_level(1), 7);
        assert_eq!(xp_for_level(2), 16); // 7 + 9
        assert_eq!(xp_for_level(3), 27); // 16 + 11
        assert_eq!(xp_for_level(4), 40); // 27 + 13
        assert_eq!(xp_for_level(15), 315);
        assert_eq!(xp_for_level(16), 352); // 315 + 37
        assert_eq!(xp_for_level(17), 391); // 352 + 39
        assert_eq!(xp_for_level(20), 520);

        // Level from XP
        assert_eq!(level_from_xp(0), 0);
        assert_eq!(level_from_xp(7), 1); // Exactly at level 1
        assert_eq!(level_from_xp(16), 2); // Exactly at level 2
        assert_eq!(level_from_xp(27), 3); // Exactly at level 3
        assert_eq!(level_from_xp(315), 15);
        assert_eq!(level_from_xp(352), 16);
        assert_eq!(level_from_xp(520), 20);

        // Progress to next level
        assert_eq!(progress_to_next_level(0), 0.0);
        assert_eq!(progress_to_next_level(7), 0.0); // Exactly at level 1
        assert_eq!(progress_to_next_level(16), 0.0); // Exactly at level 2
        // Halfway through level 1 (7 to 16, midpoint = 11.5)
        let progress = progress_to_next_level(11);
        assert!((progress - 4.0 / 9.0).abs() < 0.01); // (11-7)/(16-7) = 4/9
    }

    #[test]
    fn test_xp_orb_split() {
        // Test split_xp_into_orbs greedy algorithm
        // Orb values: Enormous=149, Massive=73, Huge=37, Large=17, Medium=7, Small=3, Tiny=1

        // 1 XP = 1 Tiny
        assert_eq!(split_xp_into_orbs(1), vec![XpOrbValue::Tiny]);

        // 3 XP = 1 Small
        assert_eq!(split_xp_into_orbs(3), vec![XpOrbValue::Small]);

        // 7 XP = 1 Medium
        assert_eq!(split_xp_into_orbs(7), vec![XpOrbValue::Medium]);

        // 17 XP = 1 Large
        assert_eq!(split_xp_into_orbs(17), vec![XpOrbValue::Large]);

        // 37 XP = 1 Huge
        assert_eq!(split_xp_into_orbs(37), vec![XpOrbValue::Huge]);

        // 73 XP = 1 Massive
        assert_eq!(split_xp_into_orbs(73), vec![XpOrbValue::Massive]);

        // 149 XP = 1 Enormous
        assert_eq!(split_xp_into_orbs(149), vec![XpOrbValue::Enormous]);

        // 2 XP = 2 Tiny
        assert_eq!(
            split_xp_into_orbs(2),
            vec![XpOrbValue::Tiny, XpOrbValue::Tiny]
        );

        // 4 XP = 1 Small + 1 Tiny
        assert_eq!(
            split_xp_into_orbs(4),
            vec![XpOrbValue::Small, XpOrbValue::Tiny]
        );

        // 10 XP = 1 Medium + 1 Small
        assert_eq!(
            split_xp_into_orbs(10),
            vec![XpOrbValue::Medium, XpOrbValue::Small]
        );

        // 150 XP = 1 Enormous + 1 Tiny
        assert_eq!(
            split_xp_into_orbs(150),
            vec![XpOrbValue::Enormous, XpOrbValue::Tiny]
        );

        // 0 XP = empty
        assert_eq!(split_xp_into_orbs(0), vec![]);
    }

    #[test]
    fn test_xp_orb_types() {
        // Test orb type matching values
        let tiny_orb = XpOrbEntity::new(1, Vec3::new(0.0, 0.0, 0.0), 1);
        assert_eq!(tiny_orb.orb_type(), XpOrbValue::Tiny);

        let small_orb = XpOrbEntity::new(3, Vec3::new(0.0, 0.0, 0.0), 1);
        assert_eq!(small_orb.orb_type(), XpOrbValue::Small);

        let medium_orb = XpOrbEntity::new(7, Vec3::new(0.0, 0.0, 0.0), 1);
        assert_eq!(medium_orb.orb_type(), XpOrbValue::Medium);

        let large_orb = XpOrbEntity::new(17, Vec3::new(0.0, 0.0, 0.0), 1);
        assert_eq!(large_orb.orb_type(), XpOrbValue::Large);

        let huge_orb = XpOrbEntity::new(37, Vec3::new(0.0, 0.0, 0.0), 1);
        assert_eq!(huge_orb.orb_type(), XpOrbValue::Huge);

        let massive_orb = XpOrbEntity::new(73, Vec3::new(0.0, 0.0, 0.0), 1);
        assert_eq!(massive_orb.orb_type(), XpOrbValue::Massive);

        let enormous_orb = XpOrbEntity::new(149, Vec3::new(0.0, 0.0, 0.0), 1);
        assert_eq!(enormous_orb.orb_type(), XpOrbValue::Enormous);

        // Unknown value falls back to Tiny
        let unknown_orb = XpOrbEntity::new(5, Vec3::new(0.0, 0.0, 0.0), 1);
        assert_eq!(unknown_orb.orb_type(), XpOrbValue::Tiny);
    }

    #[test]
    fn test_xp_orb_system_spawn() {
        let mut system = XpOrbSystem::new();
        let mut rng = GameRandom::with_seed(12345);

        // Spawn 10 XP
        system.spawn_orbs(Vec3::new(0.0, 64.0, 0.0), 10, &mut rng);

        // Should create orbs that sum to 10
        let total: u32 = system.orbs.values().map(|o| o.value).sum();
        assert_eq!(total, 10);

        // Spawn 149 XP (1 Enormous)
        system.spawn_orbs(Vec3::new(10.0, 64.0, 0.0), 149, &mut rng);
        let total: u32 = system.orbs.values().map(|o| o.value).sum();
        assert_eq!(total, 159);
    }

    #[test]
    fn test_enchantment_slot_costs() {
        // Top slot (0): 1-8 levels, 1 lapis, min level 1
        let cost = enchantment_slot_cost(0);
        assert_eq!(cost.min_level_cost, 1);
        assert_eq!(cost.max_level_cost, 8);
        assert_eq!(cost.lapis_cost, 1);
        assert_eq!(cost.min_player_level, 1);

        // Middle slot (1): 9-21 levels, 2 lapis, min level 15
        let cost = enchantment_slot_cost(1);
        assert_eq!(cost.min_level_cost, 9);
        assert_eq!(cost.max_level_cost, 21);
        assert_eq!(cost.lapis_cost, 2);
        assert_eq!(cost.min_player_level, 15);

        // Bottom slot (2): 22-30 levels, 3 lapis, min level 30
        let cost = enchantment_slot_cost(2);
        assert_eq!(cost.min_level_cost, 22);
        assert_eq!(cost.max_level_cost, 30);
        assert_eq!(cost.lapis_cost, 3);
        assert_eq!(cost.min_player_level, 30);

        // Invalid slot defaults to top
        let cost = enchantment_slot_cost(5);
        assert_eq!(cost.min_level_cost, 1);
        assert_eq!(cost.max_level_cost, 8);
    }

    #[test]
    fn test_anvil_prior_work_penalty() {
        // Prior work penalty: 2^n - 1
        assert_eq!(anvil_prior_work_penalty(0), 0);
        assert_eq!(anvil_prior_work_penalty(1), 1); // 2^1 - 1 = 1
        assert_eq!(anvil_prior_work_penalty(2), 3); // 2^2 - 1 = 3
        assert_eq!(anvil_prior_work_penalty(3), 7); // 2^3 - 1 = 7
        assert_eq!(anvil_prior_work_penalty(4), 15); // 2^4 - 1 = 15
        assert_eq!(anvil_prior_work_penalty(5), 31); // 2^5 - 1 = 31
        assert_eq!(anvil_prior_work_penalty(6), 63); // 2^6 - 1 = 63 (saturates at u32 max)
    }

    #[test]
    fn test_anvil_operation_costs() {
        // Rename: 1 level + penalty
        assert_eq!(anvil_operation_cost(AnvilOperation::Rename, 0), Some(1));
        assert_eq!(anvil_operation_cost(AnvilOperation::Rename, 1), Some(2)); // 1+1
        assert_eq!(anvil_operation_cost(AnvilOperation::Rename, 2), Some(4)); // 1+3
        assert_eq!(anvil_operation_cost(AnvilOperation::Rename, 5), Some(32)); // 1+31
        assert_eq!(anvil_operation_cost(AnvilOperation::Rename, 6), None); // 1+63 > 39

        // Repair: material_cost * count + penalty
        assert_eq!(
            anvil_operation_cost(
                AnvilOperation::Repair {
                    material_cost: 2,
                    count: 3
                },
                0
            ),
            Some(6)
        );
        assert_eq!(
            anvil_operation_cost(
                AnvilOperation::Repair {
                    material_cost: 5,
                    count: 2
                },
                1
            ),
            Some(11)
        ); // 10+1

        // Combine: enchantment_cost + penalty
        assert_eq!(
            anvil_operation_cost(
                AnvilOperation::Combine {
                    enchantment_cost: 10
                },
                0
            ),
            Some(10)
        );
        assert_eq!(
            anvil_operation_cost(
                AnvilOperation::Combine {
                    enchantment_cost: 20
                },
                2
            ),
            Some(23)
        ); // 20+3
    }

    #[test]
    fn test_can_enchant() {
        // Mock player with level 10, xp_total 100
        let player = crate::SimPlayer {
            identity: crate::PlayerIdentity::new(uuid::Uuid::now_v7(), "Test".to_string()),
            entity_id: 1,
            position: Vec3::new(0.0, 64.0, 0.0),
            velocity: Vec3::new(0.0, 0.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            on_ground: true,
            gamemode: crate::GameMode::Survival,
            health: 20.0,
            food: 20.0,
            food_saturation: 5.0,
            xp_total: 100,
            level: 10,
            inventory: crate::gameplay::inventory::PlayerInventory::default(),
            block_break_state: None,
            underwater: false,
            has_aqua_affinity: false,
            exhaustion: None,
            on_fire: false,
            fire_ticks: 0,
            in_lava: false,
            air: 300,
            suffocating: false,
            wither_ticks: 0,
            poison_ticks: 0,
            regen_ticks: 0,
            fire_resistance_ticks: 0,
        };

        // Top slot (level 1): min_level_cost=1, min_player_level=1, lapis=1
        assert!(can_enchant(&player, 0, 1)); // Has 10 levels, 1 lapis
        assert!(!can_enchant(&player, 0, 0)); // No lapis

        // Middle slot (level 2): min_level_cost=9, min_player_level=15, lapis=2
        assert!(!can_enchant(&player, 1, 2)); // Level 10 < 15
        assert!(!can_enchant(&player, 1, 3)); // Level 10 < 15

        // Bottom slot (level 3): min_level_cost=22, min_player_level=30, lapis=3
        assert!(!can_enchant(&player, 2, 3)); // Level 10 < 30
    }

    #[test]
    fn test_can_anvil() {
        let player = crate::SimPlayer {
            identity: crate::PlayerIdentity::new(uuid::Uuid::now_v7(), "Test".to_string()),
            entity_id: 1,
            position: Vec3::new(0.0, 64.0, 0.0),
            velocity: Vec3::new(0.0, 0.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            on_ground: true,
            gamemode: crate::GameMode::Survival,
            health: 20.0,
            food: 20.0,
            food_saturation: 5.0,
            xp_total: 100,
            level: 10,
            inventory: crate::gameplay::inventory::PlayerInventory::default(),
            block_break_state: None,
            underwater: false,
            has_aqua_affinity: false,
            exhaustion: None,
            on_fire: false,
            fire_ticks: 0,
            in_lava: false,
            air: 300,
            suffocating: false,
            wither_ticks: 0,
            poison_ticks: 0,
            regen_ticks: 0,
            fire_resistance_ticks: 0,
        };

        assert!(can_anvil(&player, 50));
        assert!(can_anvil(&player, 100));
        assert!(!can_anvil(&player, 101));
    }

    #[test]
    fn test_xp_reward_sources() {
        let mut rng = GameRandom::with_seed(42);

        // Mob kill: 1-5 XP
        let xp = xp_reward(XpSource::MobKill, &mut rng);
        assert!(xp >= 1 && xp <= 5);

        // Mining: 2-5 XP (or 3-6? rng.next_int_bound(4) gives 0-3, +2 = 2-5)
        let xp = xp_reward(XpSource::Mining, &mut rng);
        assert!(xp >= 2 && xp <= 5);

        // Smelting: 1-2 XP
        let xp = xp_reward(XpSource::Smelting, &mut rng);
        assert!(xp >= 1 && xp <= 2);

        // Breeding: 1-3 XP
        let xp = xp_reward(XpSource::Breeding, &mut rng);
        assert!(xp >= 1 && xp <= 3);

        // Fishing: 1-4 XP
        let xp = xp_reward(XpSource::Fishing, &mut rng);
        assert!(xp >= 1 && xp <= 4);

        // Trading: 3-6 XP
        let xp = xp_reward(XpSource::Trading, &mut rng);
        assert!(xp >= 3 && xp <= 6);

        // Bottle: 3-11 XP
        let xp = xp_reward(XpSource::Bottle, &mut rng);
        assert!(xp >= 3 && xp <= 11);

        // Block break: 0 (specific blocks handled separately)
        let xp = xp_reward(XpSource::BlockBreak, &mut rng);
        assert_eq!(xp, 0);

        // Grindstone: 0 (not in baseline)
        let xp = xp_reward(XpSource::Grindstone, &mut rng);
        assert_eq!(xp, 0);
    }

    #[test]
    fn test_block_break_xp() {
        let mut rng = GameRandom::with_seed(999);

        // Diamond ore (56): 3-5 XP
        let xp = xp_from_block_break(56, &mut rng);
        assert!(xp >= 3 && xp <= 5);

        // Emerald ore (57): 3-5 XP
        let xp = xp_from_block_break(57, &mut rng);
        assert!(xp >= 3 && xp <= 5);

        // Lapis ore (58): 3-5 XP
        let xp = xp_from_block_break(58, &mut rng);
        assert!(xp >= 3 && xp <= 5);

        // Redstone ore (59): 3-5 XP
        let xp = xp_from_block_break(59, &mut rng);
        assert!(xp >= 3 && xp <= 5);

        // Coal ore (60): 3-5 XP (baseline gives 0-2 but we give 1-3)
        let xp = xp_from_block_break(60, &mut rng);
        assert!(xp >= 3 && xp <= 5);

        // Nether quartz (61): 3-5 XP
        let xp = xp_from_block_break(61, &mut rng);
        assert!(xp >= 3 && xp <= 5);

        // Unknown block: 0 XP
        let xp = xp_from_block_break(999, &mut rng);
        assert_eq!(xp, 0);
    }
}
