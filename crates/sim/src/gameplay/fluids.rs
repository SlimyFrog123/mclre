//! Fluid simulation (water and lava) for MCLRE baseline profile.

use crate::{SimError, SimWorld};
use mclre_core::{BlockId, BlockStateId, Identifier};
use mclre_world::{BlockPos, ChunkPos};
use std::collections::VecDeque;

/// Fluid types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FluidType {
    Water,
    Lava,
}

/// Fluid properties matching LCE reference.
#[derive(Debug, Clone, Copy)]
pub struct FluidProperties {
    /// Maximum flow distance horizontally (7 for water, 3 for lava in overworld).
    pub flow_distance: u8,
    /// Maximum flow distance in Nether (lava flows 7 in Nether).
    pub nether_flow_distance: u8,
    /// Tick rate for fluid updates (every tick for water, every 2 for lava?).
    pub tick_rate: u32,
    /// Whether fluid creates infinite sources.
    pub infinite_source: bool,
    /// Viscosity (affects flow speed).
    pub viscosity: f32,
    /// Whether fluid can create obsidian/cobblestone/stone on contact.
    pub reacts_with: FluidReaction,
}

/// Fluid reaction types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FluidReaction {
    None,
    Obsidian,    // Water + Lava (lava source)
    Cobblestone, // Water + Lava (lava flowing)
    Stone,       // Water + Lava (water source into lava flowing)
}

/// Get fluid properties for a fluid type.
pub fn get_fluid_properties(fluid: FluidType, in_nether: bool) -> FluidProperties {
    match fluid {
        FluidType::Water => FluidProperties {
            flow_distance: 7,
            nether_flow_distance: 0, // Water doesn't flow in Nether (evaporates)
            tick_rate: 1,
            infinite_source: true,
            viscosity: 1.0,
            reacts_with: FluidReaction::None,
        },
        FluidType::Lava => FluidProperties {
            flow_distance: if in_nether { 7 } else { 3 },
            nether_flow_distance: 7,
            tick_rate: 2,           // Lava updates every 2 ticks
            infinite_source: false, // Lava sources are not infinite in baseline
            viscosity: 2.0,
            reacts_with: FluidReaction::Obsidian, // Will react with water
        },
    }
}

/// Fluid block state in a chunk section.
#[derive(Debug, Clone, Copy, Default)]
pub struct FluidState {
    /// Fluid level (0-15, 0 = empty, 8 = source for water, 15 = source for lava?).
    pub level: u8,
    /// Whether this is a source block.
    pub is_source: bool,
    /// Fluid type.
    pub fluid_type: Option<FluidType>,
    /// Tick counter for fluid updates.
    pub tick_counter: u32,
}

impl FluidState {
    /// Create a new fluid source block.
    pub fn source(fluid_type: FluidType) -> Self {
        Self {
            level: if fluid_type == FluidType::Water {
                8
            } else {
                15
            },
            is_source: true,
            fluid_type: Some(fluid_type),
            tick_counter: 0,
        }
    }

    /// Create a flowing fluid block.
    pub fn flowing(fluid_type: FluidType, level: u8) -> Self {
        Self {
            level,
            is_source: false,
            fluid_type: Some(fluid_type),
            tick_counter: 0,
        }
    }

    /// Check if this fluid state is empty.
    pub fn is_empty(&self) -> bool {
        self.fluid_type.is_none() || self.level == 0
    }
}

/// Check if a block state represents a fluid.
pub fn is_fluid(block_state: BlockStateId) -> bool {
    // Water: state IDs for water blocks
    // Lava: state IDs for lava blocks
    // TODO: Check block registry for fluid blocks
    // For now, use hardcoded checks
    false
}

/// Get the fluid type for a block state.
pub fn get_fluid_type(block_state: BlockStateId) -> Option<FluidType> {
    // TODO: Look up in registry
    None
}

/// Get fluid state at a position.
pub fn get_fluid_state(world: &SimWorld, pos: BlockPos) -> Option<FluidState> {
    // TODO: Look up in chunk section
    // For now, check if block at pos is a fluid
    let block_state = world.get_block_state(pos);
    if is_fluid(block_state) {
        get_fluid_type(block_state).map(|ft| FluidState {
            level: 8, // TODO: get actual level from block state
            is_source: true,
            fluid_type: Some(ft),
            tick_counter: 0,
        })
    } else {
        None
    }
}

/// Set fluid state at a position.
pub fn set_fluid_state(world: &mut SimWorld, pos: BlockPos, state: FluidState) -> bool {
    // TODO: Update chunk section with fluid state
    // This would involve setting the block state to the appropriate fluid block
    // with the correct level property
    false
}

