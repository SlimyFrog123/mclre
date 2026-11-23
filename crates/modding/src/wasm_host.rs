//! WASM sandbox host for running mods.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use parking_lot::RwLock;
use thiserror::Error;
use wasmtime::{
    Caller, Config, Engine, Store, StoreContextMut,
    component::{Component, Func, Instance, Linker, Resource, Val},
};

use crate::event_bus::EventBus;
use crate::manifest::ModManifest;
use mclre_core::{BlockId, BlockStateId, GameRandom, Identifier, ItemId, Vec3};
use mclre_mod_api::{
    BlockPos, ChunkPos, EntityHandle, EventType, GameMode, LogLevel, ModApiError, ModResult,
    Permission, PlayerHandle,
};

/// Errors that can occur in the WASM host
#[derive(Debug, Error)]
pub enum WasmError {
    #[error("Failed to create engine: {0}")]
    Engine(String),
    #[error("Failed to load module: {0}")]
    Module(String),
    #[error("Failed to instantiate: {0}")]
    Instantiate(String),
    #[error("Function not found: {0}")]
    FunctionNotFound(String),
    #[error("Execution failed: {0}")]
    Execution(String),
    #[error("Fuel exhausted")]
    FuelExhausted,
    #[error("Memory limit exceeded")]
    MemoryExceeded,
    #[error("Invalid export: {0}")]
    InvalidExport(String),
    #[error("Host function error: {0}")]
    HostFunction(String),
    #[error("Invalid component: {0}")]
    InvalidComponent(String),
    #[error("Host function error: {0}")]
    Anyhow(#[from] anyhow::Error),
}

/// Configuration for the WASM host
#[derive(Debug, Clone)]
pub struct WasmHostConfig {
    /// Maximum fuel (instructions) per tick
    pub max_fuel_per_tick: u64,
    /// Maximum memory in bytes
    pub max_memory: usize,
    /// Enable fuel metering
    pub enable_fuel: bool,
    /// Enable epoch-based interruption
    pub enable_epoch: bool,
    /// Allowed host functions
    pub allowed_host_functions: Vec<String>,
}

impl Default for WasmHostConfig {
    fn default() -> Self {
        Self {
            max_fuel_per_tick: 10_000_000, // 10M instructions per tick
            max_memory: 64 * 1024 * 1024,  // 64 MB
            enable_fuel: true,
            enable_epoch: true,
            allowed_host_functions: vec![
                "register_block".to_string(),
                "register_item".to_string(),
                "register_recipe".to_string(),
                "subscribe_event".to_string(),
                "get_block".to_string(),
                "set_block".to_string(),
                "get_player".to_string(),
                "get_entity".to_string(),
                "spawn_entity".to_string(),
                "send_chat".to_string(),
                "game_random".to_string(),
                "log".to_string(),
                "get_config".to_string(),
                "set_config".to_string(),
                "schedule_task".to_string(),
            ],
        }
    }
}

/// Player info returned by host
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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

/// Block property definition for guest registration
#[derive(Debug, Clone)]
pub struct BlockProperty {
    pub name: String,
    pub property_type: PropertyType,
    pub default_value: String,
    pub allowed_values: Vec<String>,
}

/// Property type for blocks
#[derive(Debug, Clone)]
pub enum PropertyType {
    Bool,
    Int(i32, i32), // min, max
    Enum(Vec<String>),
}

/// Recipe ingredient for guest registration
#[derive(Debug, Clone)]
pub struct RecipeIngredient {
    pub item: Identifier,
    pub count: u8,
    pub tag: Option<Identifier>,
}

/// Recipe result for guest registration
#[derive(Debug, Clone)]
pub struct RecipeResult {
    pub item: Identifier,
    pub count: u8,
}

/// Convert LogLevel to tracing::Level
fn log_level_to_tracing(level: LogLevel) -> tracing::Level {
    match level {
        LogLevel::Trace => tracing::Level::TRACE,
        LogLevel::Debug => tracing::Level::DEBUG,
        LogLevel::Info => tracing::Level::INFO,
        LogLevel::Warn => tracing::Level::WARN,
        LogLevel::Error => tracing::Level::ERROR,
    }
}

/// Log a message from a mod
fn host_log(level: LogLevel, message: &str) {
    let tracing_level = log_level_to_tracing(level);
    match tracing_level {
        tracing::Level::TRACE => tracing::trace!("[mod] {}", message),
        tracing::Level::DEBUG => tracing::debug!("[mod] {}", message),
        tracing::Level::INFO => tracing::info!("[mod] {}", message),
        tracing::Level::WARN => tracing::warn!("[mod] {}", message),
        tracing::Level::ERROR => tracing::error!("[mod] {}", message),
    }
}

/// WASM module instance with its state
pub struct WasmInstance {
    store: Store<WasmInstanceData>,
    instance: Instance,
    entry_point: Func,
    config: WasmHostConfig,
}

/// Per-instance data for host functions
pub struct WasmInstanceData {
    pub mod_id: String,
    pub event_bus: Arc<EventBus>,
    pub config: HashMap<String, String>,
    pub fuel_consumed: u64,
    pub random: GameRandom,
}

/// Main WASM host for running mods
pub struct WasmHost {
    engine: Engine,
    linker: Linker<WasmInstanceData>,
    config: WasmHostConfig,
    instances: RwLock<HashMap<String, WasmInstance>>,
    event_bus: Arc<EventBus>,
    server_random: GameRandom,
}

impl WasmHost {
    /// Create a new WASM host
    pub fn new(config: WasmHostConfig) -> Result<Self, WasmError> {
        let mut engine_config = Config::new();
        engine_config.wasm_component_model(true);

        if config.enable_fuel {
            engine_config.consume_fuel(true);
        }
        if config.enable_epoch {
            engine_config.epoch_interruption(true);
        }
        engine_config.max_wasm_stack(config.max_memory);

        let engine = Engine::new(&engine_config).map_err(|e| WasmError::Engine(e.to_string()))?;

        let mut linker = Linker::new(&engine);

        // Add host functions to linker
        Self::add_host_functions(&mut linker)?;

        let event_bus = Arc::new(EventBus::new());

        Ok(Self {
            engine,
            linker,
            config,
            instances: RwLock::new(HashMap::new()),
            event_bus,
            server_random: GameRandom::with_seed(0),
        })
    }

