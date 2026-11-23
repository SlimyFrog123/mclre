//! Tutorial world generator for LCE baseline profile.
//!
//! Generates a pre-built tutorial world matching the LCE reference tutorial,
//! with guided prompts and progression tracking.

use crate::{BiomeId, ChunkData, WorldGenerator};
use mclre_core::BlockStateId;
use mclre_world::{BlockPos, ChunkPos, WorldSizePreset};

/// Tutorial world generator - creates a fixed tutorial world layout.
pub struct TutorialGenerator {
    seed: i64,
    preset: WorldSizePreset,
}

impl TutorialGenerator {
    pub fn new(seed: i64, preset: WorldSizePreset) -> Self {
        Self { seed, preset }
    }

    /// Generate a tutorial chunk at the given position
    fn generate_tutorial_chunk(&self, chunk_pos: ChunkPos) -> ChunkData {
        let mut data = ChunkData::new();

        // Tutorial world is small (classic preset size)
        // Center around spawn (0, 64, 0)
        let chunk_x = chunk_pos.x;
        let chunk_z = chunk_pos.z;

        // Only generate content in a small area around spawn
        if chunk_x.abs() > 2 || chunk_z.abs() > 2 {
            return data; // Return empty (air) chunk
        }

        // Generate flat grass platform with tutorial structures
        for local_x in 0..16 {
            for local_z in 0..16 {
                let world_x = chunk_x * 16 + local_x;
                let world_z = chunk_z * 16 + local_z;

                // Base platform at Y=63 (sea level)
                for y in 0..=63 {
                    let idx = ((y * 16 + local_z) * 16 + local_x) as usize;
                    if y == 63 {
                        data.blocks[idx] = BlockStateId(2); // Grass block
                    } else if y >= 60 {
                        data.blocks[idx] = BlockStateId(3); // Dirt
                    } else {
                        data.blocks[idx] = BlockStateId(1); // Stone
                    }
                }

                // Set biome to plains
                let biome_idx = (local_z * 16 + local_x) as usize;
                data.biomes[biome_idx] = BiomeId::Plains;

                // Heightmap
                data.heightmap[biome_idx] = 64;
            }
        }

        // Add tutorial markers/signs at specific locations
        self.add_tutorial_markers(&mut data, chunk_pos);

        data
    }

    fn add_tutorial_markers(&self, data: &mut ChunkData, chunk_pos: ChunkPos) {
        // Tutorial stages and their positions (relative to spawn)
        // Stage 1: Movement (spawn area)
        // Stage 2: Breaking blocks
        // Stage 3: Crafting
        // Stage 4: Smelting
        // Stage 5: Combat
        // Stage 6: Mining
        // Stage 7: Nether portal

        // For now, just add some basic markers
        if chunk_pos.x == 0 && chunk_pos.z == 0 {
            // Spawn platform marker
            for local_x in 5..11 {
                for local_z in 5..11 {
                    let idx = ((64 * 16 + local_z) * 16 + local_x) as usize;
                    data.blocks[idx] = BlockStateId(41); // Gold block (marker)
                }
            }
        }
    }
}

impl WorldGenerator for TutorialGenerator {
    fn generate_chunk(&mut self, chunk_pos: ChunkPos) -> ChunkData {
        self.generate_tutorial_chunk(chunk_pos)
    }

    fn post_process_chunk(&mut self, chunk_pos: ChunkPos, _chunk_data: &mut ChunkData) {
        // Add tutorial structures post-generation
        if chunk_pos.x == 0 && chunk_pos.z == 0 {
            // Add chest with starter items at spawn
            // This would be implemented with tile entities
        }
    }

    fn get_biome(&self, _pos: BlockPos) -> BiomeId {
        BiomeId::Plains
    }
}

/// Tutorial progression steps matching LCE tutorial
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TutorialStep {
    /// Welcome screen / movement tutorial
    Welcome = 0,
    /// Move around
    Movement = 1,
    /// Look around
    Look = 2,
    /// Break a block
    BreakBlock = 3,
    /// Pick up item
    Pickup = 4,
    /// Open inventory
    Inventory = 5,
    /// Craft item
    Craft = 6,
    /// Place block
    PlaceBlock = 7,
    /// Use crafting table
    CraftingTable = 8,
    /// Smelt item
    Smelt = 9,
    /// Cook food
    CookFood = 10,
    /// Kill mob
    Combat = 11,
    /// Mine ore
    MineOre = 12,
    /// Build shelter
    Shelter = 13,
    /// Sleep in bed
    Sleep = 14,
    /// Nether portal
    NetherPortal = 15,
    /// Find fortress
    Fortress = 16,
    /// Defeat blaze
    Blaze = 17,
    /// End portal
    EndPortal = 18,
    /// Defeat dragon
    Dragon = 19,
    /// Tutorial complete
    Complete = 20,
}

