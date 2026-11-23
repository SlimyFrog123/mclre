//! Guest-side SDK crate that WASM mods depend on (no_std-friendly where possible).
//!
//! This crate provides the stable API surface for mod authors.
//! It re-exports core types and defines the host function interface
//! that the host (mclre-modding) implements.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub use mclre_core::{BlockId, BlockStateId, GameRandom, Identifier, ItemId, Vec3};
use serde::{Deserialize, Serialize};
use serde_json;
use thiserror::Error;

/// Achievements and statistics tracking
pub mod achievements;

/// Mod manifest metadata (subset exposed to guests)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModMetadata {
    /// Unique mod identifier (namespace:name)
    pub id: Identifier,
    /// Human-readable name
    pub name: String,
    /// Version string (semver)
    pub version: String,
    /// Mod description
    pub description: String,
    /// Author(s)
    pub authors: Vec<String>,
    /// License identifier (SPDX)
    pub license: String,
    /// Required MCLRE version range
    pub mclre_version: String,
    /// Dependencies on other mods
    pub dependencies: Vec<ModDependency>,
}

/// Dependency on another mod
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModDependency {
    /// Dependency mod ID
    pub id: Identifier,
    /// Version requirement (semver range)
    pub version: String,
    /// Whether this is optional
    pub optional: bool,
}

/// Errors that can occur in the mod API
#[derive(Debug, Error)]
pub enum ModApiError {
    #[error("Host function not available: {0}")]
    HostFunctionUnavailable(String),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Resource not found: {0}")]
    NotFound(String),
    #[error("Internal host error: {0}")]
    Internal(String),
}

/// Result type for mod API operations
pub type ModResult<T> = Result<T, ModApiError>;

/// Permission levels for mod capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    /// Read-only world access
    WorldRead,
    /// Read-write world access
    WorldWrite,
    /// Player interaction (inventory, chat, etc.)
    PlayerInteract,
    /// Entity manipulation
    EntityManipulate,
    /// World generation hooks
    WorldGen,
    /// Network packet access
    Network,
    /// Configuration access
    Config,
}

/// Event types that mods can subscribe to
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    /// Called every server tick (20 TPS)
    Tick,
    /// Block broken by player
    BlockBreak,
    /// Block placed by player
    BlockPlace,
    /// Entity damaged
    EntityDamage,
    /// Player joined the server
    PlayerJoin,
    /// Player left the server
    PlayerLeave,
    /// Chunk generated
    ChunkGen,
    /// Item used by player
    ItemUse,
    /// Explosion occurred
    Explosion,
    /// Player chat message
    Chat,
    /// Player respawned
    PlayerRespawn,
    /// Item dropped
    ItemDrop,
    /// Entity spawned
    EntitySpawn,
}

/// Opaque handle to a server-side entity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityHandle(pub u64);

/// Opaque handle to a player
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerHandle(pub u64);

/// Opaque handle to a chunk
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkHandle(pub u64);

/// Block position (alias for Vec3<i32>)
pub type BlockPos = Vec3<i32>;

/// Chunk position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

/// Event context provided to event handlers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventContext {
    /// Type of event
    pub event_type: EventType,
    /// Tick number when event occurred
    pub tick: u64,
    /// Player who triggered the event (if applicable)
    pub player: Option<PlayerHandle>,
    /// Position where event occurred (if applicable)
    pub position: Option<BlockPos>,
}

/// Block break event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockBreakEvent {
    pub context: EventContext,
    pub block_state: BlockStateId,
    pub tool_item: Option<Identifier>,
    pub cancelled: bool,
}

/// Block place event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockPlaceEvent {
    pub context: EventContext,
    pub block_state: BlockStateId,
    pub replaced_state: BlockStateId,
    pub cancelled: bool,
}

/// Entity damage event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDamageEvent {
    pub context: EventContext,
    pub target: EntityHandle,
    pub source: Option<EntityHandle>,
    pub amount: f32,
    pub damage_type: Identifier,
    pub cancelled: bool,
}

/// Player join event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerJoinEvent {
    pub context: EventContext,
    pub player: PlayerHandle,
    pub username: String,
    pub uuid: String,
}

/// Chunk generation event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkGenEvent {
    pub context: EventContext,
    pub chunk_pos: ChunkPos,
    pub biome_data: Vec<u8>, // Biome IDs per block
}

/// Item use event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemUseEvent {
    pub context: EventContext,
    pub item: Identifier,
    pub hand: Hand,
    pub cancelled: bool,
}

/// Hand enum for item use
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Hand {
    MainHand,
    OffHand,
}

/// Explosion event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplosionEvent {
    pub context: EventContext,
    pub position: Vec3<f64>,
    pub radius: f32,
    pub cancelled: bool,
}

