//! Block breaking mechanics: progress, tool effectiveness, fortune/silk touch.

use crate::{SimWorld, SimError, SimPlayer};
use mclre_core::{BlockId, BlockStateId, Identifier, block_state::{BlockProperties, HarvestTool, HarvestTier, SoundGroup}, GameRandom};
use mclre_world::{BlockPos};
use tracing::trace;

/// Block breaking state tracked per player per block position.
#[derive(Debug, Clone)]
pub struct BlockBreakState {
    /// The block position being broken.
    pub pos: Option<BlockPos>,
    /// Current breaking progress (0.0 to 1.0).
    pub progress: f32,
    /// The player's entity ID.
    pub player_entity_id: i32,
    /// Tick when breaking started.
    pub start_tick: u64,
    /// The block state being broken (for drop determination).
    pub block_state: BlockStateId,
    /// The block ID being broken.
    pub block_id: BlockId,
}

impl Default for BlockBreakState {
    fn default() -> Self {
        Self {
            pos: None,
            progress: 0.0,
            player_entity_id: 0,
            start_tick: 0,
            block_state: BlockStateId(0),
            block_id: BlockId(0),
        }
    }
}

/// Block breaking speed multipliers for tools.
/// Based on LCE reference: tool speed vs block hardness.
#[derive(Debug, Clone, Copy)]
pub struct ToolBreakSpeed {
    /// Base mining speed for the tool material.
    pub base_speed: f32,
    /// Tool tier (0=wood/gold, 1=stone, 2=iron, 3=diamond, 4=netherite).
    pub tier: u8,
    /// Whether the tool can harvest this block type.
    pub can_harvest: bool,
}

/// Hardness and harvest info for blocks.
#[derive(Debug, Clone, Copy)]
pub struct BlockHardness {
    /// Base hardness (0.0 = instant, higher = slower).
    pub hardness: f32,
    /// Minimum tool tier required to harvest (0 = hand/any).
    pub harvest_tier: HarvestTier,
    /// Tool type required (pickaxe, axe, shovel, etc.).
    pub harvest_tool: HarvestTool,
}

/// Get the breaking speed for a player breaking a specific block.
/// Returns (speed_multiplier, can_harvest).
pub fn get_break_speed(
    player: &SimPlayer,
    block_state: BlockStateId,
    block_id: BlockId,
    sim_world: &SimWorld,
) -> (f32, bool) {
    // Get block properties
    let block_props = sim_world.block_properties(block_state);
    let hardness = block_props.hardness_f32();
    let harvest_tier = block_props.harvest_tier;
    let harvest_tool = block_props.harvest_tool;
    
    // Get held item
    let held_item = player.held_item();
    let (tool_speed, tool_tier, tool_type) = match held_item.item {
        Some(item) => {
            let item_str = item.to_string();
            // Get tool properties from item
            let tool_type = if item_str.contains("pickaxe") { HarvestTool::Pickaxe }
                else if item_str.contains("axe") { HarvestTool::Axe }
                else if item_str.contains("shovel") { HarvestTool::Shovel }
                else if item_str.contains("hoe") { HarvestTool::Hoe }
                else if item_str.contains("sword") { HarvestTool::Sword }
                else if item_str.contains("shears") { HarvestTool::Shears }
                else { HarvestTool::Hand };
            
            // Get tool tier from item name
            let tier = if item_str.contains("diamond") { HarvestTier::Diamond }
                else if item_str.contains("iron") { HarvestTier::Iron }
                else if item_str.contains("stone") || item_str.contains("cobblestone") { HarvestTier::Stone }
                else if item_str.contains("gold") { HarvestTier::Wood } // Gold is wood tier
                else if item_str.contains("wood") { HarvestTier::Wood }
                else { HarvestTier::Wood };
            let speed = tier.mining_speed();
            (speed, tier as u8, tool_type)
        }
        _ => (1.0, 0, HarvestTool::Hand), // Hand
    };
    
    // Check if tool can harvest this block
    let can_harvest = tool_tier >= harvest_tier as u8 && tool_type == harvest_tool;
    
    // If not in creative mode, can't break unbreakable blocks
    if hardness < 0.0 && player.gamemode != crate::GameMode::Creative {
        return (0.0, false);
    }
    
    // Speed calculation
    let speed = if can_harvest {
        tool_speed
    } else {
        1.0 // Hand speed
    };
    
    // Underwater penalty (5x slower)
    let speed = if player.is_underwater() && !player.has_aqua_affinity() {
        speed / 5.0
    } else {
        speed
    };
    
    (speed, can_harvest)
}

