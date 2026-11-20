use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::Mutex;
use bevy_ecs::prelude::*;
use mclre_core::{GameRandom, Identifier, BlockStateId};
use mclre_core::mth::Vec3;
use mclre_world::{ChunkPos, ChunkColumn};
use mclre_protocol::play::{PlayerPosition, PlayerPositionAndLook, PlayerLook};
use mclre_identity::PlayerIdentity;
use thiserror::Error;
use tracing::{info, warn};

/// Fixed timestep: 20 TPS = 50ms per tick
pub const TICK_RATE: u64 = 20;
pub const TICK_DURATION_MS: u64 = 1000 / TICK_RATE;
pub const TICK_DURATION: Duration = Duration::from_millis(TICK_DURATION_MS);

/// Simulation errors.
#[derive(Debug, Error)]
pub enum SimError {
    #[error("World error: {0}")]
    World(String),

    #[error("Entity error: {0}")]
    Entity(String),

    #[error("Physics error: {0}")]
    Physics(String),

    #[error("Player not found: {0}")]
    PlayerNotFound(String),
}

/// Player entity data for simulation.
#[derive(Debug, Clone, Component)]
pub struct SimPlayer {
    pub identity: PlayerIdentity,
    pub entity_id: i32,
    pub position: Vec3<f32>,
    pub velocity: Vec3<f32>,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub gamemode: GameMode,
    pub health: f32,
    pub food: f32,
    pub food_saturation: f32,
}

/// Game mode for simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    Survival = 0,
    Creative = 1,
    Adventure = 2,
    Spectator = 3,
}

/// Input state for player movement.
#[derive(Debug, Clone, Default)]
pub struct PlayerInput {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub sprint: bool,
    pub sneak: bool,
}

/// Physics constants matching LCE reference.
pub mod physics {
    /// Gravity acceleration (blocks/tick²)
    pub const GRAVITY: f32 = 0.08;

    /// Terminal velocity (blocks/tick)
    pub const TERMINAL_VELOCITY: f32 = 78.4;

    /// Drag coefficient for air
    pub const DRAG_AIR: f32 = 0.02;

    /// Drag coefficient for water
    pub const DRAG_WATER: f32 = 0.99;

    /// Drag coefficient for lava
    pub const DRAG_LAVA: f32 = 0.95;

    /// Jump velocity
    pub const JUMP_VELOCITY: f32 = 0.42;

    /// Sprint jump velocity
    pub const SPRINT_JUMP_VELOCITY: f32 = 0.5;

    /// Walk speed (blocks/tick)
    pub const WALK_SPEED: f32 = 0.1;

    /// Sprint speed multiplier
    pub const SPRINT_MULTIPLIER: f32 = 1.3;

    /// Sneak speed multiplier
    pub const SNEAK_MULTIPLIER: f32 = 0.3;

    /// Player bounding box half-extents
    pub const PLAYER_HALF_WIDTH: f32 = 0.3;
    pub const PLAYER_HEIGHT: f32 = 1.8;
    pub const PLAYER_EYE_HEIGHT: f32 = 1.62;
}

/// Chunk storage for simulation (wraps ChunkColumn from world crate).
pub type SimChunk = ChunkColumn;

/// Simulation world containing all server-side state.
pub struct SimWorld {
    pub chunks: Arc<Mutex<std::collections::HashMap<ChunkPos, SimChunk>>>,
    pub ecs_world: bevy_ecs::world::World,
    pub tick_count: u64,
    pub rng: GameRandom,
    /// Pending player position updates from clients
    pub pending_positions: Vec<(PlayerIdentity, PlayerPositionUpdate)>,
}

/// Player position update from client.
#[derive(Debug, Clone)]
pub enum PlayerPositionUpdate {
    Position(PlayerPosition),
    PositionAndLook(PlayerPositionAndLook),
    Look(PlayerLook),
}

impl SimWorld {
    /// Create a new simulation world.
    pub fn new(seed: i64) -> Self {
        let mut ecs_world = bevy_ecs::world::World::new();
        
        Self {
            chunks: Arc::new(Mutex::new(std::collections::HashMap::new())),
            ecs_world,
            tick_count: 0,
            rng: GameRandom::with_seed(seed),
            pending_positions: Vec::new(),
        }
    }

