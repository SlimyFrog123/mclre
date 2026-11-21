//! Health, hunger, saturation, exhaustion, and regeneration system for MCLRE baseline profile (LCE TU19).

use crate::{SimWorld, SimError, SimPlayer};
use mclre_core::GameRandom;
use crate::gameplay::inventory::{ArmorSlot, PlayerInventory};

/// Health and hunger constants matching LCE baseline (TU19).
pub mod constants {
    /// Maximum health (10 hearts = 20 HP).
    pub const MAX_HEALTH: f32 = 20.0;
    
    /// Maximum hunger (10 drumsticks = 20 hunger points).
    pub const MAX_HUNGER: f32 = 20.0;
    
    /// Maximum saturation (hidden, max = hunger).
    pub const MAX_SATURATION: f32 = 20.0;
    
    /// Health regeneration rate per tick when saturation > 0 and not starving.
    /// Baseline: 1 HP per 4 seconds = 1 HP per 80 ticks = 0.0125 HP/tick.
    pub const REGEN_RATE: f32 = 1.0 / 80.0;
    
    /// Saturation cost per health point regenerated.
    pub const SATURATION_PER_HEALTH: f32 = 1.5;
    
    /// Health regeneration rate when hunger is full (20) - faster regen.
    /// 1 HP per 0.5 seconds = 1 HP per 10 ticks = 0.1 HP/tick.
    pub const REGEN_RATE_FULL: f32 = 1.0 / 10.0;
    
    /// Saturation cost per health point regenerated (full hunger).
    pub const SATURATION_PER_HEALTH_FULL: f32 = 2.0;
    
    /// Hunger depletion rate per tick (varies by activity).
    /// Base: 1 hunger per 4 minutes = 1/4800 per tick.
    pub const BASE_HUNGER_DEPLETION: f32 = 1.0 / 4800.0;
    
    /// Exhaustion multiplier when sprinting.
    pub const SPRINT_EXHAUSTION_MULTIPLIER: f32 = 6.0;
    
    /// Exhaustion added by jumping.
    pub const EXHAUSTION_JUMP: f32 = 0.05;
    
    /// Exhaustion added by attacking.
    pub const EXHAUSTION_ATTACK: f32 = 0.1;
    
    /// Exhaustion added by breaking a block.
    pub const EXHAUSTION_BREAK_BLOCK: f32 = 0.005;
    
    /// Exhaustion added by swimming.
    pub const EXHAUSTION_SWIM: f32 = 0.015;
    
    /// Exhaustion threshold for hunger depletion.
    pub const EXHAUSTION_THRESHOLD: f32 = 4.0;
    
    /// Minimum hunger for natural regeneration (6 drumsticks = 12).
    pub const REGEN_HUNGER_THRESHOLD: f32 = 18.0;
    
    /// Hunger level at which starvation begins (0).
    pub const STARVATION_HUNGER: f32 = 0.0;
    
    /// Starvation damage per tick (1 HP per 4 seconds = 1/80 per tick).
    pub const STARVATION_DAMAGE_RATE: f32 = 1.0 / 80.0;
    
    /// Saturation depletion from damage taken.
    pub const DAMAGE_SATURATION_COST: f32 = 0.3;
    
    /// Fire damage per tick (when on fire).
    pub const FIRE_DAMAGE_PER_TICK: f32 = 1.0 / 20.0; // 1 HP per second
    
    /// Lava damage per tick (when in lava).
    pub const LAVA_DAMAGE_PER_TICK: f32 = 4.0 / 20.0; // 4 HP per second
    
    /// Drowning damage per tick (when air runs out).
    pub const DROWNING_DAMAGE_PER_TICK: f32 = 2.0 / 20.0; // 2 HP per second
    
    /// Fall damage per block after 3 blocks.
    pub const FALL_DAMAGE_PER_BLOCK: f32 = 1.0;
    pub const FALL_DAMAGE_START_HEIGHT: f32 = 3.0;
    