/// Calculate breaking progress for one tick.
/// Returns true if block should be broken this tick.
pub fn calculate_break_progress(
    speed: f32,
    hardness: f32,
    progress: f32,
) -> f32 {
    if hardness <= 0.0 {
        return 1.0; // Instant break
    }
    
    // Formula from reference: progress += speed / hardness / 30
    // 30 is the base time in ticks for hardness 1.0 with speed 1.0
    let delta = speed / hardness / 30.0;
    (progress + delta).min(1.0)
}

/// Check if a tool has Silk Touch enchantment.
pub fn has_silk_touch(player: &SimPlayer) -> bool {
    let held_item = player.held_item();
    if let Some(_item) = held_item.item {
        // Check enchantments
        // TODO: Implement enchantment checking
        false
    } else {
        false
    }
}

/// Check if a tool has Fortune enchantment and return level.
pub fn get_fortune_level(player: &SimPlayer) -> u8 {
    let held_item = player.held_item();
    if let Some(_item) = held_item.item {
        // Check enchantments
        // TODO: Implement enchantment checking
        0
    } else {
        0
    }
}

/// Apply block drops based on Fortune/Silk Touch.
pub fn get_block_drops(
    block_state: BlockStateId,
    block_id: BlockId,
    fortune_level: u8,
    silk_touch: bool,
    rng: &mut crate::GameRandom,
) -> Vec<(Identifier, u32)> {
    // Silk touch drops the block itself
    if silk_touch {
        // TODO: Return the block item itself
        return Vec::new();
    }
    
    // TODO: Look up drop table for block from registry
    // For now, return empty
    Vec::new()
}

/// Start breaking a block.
pub fn start_breaking(
    world: &mut SimWorld,
    player_entity_id: i32,
    pos: mclre_world::BlockPos,
    block_state: BlockStateId,
    block_id: BlockId,
) {
    let break_state = crate::gameplay::block_breaking::BlockBreakState {
        pos: Some(pos),
        progress: 0.0,
        player_entity_id,
        start_tick: world.tick_count,
        block_state,
        block_id,
    };
    
    // Store in player's break state
    if let Some(mut player) = world.get_player_by_entity_id(player_entity_id) {
        player.block_break_state = Some(break_state);
    }
}

/// Stop breaking a block (reset progress).
pub fn stop_breaking(world: &mut SimWorld, player_entity_id: i32) {
    if let Some(mut player) = world.get_player_by_entity_id(player_entity_id) {
        player.block_break_state = None;
    }
}

/// Data collected from a player's break state for processing.
struct BreakProgressData {
    player_entity_id: i32,
    pos: mclre_world::BlockPos,
    block_state: BlockStateId,
    block_id: BlockId,
    progress: f32,
    gamemode: crate::GameMode,
    held_item_str: Option<String>,
}

/// Completion data for when a block finishes breaking.
struct BreakCompletionData {
    player_entity_id: i32,
    pos: mclre_world::BlockPos,
    block_state: BlockStateId,
    block_id: BlockId,
    silk_touch: bool,
    fortune_level: u8,
    gamemode: crate::GameMode,
}