/// Chat event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatEvent {
    pub context: EventContext,
    pub message: String,
    pub cancelled: bool,
}

/// Player respawn event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerRespawnEvent {
    pub context: EventContext,
    pub player: PlayerHandle,
    pub position: BlockPos,
}

/// Item drop event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDropEvent {
    pub context: EventContext,
    pub item: Identifier,
    pub count: u8,
    pub position: Vec3<f64>,
}

/// Entity spawn event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySpawnEvent {
    pub context: EventContext,
    pub entity: EntityHandle,
    pub entity_type: Identifier,
    pub position: Vec3<f64>,
}

/// Callback type for event handlers
pub type EventCallback = fn(&dyn EventData) -> ModResult<()>;

/// Trait for event data types.
///
/// Implementors are `Any`, so handlers can downcast `&dyn EventData` to the
/// concrete event type via the `std::any::Any` supertrait.
pub trait EventData: std::any::Any + Send + Sync {
    fn event_type(&self) -> EventType;
}

/// Host functions that mods can call
pub mod host {
    use super::*;

    /// Register a block with the game
    pub fn register_block(
        id: Identifier,
        name: &str,
        properties: &[BlockProperty],
    ) -> ModResult<BlockStateId> {
        // Implemented by host
        Err(ModApiError::HostFunctionUnavailable(
            "register_block".into(),
        ))
    }

    /// Register an item with the game
    pub fn register_item(
        id: Identifier,
        name: &str,
        max_stack: u8,
        durability: Option<u16>,
    ) -> ModResult<Identifier> {
        // Implemented by host
        Err(ModApiError::HostFunctionUnavailable("register_item".into()))
    }

    /// Register a recipe
    pub fn register_recipe(
        id: Identifier,
        recipe_type: &str,
        ingredients: &[RecipeIngredient],
        result: RecipeResult,
    ) -> ModResult<()> {
        // Implemented by host
        Err(ModApiError::HostFunctionUnavailable(
            "register_recipe".into(),
        ))
    }

    /// Subscribe to an event
    pub fn subscribe_event(event_type: EventType, callback: EventCallback) -> ModResult<()> {
        // Implemented by host
        Err(ModApiError::HostFunctionUnavailable(
            "subscribe_event".into(),
        ))
    }

    /// Get block state at position
    pub fn get_block(pos: BlockPos) -> ModResult<BlockStateId> {
        // Implemented by host
        Err(ModApiError::HostFunctionUnavailable("get_block".into()))
    }

    /// Set block at position
    pub fn set_block(pos: BlockPos, state: BlockStateId) -> ModResult<()> {
        // Implemented by host
        Err(ModApiError::HostFunctionUnavailable("set_block".into()))
    }

    /// Get player by handle
    pub fn get_player(handle: PlayerHandle) -> ModResult<PlayerInfo> {
        // Implemented by host
        Err(ModApiError::HostFunctionUnavailable("get_player".into()))
    }

    /// Get entity by handle
    pub fn get_entity(handle: EntityHandle) -> ModResult<EntityInfo> {
        // Implemented by host
        Err(ModApiError::HostFunctionUnavailable("get_entity".into()))
    }

    /// Spawn an entity
    pub fn spawn_entity(
        entity_type: Identifier,
        position: Vec3<f64>,
        nbt: Option<&str>,
    ) -> ModResult<EntityHandle> {
        // Implemented by host
        Err(ModApiError::HostFunctionUnavailable("spawn_entity".into()))
    }

    /// Send chat message to player or all players
    pub fn send_chat(handle: Option<PlayerHandle>, message: &str) -> ModResult<()> {
        // Implemented by host
        Err(ModApiError::HostFunctionUnavailable("send_chat".into()))
    }

    /// Get game random for deterministic operations
    pub fn game_random() -> GameRandom {
        // Implemented by host - returns a seeded GameRandom
        GameRandom::with_seed(0)
    }

    /// Log a message to the server console
    pub fn log(level: LogLevel, message: &str) {
        // Implemented by host
    }

    /// Get config value for this mod
    pub fn get_config(key: &str) -> ModResult<Option<String>> {
        // Implemented by host
        Err(ModApiError::HostFunctionUnavailable("get_config".into()))
    }

    /// Set config value for this mod
    pub fn set_config(key: &str, value: &str) -> ModResult<()> {
        // Implemented by host
        Err(ModApiError::HostFunctionUnavailable("set_config".into()))
    }

    /// Schedule a function to run after N ticks
    pub fn schedule_task(ticks: u64, callback: fn()) -> ModResult<u64> {
        // Implemented by host
        Err(ModApiError::HostFunctionUnavailable("schedule_task".into()))
    }
}

