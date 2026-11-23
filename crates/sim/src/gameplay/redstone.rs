//! Redstone mechanics for MCLRE baseline profile (LCE TU19).

use crate::{SimError, SimWorld};
use mclre_core::{BlockId, BlockStateId, Identifier};
use mclre_world::{BlockPos, ChunkPos};
use std::collections::{HashMap, HashSet, VecDeque};

/// Redstone signal strength (0-15).
pub type SignalStrength = u8;

/// Maximum signal strength.
pub const MAX_SIGNAL: SignalStrength = 15;

/// Redstone component types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RedstoneComponent {
    /// Redstone dust.
    Dust,
    /// Redstone torch (on/off).
    Torch,
    /// Redstone block (always on).
    Block,
    /// Repeater (with delay).
    Repeater,
    /// Comparator.
    Comparator,
    /// Lever.
    Lever,
    /// Button (stone/wood).
    Button,
    /// Pressure plate (stone/wood/weighted).
    PressurePlate,
    /// Daylight detector.
    DaylightDetector,
    /// Tripwire hook.
    TripwireHook,
    /// Trapped chest.
    TrappedChest,
    /// Observer.
    Observer,
    /// Target block.
    Target,
    /// Redstone lamp.
    RedstoneLamp,
    /// Piston (sticky/regular).
    Piston,
    /// Sticky piston.
    StickyPiston,
    /// Dispenser.
    Dispenser,
    /// Dropper.
    Dropper,
    /// Note block.
    NoteBlock,
    /// Command block.
    CommandBlock,
    /// Structure block.
    StructureBlock,
}

/// Redstone update order (matching reference).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UpdateOrder {
    /// High priority (torches, repeaters, comparators).
    High = 0,
    /// Normal priority (dust, blocks, lamps).
    Normal = 1,
    /// Low priority (observers, pistons, dispensers).
    Low = 2,
}

/// Redstone update event.
#[derive(Debug, Clone)]
pub struct RedstoneUpdate {
    pub pos: BlockPos,
    pub old_strength: SignalStrength,
    pub new_strength: SignalStrength,
    pub component: RedstoneComponent,
    pub order: UpdateOrder,
}

/// Redstone network for a chunk.
#[derive(Debug, Clone, Default)]
pub struct RedstoneNetwork {
    /// Pending updates for this chunk.
    pub pending_updates: VecDeque<RedstoneUpdate>,
    /// Current signal strengths at positions.
    pub signals: HashMap<BlockPos, SignalStrength>,
    /// Blocks that provide power.
    pub power_sources: HashMap<BlockPos, (RedstoneComponent, SignalStrength)>,
    /// Blocks that can be powered.
    pub powered_blocks: HashSet<BlockPos>,
}

/// Get the component type for a block state.
pub fn get_component(block_id: BlockId, _state: BlockStateId) -> Option<RedstoneComponent> {
    match block_id.0 {
        55 => Some(RedstoneComponent::Dust),           // Redstone dust
        76 => Some(RedstoneComponent::Torch),          // Redstone torch (on)
        75 => Some(RedstoneComponent::Torch),          // Redstone torch (off)
        152 => Some(RedstoneComponent::Block),         // Redstone block
        93 => Some(RedstoneComponent::Repeater),       // Repeater
        94 => Some(RedstoneComponent::Repeater),       // Repeater (delayed)
        149 => Some(RedstoneComponent::Comparator),    // Comparator
        69 => Some(RedstoneComponent::Lever),          // Lever
        77 => Some(RedstoneComponent::Button),         // Stone button
        143 => Some(RedstoneComponent::Button),        // Wooden button
        72 => Some(RedstoneComponent::PressurePlate),  // Stone pressure plate
        70 => Some(RedstoneComponent::PressurePlate),  // Wooden pressure plate
        157 => Some(RedstoneComponent::PressurePlate), // Light weighted pressure plate
        158 => Some(RedstoneComponent::PressurePlate), // Heavy weighted pressure plate
        151 => Some(RedstoneComponent::DaylightDetector), // Daylight detector
        132 => Some(RedstoneComponent::TripwireHook),  // Tripwire hook
        146 => Some(RedstoneComponent::TrappedChest),  // Trapped chest
        182 => Some(RedstoneComponent::Observer),      // Observer
        138 => Some(RedstoneComponent::Target),        // Target block
        123 => Some(RedstoneComponent::RedstoneLamp),  // Redstone lamp
        33 => Some(RedstoneComponent::Piston),         // Piston
        34 => Some(RedstoneComponent::StickyPiston),   // Sticky piston
        23 => Some(RedstoneComponent::Dispenser),      // Dispenser
        26 => Some(RedstoneComponent::Dropper),        // Dropper
        25 => Some(RedstoneComponent::NoteBlock),      // Note block
        137 => Some(RedstoneComponent::CommandBlock),  // Command block
        255 => Some(RedstoneComponent::StructureBlock), // Structure block
        _ => None,
    }
}

