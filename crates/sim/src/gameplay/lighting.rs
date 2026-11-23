//! Sky and block lighting propagation for MCLRE baseline profile (LCE TU19).

use crate::{SimError, SimWorld};
use mclre_core::{BlockId, BlockStateId};
use mclre_world::{BlockPos, ChunkColumn, ChunkPos};
use std::collections::VecDeque;

/// Light level (0-15).
pub type LightLevel = u8;

/// Maximum light level.
pub const MAX_LIGHT: LightLevel = 15;

/// Light type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightType {
    /// Sky light (sun/moon light).
    Sky,
    /// Block light (torches, glowstone, etc.).
    Block,
}

/// Light propagation flags.
#[derive(Debug, Clone, Copy)]
pub struct LightPropagation {
    /// Whether this block lets light through (transparent).
    pub transparent: bool,
    /// Light reduction when passing through (0 = full, 15 = opaque).
    pub opacity: u8,
    /// Whether this block emits light.
    pub emission: LightLevel,
    /// Whether this block decreases sky light (casts shadow).
    pub casts_shadow: bool,
}

/// Get light propagation properties for a block.
/// Based on LCE reference values.
pub fn get_light_propagation(block_id: BlockId, _state: BlockStateId) -> LightPropagation {
    // TODO: Look up from registry
    // Baseline values:
    match block_id.0 {
        0 => LightPropagation {
            // Air
            transparent: true,
            opacity: 0,
            emission: 0,
            casts_shadow: false,
        },
        1 => LightPropagation {
            // Stone
            transparent: false,
            opacity: 15,
            emission: 0,
            casts_shadow: true,
        },
        2 => LightPropagation {
            // Grass
            transparent: false,
            opacity: 15,
            emission: 0,
            casts_shadow: true,
        },
        3 => LightPropagation {
            // Dirt
            transparent: false,
            opacity: 15,
            emission: 0,
            casts_shadow: true,
        },
        4 => LightPropagation {
            // Cobblestone
            transparent: false,
            opacity: 15,
            emission: 0,
            casts_shadow: true,
        },
        5 => LightPropagation {
            // Planks
            transparent: false,
            opacity: 15,
            emission: 0,
            casts_shadow: true,
        },
        17 => LightPropagation {
            // Log
            transparent: false,
            opacity: 15,
            emission: 0,
            casts_shadow: true,
        },
        18 => LightPropagation {
            // Leaves
            transparent: true,
            opacity: 1,
            emission: 0,
            casts_shadow: true,
        },
        20 => LightPropagation {
            // Glass
            transparent: true,
            opacity: 0,
            emission: 0,
            casts_shadow: false,
        },
        8 => LightPropagation {
            // Water (stationary)
            transparent: true,
            opacity: 2,
            emission: 0,
            casts_shadow: true,
        },
        9 => LightPropagation {
            // Water (flowing)
            transparent: true,
            opacity: 2,
            emission: 0,
            casts_shadow: true,
        },
        10 => LightPropagation {
            // Lava (stationary)
            transparent: true,
            opacity: 2,
            emission: 15,
            casts_shadow: true,
        },
        11 => LightPropagation {
            // Lava (flowing)
            transparent: true,
            opacity: 2,
            emission: 15,
            casts_shadow: true,
        },
        50 => LightPropagation {
            // Torch
            transparent: true,
            opacity: 0,
            emission: 14,
            casts_shadow: false,
        },
        75 => LightPropagation {
            // Redstone torch (off)
            transparent: true,
            opacity: 0,
            emission: 7,
            casts_shadow: false,
        },
        76 => LightPropagation {
            // Redstone torch (on)
            transparent: true,
            opacity: 0,
            emission: 7,
            casts_shadow: false,
        },
        89 => LightPropagation {
            // Glowstone
            transparent: false,
            opacity: 15,
            emission: 15,
            casts_shadow: true,
        },
        115 => LightPropagation {
            // Netherrack
            transparent: false,
            opacity: 15,
            emission: 0,
            casts_shadow: true,
        },
        123 => LightPropagation {
            // Redstone lamp (off)
            transparent: false,
            opacity: 15,
            emission: 0,
            casts_shadow: true,
        },
        123 => LightPropagation {
            // Redstone lamp (on) - different state
            transparent: false,
            opacity: 15,
            emission: 15,
            casts_shadow: true,
        },
        91 => LightPropagation {
            // Jack o'lantern
            transparent: false,
            opacity: 15,
            emission: 15,
            casts_shadow: true,
        },
        138 => LightPropagation {
            // Beacon
            transparent: false,
            opacity: 15,
            emission: 15,
            casts_shadow: true,
        },
        155 => LightPropagation {
            // Sea lantern
            transparent: false,
            opacity: 15,
            emission: 15,
            casts_shadow: true,
        },
        169 => LightPropagation {
            // End rod
            transparent: true,
            opacity: 0,
            emission: 14,
            casts_shadow: false,
        },
        _ => LightPropagation {
            transparent: false,
            opacity: 15,
            emission: 0,
            casts_shadow: true,
        },
    }
}