    /// Add host functions to the linker
    fn add_host_functions(linker: &mut Linker<WasmInstanceData>) -> Result<(), WasmError> {
        let mut root = linker.root();

        // World interface
        let mut world = root.instance("mclre:mod-api/world")?;

        // get-block(pos: {x: s32, y: s32, z: s32}) -> result<block-state-id, string>
        world.func_new(
            "get-block",
            |caller: StoreContextMut<'_, WasmInstanceData>,
             _params: &[Val],
             results: &mut [Val]|
             -> Result<()> {
                let _data = caller.data();
                // params[0] should be a record {x: s32, y: s32, z: s32}
                // TODO: Get block from world
                results[0] = Val::U32(0); // BlockStateId(0)
                Ok(())
            },
        )?;

        // set-block(pos: {x: s32, y: s32, z: s32}, state: block-state-id) -> result<_, string>
        world.func_new(
            "set-block",
            |_caller: StoreContextMut<'_, WasmInstanceData>,
             _params: &[Val],
             _results: &mut [Val]|
             -> Result<()> {
                // TODO: Set block in world
                Ok(())
            },
        )?;

        // Player interface
        let mut player = root.instance("mclre:mod-api/player")?;

        // get-player(handle: player-handle) -> result<player-info, string>
        player.func_new(
            "get-player",
            |_caller: StoreContextMut<'_, WasmInstanceData>,
             _params: &[Val],
             results: &mut [Val]|
             -> Result<()> {
                // TODO: Get player from server
                // Return a default player info as Val record
                results[0] = Val::Bool(true); // ok variant
                Ok(())
            },
        )?;

        // send-chat(handle: option<player-handle>, message: string) -> result<_, string>
        player.func_new(
            "send-chat",
            |_caller: StoreContextMut<'_, WasmInstanceData>,
             _params: &[Val],
             _results: &mut [Val]|
             -> Result<()> {
                // TODO: Send chat message
                Ok(())
            },
        )?;

        // Entity interface
        let mut entity = root.instance("mclre:mod-api/entity")?;

        // get-entity(handle: entity-handle) -> result<entity-info, string>
        entity.func_new(
            "get-entity",
            |_caller: StoreContextMut<'_, WasmInstanceData>,
             _params: &[Val],
             _results: &mut [Val]|
             -> Result<()> { Ok(()) },
        )?;

        // spawn-entity(entity-type: identifier, position: vec3, nbt: option<string>) -> result<entity-handle, string>
        entity.func_new(
            "spawn-entity",
            |_caller: StoreContextMut<'_, WasmInstanceData>,
             _params: &[Val],
             _results: &mut [Val]|
             -> Result<()> { Ok(()) },
        )?;

        // Registry interface
        let mut registry = root.instance("mclre:mod-api/registry")?;

        // register-block(id: identifier, name: string, properties: list<block-property>) -> result<block-state-id, string>
        registry.func_new(
            "register-block",
            |_caller: StoreContextMut<'_, WasmInstanceData>,
             _params: &[Val],
             _results: &mut [Val]|
             -> Result<()> { Ok(()) },
        )?;

        // register-item(id: identifier, name: string, max-stack: u8, durability: option<u16>) -> result<identifier, string>
        registry.func_new(
            "register-item",
            |_caller: StoreContextMut<'_, WasmInstanceData>,
             _params: &[Val],
             _results: &mut [Val]|
             -> Result<()> { Ok(()) },
        )?;

        // register-recipe(id: identifier, recipe-type: string, ingredients: list<recipe-ingredient>, result: recipe-result) -> result<_, string>
        registry.func_new(
            "register-recipe",
            |_caller: StoreContextMut<'_, WasmInstanceData>,
             _params: &[Val],
             _results: &mut [Val]|
             -> Result<()> { Ok(()) },
        )?;

        // Config interface
        let mut config = root.instance("mclre:mod-api/config")?;

        // get-config(key: string) -> result<option<string>, string>
        config.func_new(
            "get-config",
            |caller: StoreContextMut<'_, WasmInstanceData>,
             params: &[Val],
             results: &mut [Val]|
             -> Result<()> {
                let data = caller.data();
                if let Some(Val::String(key)) = params.get(0) {
                    if let Some(value) = data.config.get(key.as_str()) {
                        results[0] = Val::Bool(true); // some
                        results[1] = Val::String(value.clone().into());
                    } else {
                        results[0] = Val::Bool(false); // none
                    }
                }
                Ok(())
            },
        )?;

        // set-config(key: string, value: string) -> result<_, string>
        config.func_new(
            "set-config",
            |mut caller: StoreContextMut<'_, WasmInstanceData>,
             params: &[Val],
             _results: &mut [Val]|
             -> Result<()> {
                let data = caller.data_mut();
                if let (Some(Val::String(key)), Some(Val::String(value))) =
                    (params.get(0), params.get(1))
                {
                    data.config
                        .insert(key.as_str().to_string(), value.as_str().to_string());
                }
                Ok(())
            },
        )?;

        // Events interface
        let mut events = root.instance("mclre:mod-api/events")?;

        // subscribe(event-type: event-type, callback: func(event-data) -> result<_, host-error>) -> result<u64, host-error>
        events.func_new(
            "subscribe",
            |_caller: StoreContextMut<'_, WasmInstanceData>,
             _params: &[Val],
             _results: &mut [Val]|
             -> Result<()> { Ok(()) },
        )?;

        // unsubscribe(subscription-id: u64) -> result<_, host-error>
        events.func_new(
            "unsubscribe",
            |_caller: StoreContextMut<'_, WasmInstanceData>,
             _params: &[Val],
             _results: &mut [Val]|
             -> Result<()> { Ok(()) },
        )?;

        // Utils interface
        let mut utils = root.instance("mclre:mod-api/utils")?;

        // game-random() -> u64
        utils.func_new(
            "game-random",
            |mut caller: StoreContextMut<'_, WasmInstanceData>,
             _params: &[Val],
             results: &mut [Val]|
             -> Result<()> {
                let data = caller.data_mut();
                let random = data.random.next_u64();
                results[0] = Val::U64(random);
                Ok(())
            },
        )?;

        // log(level: log-level, message: string)
        utils.func_new(
            "log",
            |_caller: StoreContextMut<'_, WasmInstanceData>,
             params: &[Val],
             _results: &mut [Val]|
             -> Result<()> {
                if let (Some(Val::U8(level)), Some(Val::String(message))) =
                    (params.get(0), params.get(1))
                {
                    match level {
                        0 => tracing::trace!("[mod] {}", message), // Trace
                        1 => tracing::debug!("[mod] {}", message), // Debug
                        2 => tracing::info!("[mod] {}", message),  // Info
                        3 => tracing::warn!("[mod] {}", message),  // Warn
                        4 => tracing::error!("[mod] {}", message), // Error
                        _ => tracing::info!("[mod] {}", message),
                    };
                }
                Ok(())
            },
        )?;

        // schedule-task(ticks: u64, callback: func() -> result<_, host-error>) -> result<u64, host-error>
        utils.func_new(
            "schedule-task",
            |_caller: StoreContextMut<'_, WasmInstanceData>,
             _params: &[Val],
             _results: &mut [Val]|
             -> Result<()> { Ok(()) },
        )?;

        Ok(())
    }