/// Get the base signal strength for a power source.
pub fn get_source_strength(component: RedstoneComponent, state: BlockStateId) -> SignalStrength {
    // For components with state-dependent output (lever, button, etc.),
    // the state would be checked. For now, return max for active sources.
    match component {
        RedstoneComponent::Block => MAX_SIGNAL,
        RedstoneComponent::Torch => MAX_SIGNAL, // Torch is ON unless powered
        RedstoneComponent::Lever => MAX_SIGNAL, // Lever: 15 when ON
        RedstoneComponent::Button => MAX_SIGNAL, // Button: 15 when pressed
        RedstoneComponent::PressurePlate => MAX_SIGNAL, // Pressure plate: 15 when activated
        RedstoneComponent::DaylightDetector => 15, // Varies by time of day (TODO)
        RedstoneComponent::TrappedChest => 15,  // Based on players looking
        RedstoneComponent::Observer => MAX_SIGNAL, // Emits pulse when detecting change
        RedstoneComponent::Target => 15,        // Based on projectile hit position
        RedstoneComponent::RedstoneLamp => 0,   // Lamp is output, not source
        RedstoneComponent::Piston => 0,
        RedstoneComponent::StickyPiston => 0,
        RedstoneComponent::Dispenser => 0,
        RedstoneComponent::Dropper => 0,
        RedstoneComponent::NoteBlock => 0,
        RedstoneComponent::CommandBlock => 0,
        RedstoneComponent::StructureBlock => 0,
        RedstoneComponent::Dust | RedstoneComponent::Repeater | RedstoneComponent::Comparator => 0,
        _ => 0,
    }
}

/// Get the signal output from a block state.
pub fn get_output_strength(block_id: BlockId, state: BlockStateId) -> SignalStrength {
    if let Some(component) = get_component(block_id, state) {
        get_source_strength(component, state)
    } else {
        0
    }
}

/// Check if a block can be powered (receives signal).
pub fn can_be_powered(block_id: BlockId, state: BlockStateId) -> bool {
    // Most solid blocks can be powered
    // Transparent blocks generally cannot
    let component = get_component(block_id, state);
    match component {
        Some(RedstoneComponent::Dust) => true,
        Some(RedstoneComponent::Repeater) => true,
        Some(RedstoneComponent::Comparator) => true,
        Some(RedstoneComponent::RedstoneLamp) => true,
        Some(RedstoneComponent::Piston) => true,
        Some(RedstoneComponent::StickyPiston) => true,
        Some(RedstoneComponent::Dispenser) => true,
        Some(RedstoneComponent::Dropper) => true,
        Some(RedstoneComponent::NoteBlock) => true,
        Some(RedstoneComponent::CommandBlock) => true,
        Some(RedstoneComponent::StructureBlock) => true,
        Some(RedstoneComponent::Torch) => true, // Torch on side can be powered
        _ => is_solid_block(block_id),          // Solid opaque blocks can be powered
    }
}