/// Calculate fluid flow from a source using BFS.
/// Returns a list of positions that should receive fluid and their new levels.
pub fn calculate_flow(
    world: &SimWorld,
    source_pos: BlockPos,
    fluid_type: FluidType,
) -> Vec<(BlockPos, u8)> {
    let props = get_fluid_properties(fluid_type, is_nether(world, source_pos));
    let max_distance = props.flow_distance as i32;

    if max_distance == 0 {
        return Vec::new();
    }

    let mut results = Vec::new();
    let mut queue = VecDeque::new();
    let mut visited = std::collections::HashSet::new();

    // Start from source
    queue.push_back((source_pos, props.flow_distance));
    visited.insert(source_pos);

    while let Some((pos, remaining_distance)) = queue.pop_front() {
        if remaining_distance == 0 {
            continue;
        }

        // Check all 4 horizontal directions + down
        let neighbors = [
            (pos.x() + 1, pos.y(), pos.z()),
            (pos.x() - 1, pos.y(), pos.z()),
            (pos.x(), pos.y(), pos.z() + 1),
            (pos.x(), pos.y(), pos.z() - 1),
            (pos.x(), pos.y() - 1, pos.z()), // Down
        ];

        for (x, y, z) in neighbors {
            let neighbor_pos = BlockPos::new(x, y, z);

            if visited.contains(&neighbor_pos) {
                continue;
            }
            visited.insert(neighbor_pos);

            // Check if fluid can flow into this position
            if can_flow_into(world, neighbor_pos, fluid_type) {
                let new_level = remaining_distance as u8;
                results.push((neighbor_pos, new_level));

                // Continue flowing from this position
                queue.push_back((neighbor_pos, remaining_distance - 1));
            }
        }
    }

    results
}

/// Check if fluid can flow into a position.
fn can_flow_into(world: &SimWorld, pos: BlockPos, fluid_type: FluidType) -> bool {
    let block_state = world.get_block_state(pos);

    // Can flow into air
    if block_state == BlockStateId(0) {
        return true;
    }

    // Can flow into same fluid type (to level it)
    if let Some(existing) = get_fluid_type(block_state) {
        if existing == fluid_type {
            return true;
        }
    }

    // Can flow into replaceable blocks (tall grass, flowers, etc.)
    if is_replaceable(block_state) {
        return true;
    }

    false
}

/// Check if a block is replaceable by fluids.
fn is_replaceable(block_state: BlockStateId) -> bool {
    // TODO: Check block properties for replaceable flag
    // Tall grass, flowers, vines, etc. are replaceable
    false
}

/// Check if two fluids react at a position.
pub fn check_fluid_reaction(
    world: &SimWorld,
    pos: BlockPos,
    fluid_a: FluidType,
    fluid_b: FluidType,
) -> Option<FluidReaction> {
    // Water + Lava reactions
    if (fluid_a == FluidType::Water && fluid_b == FluidType::Lava)
        || (fluid_a == FluidType::Lava && fluid_b == FluidType::Water)
    {
        // Need to determine if source or flowing
        // For now, return Obsidian as default
        // In reference: water source + lava source = obsidian
        // water flowing + lava source = cobblestone
        // water source + lava flowing = stone
        return Some(FluidReaction::Obsidian);
    }
    None
}

/// Apply fluid reaction at a position (e.g., create obsidian).
pub fn apply_fluid_reaction(world: &mut SimWorld, pos: BlockPos, reaction: FluidReaction) -> bool {
    match reaction {
        FluidReaction::Obsidian => {
            // Set block to obsidian
            // TODO: world.set_block_state(pos, OBSIDIAN_STATE);
            true
        }
        FluidReaction::Cobblestone => {
            // Set block to cobblestone
            true
        }
        FluidReaction::Stone => {
            // Set block to stone
            true
        }
        FluidReaction::None => false,
    }
}

/// Check if position is in Nether dimension.
fn is_nether(world: &SimWorld, pos: BlockPos) -> bool {
    // In LCE, Nether is at y < 0 or separate dimension
    // For now, check if y is in Nether range
    pos.y() < 0
}

/// Tick the fluid system.
pub fn tick(world: &mut SimWorld) -> Result<(), crate::SimError> {
    // Process fluid updates for loaded chunks
    // 1. Find all fluid blocks that need updating
    // 2. Calculate flow
    // 3. Apply reactions
    // 4. Update block states

    // TODO: Implement proper fluid ticking with chunk iteration
    // For now, just return OK
    Ok(())
}

/// Check if a position can hold fluid (air or replaceable).
pub fn can_hold_fluid(world: &SimWorld, pos: BlockPos) -> bool {
    let block_state = world.get_block_state(pos);
    block_state == BlockStateId(0) || is_replaceable(block_state)
}

/// Get the effective flow distance for a fluid in a dimension.
pub fn effective_flow_distance(fluid: FluidType, in_nether: bool) -> u8 {
    let props = get_fluid_properties(fluid, in_nether);
    if in_nether {
        props.nether_flow_distance
    } else {
        props.flow_distance
    }
}

/// Update a single fluid block (called when block changes).
pub fn update_fluid_block(
    world: &mut SimWorld,
    pos: BlockPos,
    fluid_type: FluidType,
) -> Result<(), crate::SimError> {
    // When a fluid block is placed or removed, update surrounding fluids
    // 1. If source placed, start flow
    // 2. If source removed, stop flow from that source
    // 3. If fluid level changes, propagate

    Ok(())
}

/// Get all fluid source blocks in a chunk.
pub fn get_fluid_sources_in_chunk(
    world: &SimWorld,
    chunk_pos: ChunkPos,
) -> Vec<(BlockPos, FluidType)> {
    // TODO: Iterate chunk sections and find fluid sources
    Vec::new()
}