    /// Load a mod from its manifest
    pub fn load_mod(&mut self, manifest: &ModManifest) -> Result<(), WasmError> {
        // Read WASM file
        let wasm_bytes =
            std::fs::read(&manifest.wasm_path).map_err(|e| WasmError::Module(e.to_string()))?;

        // Validate it's a component
        let component = Component::new(&self.engine, &wasm_bytes)
            .map_err(|e| WasmError::InvalidComponent(e.to_string()))?;

        // Create store with instance data
        let mut store = Store::new(
            &self.engine,
            WasmInstanceData {
                mod_id: manifest.id().to_string(),
                event_bus: self.event_bus.clone(),
                config: HashMap::new(),
                fuel_consumed: 0,
                random: GameRandom::with_seed(self.server_random.next_u64() as i64),
            },
        );

        // Instantiate
        let instance = self
            .linker
            .instantiate(&mut store, &component)
            .map_err(|e| WasmError::Instantiate(e.to_string()))?;

        // Get entry point function
        let entry_point = instance
            .get_func(&mut store, &manifest.entry_point)
            .ok_or_else(|| WasmError::FunctionNotFound(manifest.entry_point.clone()))?;

        // Store instance
        let wasm_instance = WasmInstance {
            store,
            instance,
            entry_point,
            config: self.config.clone(),
        };

        self.instances
            .write()
            .insert(manifest.id().to_string(), wasm_instance);

        // Call init function
        self.call_init(&manifest.id().to_string())?;

        Ok(())
    }

