//! Fire spread and burn-out mechanics for MCLRE baseline profile.

use crate::{SimError, SimWorld};
use mclre_core::{BlockId, BlockStateId, Identifier};
use mclre_world::{BlockPos, ChunkPos};

/// Fire state at a position.
#[derive(Debug, Clone, Copy, Default)]
pub struct FireState {
    /// Fire age (0-15, 15 = burn out).
    pub age: u8,
    /// Whether this fire was naturally generated (vs placed by player).
    pub natural: bool,
}

/// Flammability info for blocks.
#[derive(Debug, Clone, Copy)]
pub struct Flammability {
    /// How easily the block catches fire (0-300, higher = more flammable).
    pub catch_chance: u16,
    /// How fast the block burns away (0-300, higher = burns faster).
    pub burn_rate: u16,
}

/// Get flammability for a block.
/// Based on LCE reference values.
pub fn get_flammability(block_id: BlockId) -> Option<Flammability> {
    match block_id.0 {
        // Wood planks
        5 => Some(Flammability {
            catch_chance: 5,
            burn_rate: 20,
        }), // Planks
        17 => Some(Flammability {
            catch_chance: 5,
            burn_rate: 5,
        }), // Logs
        18 => Some(Flammability {
            catch_chance: 30,
            burn_rate: 60,
        }), // Leaves
        35 => Some(Flammability {
            catch_chance: 30,
            burn_rate: 60,
        }), // Wool
        31 => Some(Flammability {
            catch_chance: 60,
            burn_rate: 100,
        }), // Tall grass
        32 => Some(Flammability {
            catch_chance: 60,
            burn_rate: 100,
        }), // Dead bush
        106 => Some(Flammability {
            catch_chance: 15,
            burn_rate: 100,
        }), // Vines
        65 => Some(Flammability {
            catch_chance: 5,
            burn_rate: 20,
        }), // Ladder
        47 => Some(Flammability {
            catch_chance: 30,
            burn_rate: 20,
        }), // Bookshelf
        85 => Some(Flammability {
            catch_chance: 5,
            burn_rate: 20,
        }), // Fence
        107 => Some(Flammability {
            catch_chance: 5,
            burn_rate: 20,
        }), // Fence gate
        124 => Some(Flammability {
            catch_chance: 5,
            burn_rate: 20,
        }), // Wooden stairs
        125 => Some(Flammability {
            catch_chance: 5,
            burn_rate: 20,
        }), // Wooden slab
        64 => Some(Flammability {
            catch_chance: 5,
            burn_rate: 20,
        }), // Wooden door
        96 => Some(Flammability {
            catch_chance: 5,
            burn_rate: 20,
        }), // Trapdoor
        72 => Some(Flammability {
            catch_chance: 5,
            burn_rate: 20,
        }), // Pressure plate (wood)
        77 => Some(Flammability {
            catch_chance: 5,
            burn_rate: 20,
        }), // Button (wood)
        25 => Some(Flammability {
            catch_chance: 5,
            burn_rate: 20,
        }), // Note block
        84 => Some(Flammability {
            catch_chance: 5,
            burn_rate: 20,
        }), // Jukebox
        54 => Some(Flammability {
            catch_chance: 5,
            burn_rate: 20,
        }), // Chest
        146 => Some(Flammability {
            catch_chance: 5,
            burn_rate: 20,
        }), // Trapped chest
        58 => Some(Flammability {
            catch_chance: 5,
            burn_rate: 20,
        }), // Crafting table
        158 => Some(Flammability {
            catch_chance: 5,
            burn_rate: 20,
        }), // Daylight detector
        // TNT catches fire and explodes
        46 => Some(Flammability {
            catch_chance: 15,
            burn_rate: 100,
        }), // TNT
        _ => None,
    }
}