    /// Suffocation damage per tick.
    pub const SUFFOCATION_DAMAGE_PER_TICK: f32 = 1.0 / 10.0; // 1 HP per 0.5 seconds
    
    /// Wither effect damage per tick (1 HP per 2 seconds).
    pub const WITHER_DAMAGE_PER_TICK: f32 = 1.0 / 40.0;
    
    /// Poison effect damage per tick (1 HP per 1.25 seconds, max 1 HP).
    pub const POISON_DAMAGE_PER_TICK: f32 = 1.0 / 25.0;
}

/// Player exhaustion tracker (hidden saturation-like mechanic).
#[derive(Debug, Clone, Default)]
pub struct Exhaustion {
    /// Current exhaustion level.
    pub level: f32,
}

impl Exhaustion {
    /// Add exhaustion and handle hunger depletion.
    pub fn add(&mut self, amount: f32, player: &mut SimPlayer) {
        self.level += amount;
        
        while self.level >= constants::EXHAUSTION_THRESHOLD {
            self.level -= constants::EXHAUSTION_THRESHOLD;
            deplete_hunger(player, 1.0);
        }
    }
}

/// Deplete hunger by the given amount.
/// Returns true if hunger reached 0.
fn deplete_hunger(player: &mut SimPlayer, amount: f32) -> bool {
    // First deplete saturation
    if player.food_saturation > 0.0 {
        let depleted = amount.min(player.food_saturation);
        player.food_saturation -= depleted;
        if player.food_saturation < 0.0 {
            player.food_saturation = 0.0;
        }
        return false;
    }
    
    // Then deplete hunger
    player.food -= amount;
    if player.food <= 0.0 {
        player.food = 0.0;
        return true;
    }
    false
}

/// Apply armor damage reduction.
/// Baseline formula: each armor point reduces 4% damage, max 80% (20 armor points).
/// Armor toughness reduces damage further: damage * (1 - toughness / (toughness + 8))
pub fn apply_armor_reduction(amount: f32, player: &SimPlayer) -> f32 {
    let armor_points = player.inventory.total_armor_points() as f32;
    let toughness = player.inventory.total_armor_toughness();
    
    if armor_points <= 0.0 {
        return amount;
    }
    
    // Base reduction: 4% per armor point, max 80%
    let base_reduction = (armor_points * 0.04).min(0.8);
    let reduced = amount * (1.0 - base_reduction);
    
    // Toughness reduction (diamond/netherite)
    if toughness > 0.0 {
        let toughness_factor = toughness / (toughness + 8.0);
        reduced * (1.0 - toughness_factor)
    } else {
        reduced
    }
}

/// Apply damage to a player with specific damage type.
/// Returns true if the player died.
pub fn damage_player(
    player: &mut SimPlayer,
    amount: f32,
    damage_type: DamageType,
    rng: &mut GameRandom,
) -> bool {
    // Apply armor reduction (except for damage types that bypass armor)
    let reduced = match damage_type {
        DamageType::Void | DamageType::Magic | DamageType::Starvation | DamageType::Wither => amount,
        _ => apply_armor_reduction(amount, player),
    };
    
    // Apply damage to health
    player.health -= reduced;
    
    // Deplete saturation from damage (except starvation/wither)
    if !matches!(damage_type, DamageType::Starvation | DamageType::Wither) {
        player.food_saturation -= constants::DAMAGE_SATURATION_COST * reduced;
        if player.food_saturation < 0.0 {
            player.food_saturation = 0.0;
        }
    }
    
    // Check for death
    if player.health <= 0.0 {
        player.health = 0.0;
        return true;
    }
    
    false
}

/// Heal the player.
pub fn heal_player(player: &mut SimPlayer, amount: f32) {
    player.health = (player.health + amount).min(constants::MAX_HEALTH);
}