/// Tick the block breaking system.
pub fn tick(world: &mut SimWorld) -> Result<(), SimError> {
    let tick = world.tick_count;
    
    // First pass: collect all break progress data without holding mutable borrows across world calls
    let mut progress_data: Vec<BreakProgressData> = Vec::new();
    let mut completions: Vec<BreakCompletionData> = Vec::new();
    
    // Collect break state data from all players
    for mut player in world.players_mut() {
        if let Some(break_state) = &mut player.block_break_state {
            let pos = break_state.pos.expect("break state has position");
            let block_state = break_state.block_state;
            let block_id = break_state.block_id;
            let progress = break_state.progress;
            let gamemode = player.gamemode;
            let held_item = player.held_item();
            let held_item_str = held_item.item.map(|i| i.to_string());
            
            // Store data for processing, clear the break state to avoid borrow issues
            progress_data.push(BreakProgressData {
                player_entity_id: player.entity_id,
                pos,
                block_state,
                block_id,
                progress,
                gamemode,
                held_item_str,
            });
            
            // We'll update progress after computing it
            player.block_break_state = None;
        }
    }
    
    // Process collected break progress data
    for data in progress_data {
        // Verify block still exists at position
        let current_block = world.get_block_state(data.pos);
        if current_block != data.block_state {
            // Block changed, stop breaking - don't restore break state
            continue;
        }
        
        // Get break speed using the held item string
        let (speed, can_harvest) = get_break_speed_from_item_str(
            data.held_item_str.as_deref(),
            data.block_state,
            data.block_id,
            world,
            data.gamemode,
        );
        
        if !can_harvest && data.gamemode != crate::GameMode::Creative {
            // Can't harvest this block, stop breaking
            continue;
        }
        
        // Update progress
        let hardness = world.block_properties(data.block_state).hardness_f32();
        let new_progress = calculate_break_progress(speed, hardness, data.progress);
        
        // Check if breaking is complete
        if new_progress >= 1.0 {
            // Collect completion data - need to re-get player for silk touch/fortune
            let silk_touch = false; // TODO: check enchantments from held item
            let fortune_level = 0;  // TODO: check enchantments from held item
            
            completions.push(BreakCompletionData {
                player_entity_id: data.player_entity_id,
                pos: data.pos,
                block_state: data.block_state,
                block_id: data.block_id,
                silk_touch,
                fortune_level,
                gamemode: data.gamemode,
            });
        } else {
            // Restore break state with updated progress
            if let Some(mut player) = world.get_player_by_entity_id(data.player_entity_id) {
                player.block_break_state = Some(crate::gameplay::block_breaking::BlockBreakState {
                    pos: Some(data.pos),
                    progress: new_progress,
                    player_entity_id: data.player_entity_id,
                    start_tick: tick,
                    block_state: data.block_state,
                    block_id: data.block_id,
                });
            }
        }
    }
    
    // Apply break completions
    for completion in completions {
        // Get drops
        let mut rng = world.rng_mut();
        let drops = get_block_drops(
            completion.block_state,
            completion.block_id,
            completion.fortune_level,
            completion.silk_touch,
            &mut rng,
        );
        
        // Remove block
        world.set_block_state(completion.pos, mclre_core::BlockStateId(0)); // AIR = 0
        
        // Spawn XP orbs (if not creative and block gives XP)
        if completion.gamemode != crate::GameMode::Creative {
            // Get XP value first (using RNG)
            let xp = {
                let mut rng = world.rng_mut();
                crate::gameplay::xp::xp_from_block_break(completion.block_id.0, &mut rng)
            };
            if xp > 0 {
                let pos_f32 = mclre_core::mth::Vec3::new(
                    completion.pos.x() as f32 + 0.5,
                    completion.pos.y() as f32 + 0.5,
                    completion.pos.z() as f32 + 0.5,
                );
                // Generate orb data (values and positions) using RNG first
                let orb_data: Vec<(u32, mclre_core::mth::Vec3<f32>)> = {
                    let mut rng = world.rng_mut();
                    let orbs = crate::gameplay::xp::split_xp_into_orbs(xp);
                    orbs.into_iter().map(|orb_type| {
                        let value = orb_type as u32;
                        let mut pos = mclre_core::mth::Vec3::new(
                            pos_f32.x + (rng.next_float() - 0.5) * 0.5,
                            pos_f32.y + rng.next_float() * 0.25,
                            pos_f32.z + (rng.next_float() - 0.5) * 0.5,
                        );
                        (value, pos)
                    }).collect()
                };
                // Now add orbs to system (no RNG needed)
                let mut xp_orb_system = world.get_xp_orb_system();
                for (value, pos) in orb_data {
                    let orb = crate::gameplay::xp::XpOrbEntity::new(value, pos, 0);
                    xp_orb_system.add_orb(orb);
                }
            }
        }
        
        // Spawn drops (if not creative)
        if completion.gamemode != crate::GameMode::Creative {
            for (item, count) in drops {
                // TODO: Spawn item entity at position
                let _ = (item, count);
            }
        }
    }
    Ok(())
}