/// Check if a block is transparent to light.
pub fn is_transparent(block_id: BlockId, state: BlockStateId) -> bool {
    get_light_propagation(block_id, state).transparent
}

/// Get block opacity (0-15).
pub fn get_opacity(block_id: BlockId, state: BlockStateId) -> u8 {
    get_light_propagation(block_id, state).opacity
}

/// Get block light emission.
pub fn get_emission(block_id: BlockId, state: BlockStateId) -> LightLevel {
    get_light_propagation(block_id, state).emission
}

/// Check if block casts shadow (blocks sky light).
pub fn casts_shadow(block_id: BlockId, state: BlockStateId) -> bool {
    get_light_propagation(block_id, state).casts_shadow
}

/// Sky light propagation (sun/moon light).
/// Sky light comes from above and spreads sideways.
pub fn propagate_sky_light(world: &mut SimWorld, chunk_pos: ChunkPos) -> Result<(), SimError> {
    // Sky light propagation algorithm:
    // 1. Start from top of world (y = max_height)
    // 2. Sky light = 15 at top (day) or lower (night)
    // 3. Propagate down: each block reduces sky light by opacity
    // 4. Spread sideways: sky light spreads to neighbors with -1 per block

    let chunk_opt = world.get_chunk_mut(chunk_pos);
    if chunk_opt.is_none() {
        return Ok(());
    }
    let mut chunk = chunk_opt.unwrap();

    // Get sky light level at top based on time of day
    let time_of_day = world.get_time_of_day();
    let top_sky_light = get_sky_light_at_time(time_of_day);

    // Process each column in the chunk
    for x in 0..16 {
        for z in 0..16 {
            let mut sky_light = top_sky_light;

            // Propagate down through each section from top to bottom
            let section_count = chunk.section_count() as usize;
            for section_idx in (0..section_count).rev() {
                if let Some(section) = chunk.section_mut(section_idx) {
                    let base_y = (section_idx as i32) * 16;

                    for local_y in (0..16).rev() {
                        let world_y = base_y + local_y as i32;

                        let block_state = section.get(mclre_world::LocalPos::from_axes(
                            x as u8,
                            local_y as u8,
                            z as u8,
                        ));
                        let block_id = BlockId(block_state.0);
                        let props = get_light_propagation(block_id, block_state);

                        // Reduce by opacity
                        if sky_light > props.opacity {
                            sky_light -= props.opacity;
                        } else {
                            sky_light = 0;
                        }

                        // If block casts shadow, sky light becomes 0 below
                        if props.casts_shadow && !props.transparent {
                            sky_light = 0;
                        }

                        // Set sky light in section
                        // section.set_sky_light(local_y, x, z, sky_light);
                    }
                }
            }
        }
    }

    // After vertical propagation, do horizontal spread
    // This is more complex and typically done with BFS

    // Put chunk back
    world.set_chunk(chunk_pos, chunk);

    Ok(())
}