    /// Run a single simulation tick (20 TPS fixed timestep).
    pub fn tick(&mut self) -> Result<(), SimError> {
        self.tick_count += 1;
        
        // Process pending player position updates
        self.process_player_updates()?;
        
        // Run physics for all entities
        self.run_physics()?;
        
        // Run entity AI
        self.run_ai()?;
        
        // Update block ticks (redstone, fluids, etc.)
        self.update_block_ticks()?;
        
        Ok(())
    }

    /// Process pending player position updates from clients.
    fn process_player_updates(&mut self) -> Result<(), SimError> {
        let updates = std::mem::take(&mut self.pending_positions);
        for (identity, update) in updates {
            if let Some(mut player) = self.get_player_mut(&identity) {
                match update {
                    PlayerPositionUpdate::Position(pos) => {
                        player.position = Vec3::new(pos.x as f32, pos.y as f32, pos.z as f32);
                        player.on_ground = pos.on_ground;
                    }
                    PlayerPositionUpdate::PositionAndLook(pos) => {
                        player.position = Vec3::new(pos.x as f32, pos.y as f32, pos.z as f32);
                        player.yaw = pos.yaw;
                        player.pitch = pos.pitch;
                        player.on_ground = pos.on_ground;
                    }
                    PlayerPositionUpdate::Look(look) => {
                        player.yaw = look.yaw;
                        player.pitch = look.pitch;
                        player.on_ground = look.on_ground;
                    }
                }
            }
        }
        Ok(())
    }

    /// Run physics simulation for all entities.
    fn run_physics(&mut self) -> Result<(), SimError> {
        // Apply gravity to players
        let mut query = self.ecs_world.query::<&mut SimPlayer>();
        for mut player in query.iter_mut(&mut self.ecs_world) {
            // Apply gravity if not on ground
            if !player.on_ground {
                player.velocity.y -= physics::GRAVITY;
                
                // Clamp to terminal velocity
                if player.velocity.y < -physics::TERMINAL_VELOCITY {
                    player.velocity.y = -physics::TERMINAL_VELOCITY;
                }
            }
            
            // Apply velocity to position (copy velocity to avoid borrow conflict)
            let velocity = player.velocity;
            player.position += velocity;
            
            // TODO: Collision detection with blocks
            // For now, simple ground check at Y=64 (flat world ground level)
            if player.position.y <= 64.0 && player.velocity.y <= 0.0 {
                player.position.y = 64.0;
                player.on_ground = true;
                player.velocity.y = 0.0;
            }
        }
        Ok(())
    }

    /// Run entity AI.
    fn run_ai(&mut self) -> Result<(), SimError> {
        // TODO: Implement mob AI
        Ok(())
    }

    /// Update block ticks (redstone, fluids, crops, etc.).
    fn update_block_ticks(&mut self) -> Result<(), SimError> {
        // TODO: Implement block ticking
        Ok(())
    }

    /// Add a player to the simulation.
    pub fn add_player(&mut self, identity: PlayerIdentity, entity_id: i32, position: Vec3<f32>) {
        let player = SimPlayer {
            identity: identity.clone(),
            entity_id,
            position,
            velocity: Vec3::new(0.0, 0.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            on_ground: true,
            gamemode: GameMode::Survival,
            health: 20.0,
            food: 20.0,
            food_saturation: 5.0,
        };
        self.ecs_world.spawn(player);
    }

    /// Remove a player from the simulation.
    pub fn remove_player(&mut self, identity: &PlayerIdentity) -> bool {
        let mut query = self.ecs_world.query::<(Entity, &SimPlayer)>();
        let entities: Vec<_> = query.iter(&self.ecs_world)
            .filter(|(_, p)| p.identity == *identity)
            .map(|(e, _)| e)
            .collect();
        
        for entity in &entities {
            self.ecs_world.despawn(*entity);
        }
        !entities.is_empty()
    }

    /// Get a player by identity (mutable).
    fn get_player_mut(&mut self, identity: &PlayerIdentity) -> Option<Mut<SimPlayer>> {
        let mut query = self.ecs_world.query::<&mut SimPlayer>();
        for mut player in query.iter_mut(&mut self.ecs_world) {
            if player.identity == *identity {
                return Some(player);
            }
        }
        None
    }

    /// Get a player by identity.
    pub fn get_player(&mut self, identity: &PlayerIdentity) -> Option<&SimPlayer> {
        let mut query = self.ecs_world.query::<&SimPlayer>();
        for player in query.iter(&self.ecs_world) {
            if player.identity == *identity {
                return Some(player);
            }
        }
        None
    }

    /// Queue a player position update for the next tick.
    pub fn queue_position_update(&mut self, identity: PlayerIdentity, update: PlayerPositionUpdate) {
        self.pending_positions.push((identity, update));
    }

    /// Get current tick count.
    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }
}