/// Check if a block can catch fire from a neighboring fire.
pub fn can_catch_fire(world: &crate::SimWorld, pos: BlockPos) -> bool {
    let block_state = world.get_block_state(pos);
    let block_id = get_block_id_from_state(block_state);

    // Check if block is flammable
    if get_flammability(block_id).is_none() {
        return false;
    }

    // Check if any neighbor has fire
    let neighbors = [
        (pos.x() + 1, pos.y(), pos.z()),
        (pos.x() - 1, pos.y(), pos.z()),
        (pos.x(), pos.y() + 1, pos.z()),
        (pos.x(), pos.y() - 1, pos.z()),
        (pos.x(), pos.y(), pos.z() + 1),
        (pos.x(), pos.y(), pos.z() - 1),
    ];

    for (x, y, z) in neighbors {
        let neighbor_pos = BlockPos::new(x, y, z);
        if has_fire(world, neighbor_pos) {
            return true;
        }
    }

    false
}

/// Check if fire can spread to a position.
/// Fire spreads:
/// - Up to 1 block down
/// - Up to 1 block sideways
/// - Up to 4 blocks up
/// Through flammable blocks
/// Not through non-flammable blocks
pub fn can_spread_to(world: &crate::SimWorld, from: BlockPos, to: BlockPos) -> bool {
    let dx = (from.x() - to.x()).abs();
    let dy = (from.y() - to.y()).abs();
    let dz = (from.z() - to.z()).abs();

    // Check spread limits
    // Can spread down 1
    if to.y() < from.y() && dy > 1 {
        return false;
    }
    // Can spread sideways 1
    if (dx > 1 || dz > 1) && to.y() == from.y() {
        return false;
    }
    // Can spread up 4
    if to.y() > from.y() && dy > 4 {
        return false;
    }

    // Check if target block is flammable
    let block_state = world.get_block_state(to);
    let block_id = get_block_id_from_state(block_state);
    get_flammability(block_id).is_some()
}

/// Try to ignite a block at position.
pub fn try_ignite(world: &mut crate::SimWorld, pos: BlockPos) -> bool {
    let block_state = world.get_block_state(pos);
    let block_id = get_block_id_from_state(block_state);

    // Check if block is flammable
    if get_flammability(block_id).is_none() {
        return false;
    }

    // Check if block is air (replace with fire)
    if block_state == BlockStateId(0) {
        // Set fire block
        // TODO: world.set_block_state(pos, FIRE_STATE);
        return true;
    }

    false
}

/// Age fire at a position.
pub fn age_fire(world: &mut crate::SimWorld, pos: BlockPos) -> bool {
    if let Some(mut fire_state) = get_fire_state(world, pos) {
        fire_state.age += 1;

        if fire_state.age >= 15 {
            // Fire burns out
            extinguish_fire(world, pos);
            return false;
        }

        // Update fire state
        set_fire_state(world, pos, fire_state.age);

        // Try to spread
        spread_fire(world, pos, fire_state);

        true
    } else {
        false
    }
}

/// Spread fire from a position.
fn spread_fire(world: &mut crate::SimWorld, from: BlockPos, fire_state: FireState) {
    let _ = fire_state; // Suppress unused warning for now

    // Fire spreads to neighboring flammable blocks
    let neighbors = [
        (from.x() + 1, from.y(), from.z()),
        (from.x() - 1, from.y(), from.z()),
        (from.x(), from.y() + 1, from.z()),
        (from.x(), from.y() - 1, from.z()),
        (from.x(), from.y(), from.z() + 1),
        (from.x(), from.y(), from.z() - 1),
    ];

    for (x, y, z) in neighbors {
        let to = BlockPos::new(x, y, z);
        if can_spread_to(world, from, to) {
            // Random chance to spread based on flammability
            let block_state = world.get_block_state(to);
            let block_id = get_block_id_from_state(block_state);
            if let Some(flammability) = get_flammability(block_id) {
                // Use catch_chance as probability (out of 300)
                // TODO: Use RNG
                if flammability.catch_chance > 150 {
                    // Simplified: always spread for highly flammable
                    try_ignite(world, to);
                }
            }
        }
    }
}

/// Extinguish fire at a position.
pub fn extinguish_fire(world: &mut crate::SimWorld, pos: BlockPos) -> bool {
    // Remove fire block (set to air)
    // TODO: world.set_block_state(pos, BlockStateId(0));
    true
}

