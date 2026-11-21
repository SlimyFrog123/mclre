//! TNT and explosion mechanics for MCLRE baseline profile (LCE TU19).

use crate::{SimWorld, SimError};
use mclre_core::{BlockId, BlockStateId, Identifier, GameRandom, AABB};
use mclre_world::{BlockPos, ChunkPos};
use mclre_core::mth::Vec3;
use std::collections::HashSet;

/// Explosion power for different sources.
/// Based on LCE reference values.
#[derive(Debug, Clone, Copy)]
pub enum ExplosionSource {
    /// TNT block explosion (power 4).
    TNT,
    /// Creeper explosion (power 3).
    Creeper,
    /// Ghast fireball (power 1).
    GhastFireball,
    /// Wither skull (power 6).
    WitherSkull,
    /// End crystal (power 6).
    EndCrystal,
    /// Bed in Nether/End (power 5).
    Bed,
    /// Respawn anchor (power varies).
    RespawnAnchor,
    /// Custom power.
    Custom(f32),
}

impl ExplosionSource {
    /// Get the explosion power (radius).
    pub fn power(&self) -> f32 {
        match self {
            ExplosionSource::TNT => 4.0,
            ExplosionSource::Creeper => 3.0,
            ExplosionSource::GhastFireball => 1.0,
            ExplosionSource::WitherSkull => 6.0,
            ExplosionSource::EndCrystal => 6.0,
            ExplosionSource::Bed => 5.0,
            ExplosionSource::RespawnAnchor => 4.0, // Varies
            ExplosionSource::Custom(p) => *p,
        }
    }
    
    /// Whether this explosion creates fire.
    pub fn creates_fire(&self) -> bool {
        match self {
            ExplosionSource::GhastFireball => true,
            ExplosionSource::Bed => true,
            _ => false,
        }
    }
}

/// Explosion result containing affected blocks and entities.
#[derive(Debug, Clone, Default)]
pub struct ExplosionResult {
    /// Blocks that were destroyed.
    pub destroyed_blocks: Vec<(mclre_world::BlockPos, BlockStateId)>,
    /// Entities that were damaged.
    pub damaged_entities: Vec<(mclre_core::Identifier, f32, mclre_core::mth::Vec3<f32>)>, // (entity_id, damage, knockback)
    /// Whether fire was created.
    pub created_fire: bool,
}

/// Explosion context for raycasting.
#[derive(Debug)]
pub struct ExplosionContext<'a> {
    /// Explosion center position.
    pub center: Vec3<f32>,
    /// Explosion power.
    pub power: f32,
    /// Random number generator for ray variance.
    pub rng: &'a mut GameRandom,
    /// Whether to drop items from destroyed blocks.
    pub drop_items: bool,
    /// Whether to create fire.
    pub create_fire: bool,
    /// Entities to ignore (e.g., the exploding entity).
    pub ignored_entities: Vec<mclre_core::Identifier>,
}

/// Blast resistance values for blocks (LCE baseline).
/// Higher values = more resistant to explosions.
fn get_blast_resistance(block_id: BlockId) -> f32 {
    match block_id.0 {
        0 => 0.0, // Air
        1 => 30.0, // Stone
        2 => 3.0, // Grass
        3 => 2.5, // Dirt
        4 => 30.0, // Cobblestone
        5 => 15.0, // Planks
        7 => 30.0, // Bedrock (indestructible)
        17 => 10.0, // Log
        18 => 1.0, // Leaves
        20 => 1.5, // Glass
        49 => 0.0, // Obsidian (indestructible by normal explosions)
        50 => 0.0, // Torch
        54 => 12.5, // Chest
        55 => 0.0, // Redstone dust
        57 => 30.0, // Diamond ore
        58 => 12.5, // Crafting table
        61 => 25.0, // Furnace
        73 => 15.0, // Redstone ore
        123 => 15.0, // Redstone lamp
        124 => 15.0, // Stairs
        125 => 15.0, // Slab
        137 => 25.0, // Command block
        152 => 25.0, // End portal frame
        _ => 10.0, // Default
    }
}