/// Simulation runner that manages the fixed timestep loop.
pub struct SimRunner {
    sim_world: Arc<Mutex<SimWorld>>,
    running: Arc<Mutex<bool>>,
}

impl SimRunner {
    /// Create a new simulation runner.
    pub fn new(sim_world: SimWorld) -> Self {
        Self {
            sim_world: Arc::new(Mutex::new(sim_world)),
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// Run the simulation loop (blocks until stopped).
    pub fn run(&self) {
        *self.running.lock() = true;
        info!("Starting simulation loop at {} TPS", TICK_RATE);
        
        while *self.running.lock() {
            let tick_start = Instant::now();
            
            // Run simulation tick
            if let Err(e) = self.sim_world.lock().tick() {
                warn!("Simulation tick error: {}", e);
            }
            
            // Calculate sleep time to maintain 20 TPS
            let elapsed = tick_start.elapsed();
            if elapsed < TICK_DURATION {
                let sleep_time = TICK_DURATION - elapsed;
                std::thread::sleep(sleep_time);
            } else {
                // Tick took too long, log warning
                warn!("Tick took {}ms (budget: {}ms)", elapsed.as_millis(), TICK_DURATION_MS);
            }
        }
        
        info!("Simulation loop stopped");
    }

    /// Stop the simulation loop.
    pub fn stop(&self) {
        *self.running.lock() = false;
    }

    /// Get a reference to the simulation world.
    pub fn world(&self) -> Arc<Mutex<SimWorld>> {
        self.sim_world.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mclre_core::Vec3;

    #[test]
    fn test_sim_world_creation() {
        let sim = SimWorld::new(12345);
        assert_eq!(sim.tick_count, 0);
    }

    #[test]
    fn test_sim_tick() {
        let mut sim = SimWorld::new(12345);
        
        let identity = PlayerIdentity::new(uuid::Uuid::now_v7(), "TestPlayer".to_string());
        sim.add_player(identity.clone(), 1, Vec3::new(0.0, 64.0, 0.0));
        
        sim.tick().unwrap();
        assert_eq!(sim.tick_count, 1);
        
        let player = sim.get_player(&identity).unwrap();
        assert_eq!(player.entity_id, 1);
    }

    #[test]
    fn test_player_position_update() {
        let mut sim = SimWorld::new(12345);
        
        let identity = PlayerIdentity::new(uuid::Uuid::now_v7(), "TestPlayer".to_string());
        sim.add_player(identity.clone(), 1, Vec3::new(0.0, 64.0, 0.0));
        
        let update = PlayerPosition {
            x: 10.0,
            y: 65.0,
            z: 5.0,
            on_ground: true,
        };
        sim.queue_position_update(identity.clone(), PlayerPositionUpdate::Position(update));
        sim.tick().unwrap();
        
        let player = sim.get_player(&identity).unwrap();
        assert!((player.position.x - 10.0).abs() < 0.001);
        assert!((player.position.y - 65.0).abs() < 0.001);
        assert!((player.position.z - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_gravity() {
        let mut sim = SimWorld::new(12345);
        
        let identity = PlayerIdentity::new(uuid::Uuid::now_v7(), "TestPlayer".to_string());
        sim.add_player(identity.clone(), 1, Vec3::new(0.0, 100.0, 0.0));
        
        // Player starts in air
        {
            let mut query = sim.ecs_world.query::<&mut SimPlayer>();
            for mut player in query.iter_mut(&mut sim.ecs_world) {
                player.on_ground = false;
            }
        }
        
        sim.tick().unwrap();
        
        let player = sim.get_player(&identity).unwrap();
        // Should have fallen due to gravity
        assert!(player.velocity.y < 0.0);
        assert!(player.position.y < 100.0);
    }
}