impl TutorialStep {
    pub const ALL: &'static [TutorialStep] = &[
        TutorialStep::Welcome,
        TutorialStep::Movement,
        TutorialStep::Look,
        TutorialStep::BreakBlock,
        TutorialStep::Pickup,
        TutorialStep::Inventory,
        TutorialStep::Craft,
        TutorialStep::PlaceBlock,
        TutorialStep::CraftingTable,
        TutorialStep::Smelt,
        TutorialStep::CookFood,
        TutorialStep::Combat,
        TutorialStep::MineOre,
        TutorialStep::Shelter,
        TutorialStep::Sleep,
        TutorialStep::NetherPortal,
        TutorialStep::Fortress,
        TutorialStep::Blaze,
        TutorialStep::EndPortal,
        TutorialStep::Dragon,
        TutorialStep::Complete,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            TutorialStep::Welcome => "Welcome to Minecraft",
            TutorialStep::Movement => "Move Around",
            TutorialStep::Look => "Look Around",
            TutorialStep::BreakBlock => "Break a Block",
            TutorialStep::Pickup => "Pick Up Item",
            TutorialStep::Inventory => "Open Inventory",
            TutorialStep::Craft => "Craft an Item",
            TutorialStep::PlaceBlock => "Place a Block",
            TutorialStep::CraftingTable => "Use Crafting Table",
            TutorialStep::Smelt => "Smelt an Item",
            TutorialStep::CookFood => "Cook Food",
            TutorialStep::Combat => "Kill a Mob",
            TutorialStep::MineOre => "Mine Ore",
            TutorialStep::Shelter => "Build a Shelter",
            TutorialStep::Sleep => "Sleep in a Bed",
            TutorialStep::NetherPortal => "Build Nether Portal",
            TutorialStep::Fortress => "Find a Fortress",
            TutorialStep::Blaze => "Defeat a Blaze",
            TutorialStep::EndPortal => "Find End Portal",
            TutorialStep::Dragon => "Defeat the Dragon",
            TutorialStep::Complete => "Tutorial Complete",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            TutorialStep::Welcome => {
                "Welcome to the tutorial world! Follow the markers to learn the basics."
            }
            TutorialStep::Movement => "Use WASD to move around the spawn platform.",
            TutorialStep::Look => "Move your mouse to look around.",
            TutorialStep::BreakBlock => "Hold left-click to break the marked block.",
            TutorialStep::Pickup => "Walk over the dropped item to pick it up.",
            TutorialStep::Inventory => "Press E to open your inventory.",
            TutorialStep::Craft => "Use the 2x2 crafting grid to make planks from logs.",
            TutorialStep::PlaceBlock => "Right-click to place a block.",
            TutorialStep::CraftingTable => "Craft a crafting table and place it.",
            TutorialStep::Smelt => "Craft a furnace and smelt some cobblestone.",
            TutorialStep::CookFood => "Cook raw meat in the furnace.",
            TutorialStep::Combat => "Craft a sword and kill a zombie.",
            TutorialStep::MineOre => "Craft a pickaxe and mine iron ore.",
            TutorialStep::Shelter => "Build a small shelter before night.",
            TutorialStep::Sleep => "Craft a bed and sleep through the night.",
            TutorialStep::NetherPortal => "Build a nether portal with obsidian.",
            TutorialStep::Fortress => "Find a nether fortress.",
            TutorialStep::Blaze => "Defeat a blaze and collect its rod.",
            TutorialStep::EndPortal => "Find the stronghold and activate the end portal.",
            TutorialStep::Dragon => "Defeat the Ender Dragon!",
            TutorialStep::Complete => "Congratulations! You've completed the tutorial.",
        }
    }

    pub fn next(&self) -> Option<TutorialStep> {
        let idx = *self as usize;
        if idx + 1 < TutorialStep::ALL.len() {
            Some(TutorialStep::ALL[idx + 1])
        } else {
            None
        }
    }
}