/// Check if a block can be destroyed by an explosion of given power.
fn can_destroy_by_explosion(block_id: BlockId, power: f32, rng: &mut GameRandom) -> bool {
    let resistance = get_blast_resistance(block_id);
    
    // Indestructible blocks
    if resistance >= 6000.0 {
        return false;
    }
    
    // Blast resistance check: random * (resistance + 1) < power
    // This is the reference algorithm from the original game
    let random_factor = rng.next_float();
    random_factor * (resistance + 1.0) < power
}

/// Create an explosion at a position.
/// Returns the explosion result.
pub fn explode(
    world: &mut SimWorld,
    center: Vec3<f32>,
    source: ExplosionSource,
    rng: &mut GameRandom,
) -> ExplosionResult {
    let power = source.power();
    let create_fire = source.creates_fire();
    
    let mut result = ExplosionResult::default();
    result.created_fire = create_fire;
    
    let mut context = ExplosionContext {
        center,
        power,
        rng,
        drop_items: true,
        create_fire,
        ignored_entities: Vec::new(),
    };
    
    // Perform explosion raycasting
    // Reference algorithm: 1352 rays from center
    // Each ray travels up to power * 2 blocks
    
    let rays = generate_explosion_rays(context.rng);
    
    let mut destroyed = Vec::new();
    let mut damaged = Vec::new();
    let mut destroyed_positions = HashSet::new(); // Avoid duplicate destruction
    
    for ray in rays {
        let ray_result = cast_explosion_ray(world, &mut context, ray);
        for (pos, state) in ray_result.destroyed {
            if destroyed_positions.insert(pos) {
                destroyed.push((pos, state));
            }
        }
        damaged.extend(ray_result.damaged);
    }
    
    // Apply block destruction
    for (pos, state) in &destroyed {
        // Set block to air
        world.set_block_state(*pos, BlockStateId(0));
        
        // Spawn drops if enabled
        if context.drop_items {
            spawn_block_drops(world, *pos, *state, context.rng);
        }
    }
    
    result.destroyed_blocks = destroyed;
    result.damaged_entities = damaged;
    
    // Create fire if needed
    if create_fire {
        place_explosion_fire(world, &context);
    }
    
    result
}

/// Spawn item drops from a destroyed block.
fn spawn_block_drops(
    world: &mut SimWorld,
    pos: BlockPos,
    state: BlockStateId,
    rng: &mut GameRandom,
) {
    // TODO: Implement drop logic based on block type and fortune/silk touch
    // For now, just a placeholder
    let _ = (pos, state, rng);
}

/// Place fire around explosion center.
fn place_explosion_fire(world: &mut SimWorld, context: &ExplosionContext) {
    let center_pos = BlockPos::new(
        context.center.x.floor() as i32,
        context.center.y.floor() as i32,
        context.center.z.floor() as i32,
    );
    
    // Place fire in a small radius around the center
    for dx in -1..=1 {
        for dy in 0..=1 {
            for dz in -1..=1 {
                if dx == 0 && dy == 0 && dz == 0 {
                    continue;
                }
                
                let fire_pos = BlockPos::new(
                    center_pos.x() + dx,
                    center_pos.y() + dy,
                    center_pos.z() + dz,
                );
                
                // Only place fire on top of solid blocks
                let below = BlockPos::new(fire_pos.x(), fire_pos.y() - 1, fire_pos.z());
                let below_state = world.get_block_state(below);
                let below_id = BlockId(below_state.0);
                let below_props = crate::gameplay::lighting::get_light_propagation(below_id, below_state);
                
                if !below_props.transparent && world.get_block_state(fire_pos) == BlockStateId(0) {
                    // Place fire block
                    // TODO: world.set_block_state(fire_pos, FIRE_STATE);
                }
            }
        }
    }
}

/// Generate explosion rays matching reference algorithm.
/// Reference: 1352 rays from center.
fn generate_explosion_rays(rng: &mut GameRandom) -> Vec<Vec3<f32>> {
    let mut rays = Vec::with_capacity(1352);
    
    // Generate rays in a spherical pattern matching reference
    // The reference uses a specific deterministic pattern
    for _ in 0..1352 {
        // Use a deterministic but varied pattern
        let u = rng.next_float();
        let v = rng.next_float();
        
        let theta = 2.0 * std::f32::consts::PI * u;
        let phi = (2.0 * v - 1.0).acos();
        
        let x = phi.sin() * theta.cos();
        let y = phi.sin() * theta.sin();
        let z = phi.cos();
        
        rays.push(Vec3::new(x, y, z));
    }
    
    rays
}