/// Check if a block powers its neighbors (strong power).
pub fn powers_neighbors(block_id: BlockId, state: BlockStateId) -> bool {
    let component = get_component(block_id, state);
    matches!(
        component,
        Some(RedstoneComponent::Block) |
        Some(RedstoneComponent::Repeater) | // Repeater outputs strong power
        Some(RedstoneComponent::Comparator) |
        Some(RedstoneComponent::Lever) |
        Some(RedstoneComponent::Button) |
        Some(RedstoneComponent::PressurePlate) |
        Some(RedstoneComponent::DaylightDetector) |
        Some(RedstoneComponent::TrappedChest) |
        Some(RedstoneComponent::Observer) |
        Some(RedstoneComponent::Target) |
        Some(RedstoneComponent::RedstoneLamp) // When powered
    )
}

/// Check if a block is solid and opaque (can receive strong power).
fn is_solid_block(block_id: BlockId) -> bool {
    // Most full blocks are solid
    // Air, fluids, transparent blocks are not
    match block_id.0 {
        0 => false,                           // Air
        8 | 9 | 10 | 11 => false,             // Water, lava (stationary/flowing)
        31 | 32 | 37 | 38 | 39 | 40 => false, // Tall grass, flowers, etc.
        50 | 51 => false,                     // Torch, fire
        55 => false,                          // Redstone dust
        78 | 79 => false,                     // Snow layer, ice
        106 => false,                         // Vines
        _ => true,                            // Assume solid for now
    }
}

/// Get the delay for a repeater (1-4 ticks).
pub fn get_repeater_delay(state: BlockStateId) -> u8 {
    // Repeater delay is stored in block state properties (0-3 = 1-4 ticks)
    // For now, return 1
    1
}

/// Get comparator mode (compare vs subtract).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparatorMode {
    Compare,
    Subtract,
}

pub fn get_comparator_mode(state: BlockStateId) -> ComparatorMode {
    // TODO: Extract mode from state
    ComparatorMode::Compare
}

/// Propagate signal through redstone dust.
/// Returns the new signal strength at target position.
pub fn propagate_dust(
    world: &crate::SimWorld,
    from: BlockPos,
    to: BlockPos,
    strength: SignalStrength,
) -> SignalStrength {
    if strength == 0 {
        return 0;
    }

    // Signal decreases by 1 per block of dust
    // But can go up/down one block without loss
    let dx = (from.x() - to.x()).abs();
    let dy = (from.y() - to.y()).abs();
    let dz = (from.z() - to.z()).abs();

    let distance = dx + dy + dz;

    if distance == 0 {
        return strength;
    }

    // Vertical movement doesn't reduce signal (if supported by block)
    // Horizontal reduces by 1 per block
    let horizontal_dist = dx + dz;

    if strength > horizontal_dist as u8 {
        strength - horizontal_dist as u8
    } else {
        0
    }
}

/// Calculate new dust signal strength at a position based on neighbors.
pub fn calculate_dust_strength(world: &crate::SimWorld, pos: BlockPos) -> SignalStrength {
    let mut max_strength = 0;

    // Check all 6 neighbors
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
        let neighbor_strength = get_signal_at(world, neighbor_pos);

        // Propagate from neighbor
        let propagated = propagate_dust(world, neighbor_pos, pos, neighbor_strength);
        if propagated > max_strength {
            max_strength = propagated;
        }
    }

    // Also check power sources directly adjacent
    // (blocks that power dust)
    for (x, y, z) in neighbors {
        let neighbor_pos = BlockPos::new(x, y, z);
        let source_strength = get_direct_power(world, neighbor_pos, pos);
        if source_strength > max_strength {
            max_strength = source_strength;
        }
    }

    max_strength
}

/// Get signal strength at a position.
fn get_signal_at(world: &crate::SimWorld, pos: BlockPos) -> SignalStrength {
    let block_state = world.get_block_state(pos);
    let block_id = BlockId(block_state.0);

    if let Some(component) = get_component(block_id, block_state) {
        match component {
            RedstoneComponent::Dust => {
                // Dust strength is stored in the network
                // TODO: Look up from redstone network
                0
            }
            _ => get_output_strength(block_id, block_state),
        }
    } else {
        0
    }
}