/// Check if a block can be burned away.
pub fn can_burn_away(block_id: BlockId) -> bool {
    get_flammability(block_id).is_some()
}

/// Get burn time for a block (how long it takes to burn away).
pub fn get_burn_time(block_id: BlockId) -> u32 {
    if let Some(flammability) = get_flammability(block_id) {
        // Higher burn_rate = shorter burn time
        // Base: 600 ticks (30 seconds) for burn_rate = 20
        // burn_time = 600 * 20 / burn_rate
        (600 * 20 / flammability.burn_rate as u32).max(1)
    } else {
        600 // 30 seconds at 20 TPS (default)
    }
}

/// Get block ID from block state.
fn get_block_id_from_state(state: BlockStateId) -> BlockId {
    // TODO: Look up block ID from state registry
    // For now, use state.0 as block ID
    BlockId(state.0)
}

/// Check if a position has fire.
pub fn has_fire(world: &crate::SimWorld, pos: BlockPos) -> bool {
    get_fire_state(world, pos).is_some()
}

/// Get fire state at position.
pub fn get_fire_state(world: &crate::SimWorld, pos: BlockPos) -> Option<FireState> {
    // TODO: Look up fire state in chunk data
    // For now, check if block at pos is fire
    let block_state = world.get_block_state(pos);
    // Fire block ID would be checked here
    None
}

/// Set fire state at position.
pub fn set_fire_state(world: &mut crate::SimWorld, pos: BlockPos, age: u8) -> bool {
    // TODO: Update fire state in chunk
    false
}

/// Tick the fire system.
pub fn tick(world: &mut crate::SimWorld) -> Result<(), crate::SimError> {
    // 1. Age all fire blocks
    // 2. Extinguish fire that reached max age
    // 3. Spread fire to neighboring flammable blocks
    // 4. Burn away flammable blocks that have been on fire long enough

    // TODO: Iterate over all loaded chunks and process fire
    // For now, just return OK
    Ok(())
}

/// Ignition sources.
#[derive(Debug, Clone, Copy)]
pub enum IgnitionSource {
    FlintAndSteel,
    FireCharge,
    Lava,
    Lightning,
    GhastFireball,
    BedExplosion,
}

/// Ignite a block from an external source (flint and steel, fire charge, lava, lightning).
pub fn ignite_from_source(
    world: &mut crate::SimWorld,
    pos: BlockPos,
    source: IgnitionSource,
) -> bool {
    match source {
        IgnitionSource::FlintAndSteel => {
            // Flint and steel can ignite flammable blocks or netherrack
            try_ignite(world, pos)
        }
        IgnitionSource::FireCharge => {
            // Fire charge acts like flint and steel but at range
            try_ignite(world, pos)
        }
        IgnitionSource::Lava => {
            // Lava can ignite flammable blocks nearby
            try_ignite(world, pos)
        }
        IgnitionSource::Lightning => {
            // Lightning ignites blocks at strike point
            try_ignite(world, pos)
        }
        IgnitionSource::GhastFireball => {
            // Ghast fireball creates fire on impact
            try_ignite(world, pos)
        }
        IgnitionSource::BedExplosion => {
            // Bed explosion in Nether/End creates fire
            try_ignite(world, pos)
        }
    }
}

/// Check if a block is netherrack (burns forever in Nether).
pub fn is_netherrack(block_id: BlockId) -> bool {
    // Netherrack block ID
    block_id == BlockId(87) // Netherrack
}

/// Check if fire should be eternal (netherrack in Nether).
pub fn is_eternal_fire(world: &crate::SimWorld, pos: BlockPos) -> bool {
    let block_state = world.get_block_state(pos);
    let block_id = get_block_id_from_state(block_state);

    if is_netherrack(block_id) {
        // Check if in Nether
        is_nether(world, pos)
    } else {
        false
    }
}

/// Check if position is in Nether dimension.
fn is_nether(world: &SimWorld, pos: BlockPos) -> bool {
    // In LCE, Nether is at y < 0 or separate dimension
    pos.y() < 0
}