/// Block light propagation (torches, glowstone, etc.).
/// Block light spreads in all 6 directions, decreasing by 1 per block.
pub fn propagate_block_light(world: &mut SimWorld, chunk_pos: ChunkPos) -> Result<(), SimError> {
    let chunk_opt = world.get_chunk_mut(chunk_pos);
    if chunk_opt.is_none() {
        return Ok(());
    }
    let mut chunk = chunk_opt.unwrap();

    // Find all light sources in chunk (emission > 0)
    let mut sources = Vec::new();

    let section_count = chunk.section_count() as usize;
    for section_idx in 0..section_count {
        if let Some(section) = chunk.section(section_idx) {
            let base_y = (section_idx as i32) * 16;

            for x in 0..16 {
                for y in 0..16 {
                    for z in 0..16 {
                        let block_state = section
                            .get(mclre_world::LocalPos::from_axes(x as u8, y as u8, z as u8));
                        let block_id = BlockId(block_state.0);
                        let emission = get_emission(block_id, block_state);

                        if emission > 0 {
                            let pos = BlockPos::new(
                                chunk_pos.x * 16 + x as i32,
                                base_y + y as i32,
                                chunk_pos.z * 16 + z as i32,
                            );
                            sources.push((pos, emission));
                        }
                    }
                }
            }
        }
    }

    // BFS from each source
    for (source_pos, emission) in sources {
        bfs_block_light(world, source_pos, emission)?;
    }

    Ok(())
}

/// BFS for block light propagation.
fn bfs_block_light(
    world: &mut SimWorld,
    start: BlockPos,
    initial_light: LightLevel,
) -> Result<(), SimError> {
    let mut queue = VecDeque::new();
    let mut visited = std::collections::HashSet::new();

    // Start with the source block at its emission level
    queue.push_back((start, initial_light));
    visited.insert(start);

    while let Some((pos, light)) = queue.pop_front() {
        if light <= 1 {
            continue;
        }

        let next_light = light - 1;

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

            if visited.contains(&neighbor_pos) {
                continue;
            }

            // Check if light can pass through
            let neighbor_state = world.get_block_state(neighbor_pos);
            let neighbor_id = BlockId(neighbor_state.0);
            let props = get_light_propagation(neighbor_id, neighbor_state);

            if props.transparent || props.opacity < 15 {
                visited.insert(neighbor_pos);

                // Calculate light at neighbor
                let neighbor_light = if next_light > props.opacity {
                    next_light - props.opacity
                } else {
                    0
                };

                if neighbor_light > 0 {
                    queue.push_back((neighbor_pos, neighbor_light));
                }
            }
        }
    }

    Ok(())
}

/// Update lighting for a single block change.
pub fn update_lighting_for_block_change(
    world: &mut SimWorld,
    pos: BlockPos,
    old_state: BlockStateId,
    new_state: BlockStateId,
) -> Result<(), SimError> {
    let old_id = BlockId(old_state.0);
    let new_id = BlockId(new_state.0);

    let old_prop = get_light_propagation(old_id, old_state);
    let new_prop = get_light_propagation(new_id, new_state);

    // If emission changed, update block light
    if old_prop.emission != new_prop.emission {
        update_block_light_at(world, pos)?;
    }

    // If opacity changed, update both sky and block light
    if old_prop.opacity != new_prop.opacity || old_prop.casts_shadow != new_prop.casts_shadow {
        update_sky_light_at(world, pos)?;
        update_block_light_at(world, pos)?;
    }

    Ok(())
}

/// Update sky light at a position and propagate.
fn update_sky_light_at(world: &mut SimWorld, pos: BlockPos) -> Result<(), SimError> {
    // Sky light update:
    // 1. Recalculate column from pos.y to top
    // 2. Propagate changes down
    // 3. Horizontal spread if needed

    // Get chunk
    let chunk_pos = ChunkPos::new(pos.x() >> 4, pos.z() >> 4);
    let chunk_opt = world.get_chunk_mut(chunk_pos);
    if chunk_opt.is_none() {
        return Ok(());
    }
    let mut chunk = chunk_opt.unwrap();

    // Recalculate sky light for this column
    let time_of_day = world.get_time_of_day();
    let top_sky_light = get_sky_light_at_time(time_of_day);

    // Propagate down from top to pos.y
    let mut sky_light = top_sky_light;
    let max_y = world.get_max_build_height();

    for y in (pos.y()..=max_y).rev() {
        let check_pos = BlockPos::new(pos.x(), y, pos.z());
        let block_state = world.get_block_state(check_pos);
        let block_id = BlockId(block_state.0);
        let props = get_light_propagation(block_id, block_state);

        if sky_light > props.opacity {
            sky_light -= props.opacity;
        } else {
            sky_light = 0;
        }

        if props.casts_shadow && !props.transparent {
            sky_light = 0;
        }

        // world.set_sky_light(check_pos, sky_light);
    }

    // Also need to update blocks below pos if opacity decreased
    // This is complex - for now just recalculate whole column

    Ok(())
}