/// Add hunger (food points) and saturation.
pub fn add_hunger(player: &mut SimPlayer, food: f32, saturation: f32) {
    player.food = (player.food + food).min(constants::MAX_HUNGER);
    player.food_saturation = (player.food_saturation + saturation).min(player.food);
}

/// Apply environmental damage based on player state.
pub fn apply_environmental_damage(
    player: &mut SimPlayer,
    rng: &mut GameRandom,
) -> bool {
    let mut died = false;
    
    // Fire damage
    if player.on_fire {
        died |= damage_player(player, constants::FIRE_DAMAGE_PER_TICK, DamageType::Fire, rng);
    }
    
    // Lava damage (in lava)
    if player.in_lava {
        died |= damage_player(player, constants::LAVA_DAMAGE_PER_TICK, DamageType::Lava, rng);
    }
    
    // Drowning (air <= 0)
    if player.air <= 0 {
        died |= damage_player(player, constants::DROWNING_DAMAGE_PER_TICK, DamageType::Drowning, rng);
    } else if player.underwater && player.air > 0 {
        player.air = (player.air - 1).max(0);
    } else if !player.underwater {
        // Restore air when not underwater (300 ticks = 15 seconds)
        player.air = (player.air + 4).min(300);
    }
    
    // Suffocation (inside solid block)
    if player.suffocating {
        died |= damage_player(player, constants::SUFFOCATION_DAMAGE_PER_TICK, DamageType::Suffocation, rng);
    }
    
    died
}

/// Apply status effect damage (wither, poison, etc.).
pub fn apply_status_effects(
    player: &mut SimPlayer,
    rng: &mut GameRandom,
) -> bool {
    let mut died = false;
    
    // Wither effect
    if player.wither_ticks > 0 {
        player.wither_ticks -= 1;
        died |= damage_player(player, constants::WITHER_DAMAGE_PER_TICK, DamageType::Wither, rng);
    }
    
    // Poison effect (doesn't kill, stops at 1 HP)
    if player.poison_ticks > 0 {
        player.poison_ticks -= 1;
        if player.health > 1.0 {
            died |= damage_player(player, constants::POISON_DAMAGE_PER_TICK, DamageType::Magic, rng);
            if player.health < 1.0 {
                player.health = 1.0;
            }
        }
    }
    
    // Regeneration effect
    if player.regen_ticks > 0 {
        player.regen_ticks -= 1;
        let heal_amount = 1.0 / 20.0; // 1 HP per second
        let sat_cost = heal_amount * 2.0;
        if player.food_saturation >= sat_cost {
            player.health = (player.health + heal_amount).min(constants::MAX_HEALTH);
            player.food_saturation -= sat_cost;
        }
    }
    
    // Fire resistance (immunity to fire/lava)
    if player.fire_resistance_ticks > 0 {
        player.fire_resistance_ticks -= 1;
        player.on_fire = false;
    }
    
    died
}

/// Check if player can naturally regenerate.
fn can_regenerate(player: &SimPlayer) -> bool {
    player.food >= constants::REGEN_HUNGER_THRESHOLD
        && player.health > 0.0
        && player.health < constants::MAX_HEALTH
        && player.gamemode != crate::GameMode::Creative
        && player.gamemode != crate::GameMode::Spectator
}

/// Calculate fall damage based on fall distance.
pub fn calculate_fall_damage(fall_distance: f32) -> f32 {
    if fall_distance <= constants::FALL_DAMAGE_START_HEIGHT {
        return 0.0;
    }
    (fall_distance - constants::FALL_DAMAGE_START_HEIGHT) * constants::FALL_DAMAGE_PER_BLOCK
}

/// Apply fall damage to player.
pub fn apply_fall_damage(player: &mut SimPlayer, fall_distance: f32, rng: &mut GameRandom) -> bool {
    let damage = calculate_fall_damage(fall_distance);
    if damage > 0.0 {
        damage_player(player, damage, DamageType::Fall, rng)
    } else {
        false
    }
}