/// Get direct power from a block to a neighbor.
fn get_direct_power(world: &crate::SimWorld, from: BlockPos, to: BlockPos) -> SignalStrength {
    let from_state = world.get_block_state(from);
    let from_id = BlockId(from_state.0);

    if !powers_neighbors(from_id, from_state) {
        return 0;
    }

    // Check if 'from' block can power 'to' block
    // Most power sources power all 6 sides
    // Torches don't power the block they're attached to
    // Repeaters only power in front
    // Comparators only power in front

    let component = get_component(from_id, from_state);
    match component {
        Some(RedstoneComponent::Torch) => {
            // Torch powers all sides except the block it's attached to
            // TODO: Check attachment direction
            get_output_strength(from_id, from_state)
        }
        Some(RedstoneComponent::Repeater) => {
            // Repeater only powers block in front
            // TODO: Check repeater facing direction
            if is_in_front_of_repeater(from, to, from_state) {
                get_output_strength(from_id, from_state)
            } else {
                0
            }
        }
        Some(RedstoneComponent::Comparator) => {
            // Comparator only powers block in front
            if is_in_front_of_comparator(from, to, from_state) {
                get_output_strength(from_id, from_state)
            } else {
                0
            }
        }
        _ => get_output_strength(from_id, from_state),
    }
}

/// Check if position 'to' is in front of repeater at 'from'.
fn is_in_front_of_repeater(from: BlockPos, to: BlockPos, _state: BlockStateId) -> bool {
    // TODO: Check repeater facing from state
    // For now, assume +X
    to.x() == from.x() + 1 && to.y() == from.y() && to.z() == from.z()
}

/// Check if position 'to' is in front of comparator at 'from'.
fn is_in_front_of_comparator(from: BlockPos, to: BlockPos, _state: BlockStateId) -> bool {
    // TODO: Check comparator facing from state
    to.x() == from.x() + 1 && to.y() == from.y() && to.z() == from.z()
}

/// Update redstone dust at a position.
pub fn update_dust(world: &mut crate::SimWorld, pos: BlockPos) -> Result<(), crate::SimError> {
    let new_strength = calculate_dust_strength(world, pos);
    let old_strength = get_signal_at(world, pos);

    if new_strength != old_strength {
        // Update signal in network
        // TODO: Update in redstone network

        // Schedule neighbor updates
        schedule_neighbor_updates(world, pos)?;
    }

    Ok(())
}

/// Schedule updates for neighbors of a position.
fn schedule_neighbor_updates(
    world: &mut crate::SimWorld,
    pos: BlockPos,
) -> Result<(), crate::SimError> {
    // TODO: Add to redstone network update queue
    Ok(())
}

/// Process a redstone update.
pub fn process_update(
    world: &mut crate::SimWorld,
    update: RedstoneUpdate,
) -> Result<(), crate::SimError> {
    match update.component {
        RedstoneComponent::Dust => {
            update_dust(world, update.pos)?;
        }
        RedstoneComponent::Repeater => {
            update_repeater(world, update.pos)?;
        }
        RedstoneComponent::Comparator => {
            update_comparator(world, update.pos)?;
        }
        RedstoneComponent::Torch => {
            update_torch(world, update.pos)?;
        }
        RedstoneComponent::RedstoneLamp => {
            update_lamp(world, update.pos)?;
        }
        RedstoneComponent::Piston | RedstoneComponent::StickyPiston => {
            update_piston(world, update.pos)?;
        }
        RedstoneComponent::Dispenser => {
            update_dispenser(world, update.pos)?;
        }
        RedstoneComponent::Dropper => {
            update_dropper(world, update.pos)?;
        }
        RedstoneComponent::NoteBlock => {
            update_note_block(world, update.pos)?;
        }
        _ => {
            // Other components handled by block update
        }
    }
    Ok(())
}