/// Spawn XP orbs for a broken block.
fn spawn_block_xp(world: &mut SimWorld, pos: BlockPos, block_id: BlockId, rng: &mut GameRandom) {
    let xp = crate::gameplay::xp::xp_from_block_break(block_id.0, rng);
    if xp > 0 {
        let pos_f32 = mclre_core::mth::Vec3::new(pos.x() as f32 + 0.5, pos.y() as f32 + 0.5, pos.z() as f32 + 0.5);
        crate::gameplay::xp::spawn_xp_orbs(&mut world.xp_orb_system, pos_f32, xp, rng);
    }
}

/// Get break speed from item string (avoids borrowing player).
fn get_break_speed_from_item_str(
    held_item_str: Option<&str>,
    block_state: BlockStateId,
    block_id: BlockId,
    world: &SimWorld,
    gamemode: crate::GameMode,
) -> (f32, bool) {
    // Get block properties
    let block_props = world.block_properties(block_state);
    let hardness = block_props.hardness_f32();
    let harvest_tier = block_props.harvest_tier;
    let harvest_tool = block_props.harvest_tool;
    
    // If not in creative mode, can't break unbreakable blocks
    if hardness < 0.0 && gamemode != crate::GameMode::Creative {
        return (0.0, false);
    }
    
    // Get tool properties from item string
    let (tool_speed, tool_tier, tool_type) = match held_item_str {
        Some(item_str) => {
            let tool_type = if item_str.contains("pickaxe") { HarvestTool::Pickaxe }
                else if item_str.contains("axe") { HarvestTool::Axe }
                else if item_str.contains("shovel") { HarvestTool::Shovel }
                else if item_str.contains("hoe") { HarvestTool::Hoe }
                else if item_str.contains("sword") { HarvestTool::Sword }
                else if item_str.contains("shears") { HarvestTool::Shears }
                else { HarvestTool::Hand };
            
            let tier = if item_str.contains("diamond") { HarvestTier::Diamond }
                else if item_str.contains("iron") { HarvestTier::Iron }
                else if item_str.contains("stone") || item_str.contains("cobblestone") { HarvestTier::Stone }
                else if item_str.contains("gold") { HarvestTier::Wood } // Gold is wood tier
                else if item_str.contains("wood") { HarvestTier::Wood }
                else { HarvestTier::Wood };
            let speed = tier.mining_speed();
            (speed, tier as u8, tool_type)
        }
        _ => (1.0, 0, HarvestTool::Hand), // Hand
    };
    
    // Check if tool can harvest this block
    let can_harvest = tool_tier >= harvest_tier as u8 && tool_type == harvest_tool;
    
    // Speed calculation
    let speed = if can_harvest {
        tool_speed
    } else {
        1.0 // Hand speed
    };
    
    (speed, can_harvest)
}