/// Update block light at a position and propagate.
fn update_block_light_at(world: &mut SimWorld, pos: BlockPos) -> Result<(), SimError> {
    // Block light update:
    // 1. If emission increased, BFS from this position
    // 2. If emission decreased, recalculate affected area

    let block_state = world.get_block_state(pos);
    let block_id = BlockId(block_state.0);
    let emission = get_emission(block_id, block_state);

    if emission > 0 {
        bfs_block_light(world, pos, emission)?;
    } else {
        // Light source removed - need to recalculate
        // This is complex; for now, recalculate chunk
        let chunk_pos = ChunkPos::new(pos.x() >> 4, pos.z() >> 4);
        propagate_block_light(world, chunk_pos)?;
    }

    Ok(())
}

/// Get sky light level at position.
pub fn get_sky_light(world: &crate::SimWorld, pos: BlockPos) -> LightLevel {
    // TODO: Look up in chunk
    15
}

/// Get block light level at position.
pub fn get_block_light(world: &crate::SimWorld, pos: BlockPos) -> LightLevel {
    // TODO: Look up in chunk
    0
}

/// Get combined light level (max of sky and block).
pub fn get_light_level(world: &crate::SimWorld, pos: BlockPos) -> LightLevel {
    let sky = get_sky_light(world, pos);
    let block = get_block_light(world, pos);
    sky.max(block)
}

/// Set sky light at position.
pub fn set_sky_light(world: &mut crate::SimWorld, pos: BlockPos, level: LightLevel) -> bool {
    // TODO: Update in chunk
    true
}

/// Set block light at position.
pub fn set_block_light(world: &mut crate::SimWorld, pos: BlockPos, level: LightLevel) -> bool {
    // TODO: Update in chunk
    true
}

/// Recalculate lighting for an entire chunk.
pub fn recalculate_chunk_lighting(
    world: &mut SimWorld,
    chunk_pos: ChunkPos,
) -> Result<(), SimError> {
    propagate_sky_light(world, chunk_pos)?;
    propagate_block_light(world, chunk_pos)?;
    Ok(())
}

/// Tick the lighting system.
pub fn tick(world: &mut SimWorld) -> Result<(), SimError> {
    // Lighting updates are event-driven (on block changes)
    // This tick can be used for:
    // - Day/night cycle sky light updates
    // - Delayed lighting updates
    // - Chunk loading/unloading lighting

    // Update sky light for day/night cycle
    update_day_night_cycle(world)?;

    Ok(())
}

/// Update sky light for day/night cycle.
fn update_day_night_cycle(world: &mut SimWorld) -> Result<(), SimError> {
    // Sky light level changes based on time of day
    // Day: 15, Night: 4 (with moon), transitions at dawn/dusk

    // TODO: Get time of day from world
    // Update top-level sky light values
    // Propagate changes down

    Ok(())
}

/// Get the current sky light level at world top based on time.
pub fn get_sky_light_at_time(time_of_day: u32) -> LightLevel {
    // 20 minute day cycle = 24000 ticks
    // Day: 0-12000 (sunrise at 0, noon at 6000, sunset at 12000)
    // Night: 12000-24000 (midnight at 18000)

    let time = time_of_day % 24000;

    if time < 2400 {
        // Dawn: 4 -> 15
        4 + ((time as f32 / 2400.0) * 11.0) as u8
    } else if time < 12000 {
        // Day: 15
        15
    } else if time < 13800 {
        // Dusk: 15 -> 4
        15 - (((time - 12000) as f32 / 1800.0) * 11.0) as u8
    } else if time < 22200 {
        // Night: 4 (moon light)
        4
    } else {
        // Pre-dawn: 4 -> 4 (stays 4 until dawn)
        4
    }
}