/// Update a repeater.
fn update_repeater(world: &mut crate::SimWorld, pos: BlockPos) -> Result<(), crate::SimError> {
    // Repeater: outputs input strength (1-15), delays by 1-4 ticks
    // 1. Get input from back
    // 2. Apply delay
    // 3. Output to front
    // 4. Lock mechanism if side-powered by another repeater

    // TODO: Implement
    Ok(())
}

/// Update a comparator.
fn update_comparator(world: &mut crate::SimWorld, pos: BlockPos) -> Result<(), crate::SimError> {
    // Comparator: compare/subtract modes
    // Compare: output = max(input, side1, side2)
    // Subtract: output = input - max(side1, side2)
    // Container mode: output based on container fullness

    // TODO: Implement
    Ok(())
}

/// Update a redstone torch.
fn update_torch(world: &mut crate::SimWorld, pos: BlockPos) -> Result<(), crate::SimError> {
    // Torch: inverts input (ON when not powered)
    // Powers all 6 sides except the block it's attached to
    // Burns out if toggled too fast (8 times in 100 ticks)

    // TODO: Implement
    Ok(())
}

/// Update a redstone lamp.
fn update_lamp(world: &mut crate::SimWorld, pos: BlockPos) -> Result<(), crate::SimError> {
    // Lamp: turns on when powered
    // TODO: Implement
    Ok(())
}

/// Update a piston.
fn update_piston(world: &mut crate::SimWorld, pos: BlockPos) -> Result<(), crate::SimError> {
    // Piston: extends when powered, retracts when unpowered
    // Sticky piston pulls block back
    // Max 12 blocks
    // TODO: Implement
    Ok(())
}

/// Update a dispenser.
fn update_dispenser(world: &mut crate::SimWorld, pos: BlockPos) -> Result<(), crate::SimError> {
    // Dispenser: dispenses/uses item when powered
    // TODO: Implement
    Ok(())
}

/// Update a dropper.
fn update_dropper(world: &mut crate::SimWorld, pos: BlockPos) -> Result<(), crate::SimError> {
    // Dropper: drops item when powered
    // TODO: Implement
    Ok(())
}

/// Update a note block.
fn update_note_block(world: &mut crate::SimWorld, pos: BlockPos) -> Result<(), crate::SimError> {
    // Note block: plays note when powered
    // Pitch changes on right-click
    // TODO: Implement
    Ok(())
}

/// Tick the redstone system.
pub fn tick(world: &mut SimWorld) -> Result<(), crate::SimError> {
    // Process redstone updates in priority order
    // 1. High priority: torches, repeaters, comparators
    // 2. Normal: dust, blocks, lamps
    // 3. Low: observers, pistons, dispensers

    // TODO: Process pending updates from redstone networks

    Ok(())
}

/// Notify the redstone system of a block change.
pub fn on_block_changed(
    world: &mut SimWorld,
    pos: BlockPos,
    old_state: BlockStateId,
    new_state: BlockStateId,
) -> Result<(), crate::SimError> {
    let old_id = BlockId(old_state.0);
    let new_id = BlockId(new_state.0);

    let old_component = get_component(old_id, old_state);
    let new_component = get_component(new_id, new_state);

    // If component type changed, schedule updates
    if old_component != new_component {
        // Schedule updates for this position and neighbors
        schedule_block_update(world, pos)?;
        schedule_neighbor_updates(world, pos)?;
    }

    // If power state changed, schedule updates
    let old_power = powers_neighbors(old_id, old_state);
    let new_power = powers_neighbors(new_id, new_state);

    if old_power != new_power {
        schedule_block_update(world, pos)?;
        schedule_neighbor_updates(world, pos)?;
    }

    Ok(())
}

/// Schedule a block update.
fn schedule_block_update(world: &mut SimWorld, pos: BlockPos) -> Result<(), crate::SimError> {
    // TODO: Add to update queue
    Ok(())
}