/// Add exhaustion from an action.
pub fn add_exhaustion(player: &mut SimPlayer, amount: f32) {
    if player.exhaustion.is_none() {
        player.exhaustion = Some(Exhaustion::default());
    }
    if let Some(ref mut ex) = player.exhaustion {
        ex.level += amount;
        
        // Calculate how many times we need to deplete hunger
        let depletions = (ex.level / constants::EXHAUSTION_THRESHOLD) as u32;
        if depletions > 0 {
            ex.level -= depletions as f32 * constants::EXHAUSTION_THRESHOLD;
            for _ in 0..depletions {
                deplete_hunger(player, 1.0);
            }
        }
    }
}

/// Tick the health/hunger/exhaustion system for all players.
pub fn tick(world: &mut crate::SimWorld) -> Result<(), crate::SimError> {
    // Get RNG directly from the field to avoid borrow conflicts with ecs_world
    let mut rng = &mut world.rng;
    let mut query = world.ecs_world.query::<&mut SimPlayer>();
    
    for mut player in query.iter_mut(&mut world.ecs_world) {
        
        let mut died = false;
        died |= apply_environmental_damage(&mut player, &mut rng);
        died |= apply_status_effects(&mut player, &mut rng);
        
        // Handle natural regeneration
        if can_regenerate(&player) && player.food_saturation > 0.0 {
            // Faster regeneration when hunger is full (20)
            let (heal_amount, sat_cost) = if player.food >= 20.0 {
                (constants::REGEN_RATE_FULL, constants::SATURATION_PER_HEALTH_FULL)
            } else {
                (constants::REGEN_RATE, constants::SATURATION_PER_HEALTH)
            };
            
            if player.food_saturation >= sat_cost {
                player.health = (player.health + heal_amount).min(constants::MAX_HEALTH);
                player.food_saturation -= sat_cost;
            }
        }
        
        // Handle starvation
        if player.food <= constants::STARVATION_HUNGER && player.health > 0.0 {
            damage_player(&mut player, constants::STARVATION_DAMAGE_RATE, DamageType::Starvation, &mut rng);
        }
        
        // Passive hunger depletion (base rate)
        // Only in survival/adventure, not creative/spectator
        if player.gamemode == crate::GameMode::Survival || player.gamemode == crate::GameMode::Adventure {
            // Add base exhaustion per tick
            if player.exhaustion.is_none() {
                player.exhaustion = Some(Exhaustion::default());
            }
            if let Some(ref mut ex) = player.exhaustion {
                ex.level += constants::BASE_HUNGER_DEPLETION;
                
                // Calculate how many times we need to deplete hunger
                let depletions = (ex.level / constants::EXHAUSTION_THRESHOLD) as u32;
                if depletions > 0 {
                    ex.level -= depletions as f32 * constants::EXHAUSTION_THRESHOLD;
                    // Inline the deplete_hunger logic to avoid borrow issues
                    for _ in 0..depletions {
                        if player.food_saturation > 0.0 {
                            let depleted = 1.0_f32.min(player.food_saturation);
                            player.food_saturation -= depleted;
                            if player.food_saturation < 0.0 {
                                player.food_saturation = 0.0;
                            }
                        } else {
                            player.food -= 1.0;
                            if player.food <= 0.0 {
                                player.food = 0.0;
                            }
                        }
                    }
                }
            }
        }
        
        // Clamp values
        player.health = player.health.clamp(0.0, constants::MAX_HEALTH);
        player.food = player.food.clamp(0.0, constants::MAX_HUNGER);
        player.food_saturation = player.food_saturation.clamp(0.0, player.food);
        if let Some(ref mut ex) = player.exhaustion {
            ex.level = ex.level.max(0.0);
        }
    }
    
    Ok(())
}

/// Damage types for different damage calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageType {
    Generic,
    Fall,
    Fire,
    Lava,
    Drowning,
    Suffocation,
    Void,
    Magic,
    Explosion,
    Projectile,
    Thorns,
    Starvation,
    Wither,
}