/// Log levels for host logging
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Block property definition for guest registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockProperty {
    pub name: String,
    pub property_type: PropertyType,
    pub default_value: String,
    pub allowed_values: Vec<String>,
}

/// Property type for blocks
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropertyType {
    Bool,
    Int(i32, i32), // min, max
    Enum(Vec<String>),
}

/// Recipe ingredient for guest registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeIngredient {
    pub item: Identifier,
    pub count: u8,
    pub tag: Option<Identifier>, // Optional tag instead of specific item
}

/// Recipe result for guest registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeResult {
    pub item: Identifier,
    pub count: u8,
}

/// Player info returned by host
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub handle: PlayerHandle,
    pub username: String,
    pub uuid: String,
    pub position: Vec3<f64>,
    pub yaw: f32,
    pub pitch: f32,
    pub health: f32,
    pub max_health: f32,
    pub food_level: i32,
    pub saturation: f32,
    pub game_mode: GameMode,
}

/// Entity info returned by host
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityInfo {
    pub handle: EntityHandle,
    pub entity_type: Identifier,
    pub position: Vec3<f64>,
    pub yaw: f32,
    pub pitch: f32,
    pub health: f32,
    pub max_health: f32,
    pub is_alive: bool,
}

/// Game mode enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameMode {
    Survival = 0,
    Creative = 1,
    Adventure = 2,
    Spectator = 3,
}

impl EventData for BlockBreakEvent {
    fn event_type(&self) -> EventType {
        EventType::BlockBreak
    }
}

impl EventData for BlockPlaceEvent {
    fn event_type(&self) -> EventType {
        EventType::BlockPlace
    }
}

impl EventData for EntityDamageEvent {
    fn event_type(&self) -> EventType {
        EventType::EntityDamage
    }
}

impl EventData for PlayerJoinEvent {
    fn event_type(&self) -> EventType {
        EventType::PlayerJoin
    }
}

impl EventData for ChunkGenEvent {
    fn event_type(&self) -> EventType {
        EventType::ChunkGen
    }
}

impl EventData for ItemUseEvent {
    fn event_type(&self) -> EventType {
        EventType::ItemUse
    }
}

impl EventData for ExplosionEvent {
    fn event_type(&self) -> EventType {
        EventType::Explosion
    }
}

impl EventData for ChatEvent {
    fn event_type(&self) -> EventType {
        EventType::Chat
    }
}

impl EventData for PlayerRespawnEvent {
    fn event_type(&self) -> EventType {
        EventType::PlayerRespawn
    }
}

impl EventData for ItemDropEvent {
    fn event_type(&self) -> EventType {
        EventType::ItemDrop
    }
}

impl EventData for EntitySpawnEvent {
    fn event_type(&self) -> EventType {
        EventType::EntitySpawn
    }
}

/// Re-export core types for convenience
pub mod prelude {
    pub use crate::{
        BlockBreakEvent, BlockPlaceEvent, BlockPos, BlockProperty, BlockStateId, ChatEvent,
        ChunkGenEvent, ChunkHandle, ChunkPos, EntityDamageEvent, EntityHandle, EntityInfo,
        EntitySpawnEvent, EventCallback, EventContext, EventData, EventType, ExplosionEvent,
        GameMode, GameRandom, Hand, Identifier, ItemDropEvent, ItemUseEvent, LogLevel, ModApiError,
        ModDependency, ModMetadata, ModResult, Permission, PlayerHandle, PlayerInfo,
        PlayerJoinEvent, PlayerRespawnEvent, PropertyType, RecipeIngredient, RecipeResult, Vec3,
        achievements::{
            AchievementId, AchievementProgress, AchievementTier, AchievementsRegistry,
            PlayerAchievements, PlayerProfile, PlayerStatistics, StatCategory, Statistic,
        },
        host,
    };
}

/// Version of the mod API for compatibility checking
pub const MOD_API_VERSION: &str = "1.0.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mod_api_version() {
        assert_eq!(MOD_API_VERSION, "1.0.0");
    }

    #[test]
    fn test_event_type_serialization() {
        let event_type = EventType::BlockBreak;
        let serialized = serde_json::to_string(&event_type).unwrap();
        assert_eq!(serialized, "\"BlockBreak\"");
    }

    #[test]
    fn test_permission_serialization() {
        let perm = Permission::WorldRead;
        let serialized = serde_json::to_string(&perm).unwrap();
        assert_eq!(serialized, "\"WorldRead\"");
    }

    #[test]
    fn test_mod_dependency() {
        let dep = ModDependency {
            id: Identifier::new("minecraft", "test_mod").unwrap(),
            version: "^1.0.0".to_string(),
            optional: false,
        };
        let serialized = serde_json::to_string(&dep).unwrap();
        assert!(serialized.contains("test_mod"));
    }
}