    /// Call the mod's init function (placeholder)
    fn call_init(&self, _mod_id: &str) -> Result<(), WasmError> {
        // TODO: Implement component model function calling
        Ok(())
    }

    /// Call a function in a mod (placeholder - not fully implemented for component model)
    pub fn call_function(
        &self,
        _mod_id: &str,
        _func_name: &str,
        _args: &[u8],
    ) -> Result<Vec<u8>, WasmError> {
        // TODO: Implement component model function calling with proper Val types
        Err(WasmError::HostFunction(
            "Function calling not yet implemented".into(),
        ))
    }

    /// Tick a mod (call its tick function)
    pub fn tick_mod(&self, mod_id: &str) -> Result<(), WasmError> {
        // Call tick function via init/entry point
        self.call_init(mod_id)?;

        Ok(())
    }

    /// Tick all loaded mods
    pub fn tick_all(&self) -> Result<(), WasmError> {
        let mod_ids: Vec<String> = self.instances.read().keys().cloned().collect();
        for mod_id in mod_ids {
            if let Err(e) = self.tick_mod(&mod_id) {
                // Log error but continue ticking other mods
                tracing::error!("Error ticking mod {}: {}", mod_id, e);
            }
        }
        Ok(())
    }

    /// Unload a mod
    pub fn unload_mod(&self, mod_id: &str) -> Result<(), WasmError> {
        self.instances.write().remove(mod_id);
        Ok(())
    }

    /// Get the event bus
    pub fn event_bus(&self) -> Arc<EventBus> {
        self.event_bus.clone()
    }

    /// Get loaded mod IDs
    pub fn loaded_mods(&self) -> Vec<String> {
        self.instances.read().keys().cloned().collect()
    }

    /// Check if a mod is loaded
    pub fn is_loaded(&self, mod_id: &str) -> bool {
        self.instances.read().contains_key(mod_id)
    }

    /// Increment epoch for interruption
    pub fn increment_epoch(&self) {
        self.engine.increment_epoch();
    }
}

/// Host function implementations
mod host_impl {
    use super::*;
    use wasmtime::component::Resource;
    use wasmtime::{Val, ValType};

    // These would be implemented using the component model
    // For now, stubs showing the intended interface
}

/// Convert WASM values to/from Rust types
mod wasm_conversion {
    use super::*;
    use wasmtime::Val;

    pub fn val_to_i32(val: &Val) -> Result<i32, WasmError> {
        match val {
            Val::I32(i) => Ok(*i),
            _ => Err(WasmError::InvalidExport("Expected i32".into())),
        }
    }

    pub fn val_to_i64(val: &Val) -> Result<i64, WasmError> {
        match val {
            Val::I64(i) => Ok(*i),
            _ => Err(WasmError::InvalidExport("Expected i64".into())),
        }
    }

    pub fn val_to_f32(val: &Val) -> Result<f32, WasmError> {
        match val {
            Val::F32(f) => Ok(f32::from_bits(*f)),
            _ => Err(WasmError::InvalidExport("Expected f32".into())),
        }
    }

    pub fn val_to_f64(val: &Val) -> Result<f64, WasmError> {
        match val {
            Val::F64(f) => Ok(f64::from_bits(*f)),
            _ => Err(WasmError::InvalidExport("Expected f64".into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_wasm_host_creation() {
        let config = WasmHostConfig::default();
        WasmHost::new(config).expect("WasmHost::new should succeed with default config");
    }

    #[test]
    fn test_wasm_host_config() {
        let config = WasmHostConfig::default();
        assert_eq!(config.max_fuel_per_tick, 10_000_000);
        assert_eq!(config.max_memory, 64 * 1024 * 1024);
        assert!(config.enable_fuel);
        assert!(config.enable_epoch);
    }
}