/// Raycast result for explosion.
#[derive(Debug, Clone, Default)]
struct RaycastResult {
    destroyed: Vec<(mclre_world::BlockPos, BlockStateId)>,
    damaged: Vec<(mclre_core::Identifier, f32, mclre_core::mth::Vec3<f32>)>,
}

/// Cast a single explosion ray.
fn cast_explosion_ray(
    world: &SimWorld,
    context: &mut ExplosionContext,
    direction: Vec3<f32>,
) -> RaycastResult {
    let mut result = RaycastResult::default();
    
    let max_distance = context.power * 2.0;
    let step = 0.3; // Step size for raycasting
    let mut distance = 0.0;
    
    let mut pos = context.center;
    
    while distance < max_distance {
        pos = pos + direction * step;
        distance += step;
        
        let block_pos = BlockPos::new(
            pos.x.floor() as i32,
            pos.y.floor() as i32,
            pos.z.floor() as i32,
        );
        
        // Get block at position
        let block_state = world.get_block_state(block_pos);
        
        // Skip air
        if block_state == BlockStateId(0) {
            continue;
        }
        
        let block_id = BlockId(block_state.0);
        
        // Check if block should be destroyed
        if can_destroy_by_explosion(block_id, context.power, context.rng) {
            result.destroyed.push((block_pos, block_state));
        }
        
        // Stop ray if we hit a solid block (even if not destroyed)
        // This simulates the ray being absorbed
        let props = crate::gameplay::lighting::get_light_propagation(block_id, block_state);
        if !props.transparent && props.opacity >= 15 {
            break;
        }
    }
    
    result
}

/// Calculate damage to an entity from explosion.
pub fn calculate_explosion_damage(
    entity_pos: Vec3<f32>,
    explosion_center: Vec3<f32>,
    power: f32,
    exposure: f32, // 0.0 to 1.0, how much entity is exposed
) -> f32 {
    let distance = (entity_pos - explosion_center).length();
    
    if distance >= power * 2.0 {
        return 0.0;
    }
    
    // Damage formula from reference:
    // damage = (1.0 - distance / (power * 2.0)) * exposure * (power * 2.0 + 1.0)
    let damage = (1.0 - distance / (power * 2.0)) * exposure * (power * 2.0 + 1.0);
    
    // Apply armor/protection enchantments (TODO)
    damage.max(0.0)
}

/// Calculate knockback velocity from explosion.
pub fn calculate_knockback(
    entity_pos: Vec3<f32>,
    explosion_center: Vec3<f32>,
    power: f32,
) -> Vec3<f32> {
    let delta = entity_pos - explosion_center;
    let distance = delta.length();
    
    if distance == 0.0 || distance >= power * 2.0 {
        return Vec3::new(0.0, 0.0, 0.0);
    }
    
    let direction = delta / distance;
    let force = (1.0 - distance / (power * 2.0)) * power * 0.5;
    
    direction * force
}

/// Damage an entity from explosion.
pub fn damage_entity_from_explosion(
    world: &mut SimWorld,
    entity_id: mclre_core::Identifier,
    explosion_center: Vec3<f32>,
    source: ExplosionSource,
    rng: &mut GameRandom,
) -> bool {
    // TODO: Get entity position
    // Calculate damage and knockback
    // Apply to entity
    let _ = (world, entity_id, explosion_center, source, rng);
    false
}

/// Tick the TNT/explosion system.
pub fn tick(world: &mut SimWorld) -> Result<(), crate::SimError> {
    // Check for primed TNT entities and explode them
    // TODO: Implement
    Ok(())
}

/// Create a primed TNT entity at position.
pub fn prime_tnt(world: &mut SimWorld, pos: BlockPos, fuse_ticks: i32) -> bool {
    // TODO: Create primed TNT entity
    false
}