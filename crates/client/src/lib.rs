//! Client library: ClientWorld mirror, prediction, interpolation, local players, UI state machine.

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender};
use mclre_assets::{AssetManager, Identifier};
use mclre_content_vanilla::block_states;
use mclre_core::{BlockStateId, Vec3};
use mclre_input::{Action, InputManager};
use mclre_world::{ChunkColumn, ChunkPos, ChunkSection, WorldSizePreset};
use parking_lot::Mutex;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::Window,
};

/// The client's view of the world (mirror of server state).
#[derive(Debug)]
pub struct ClientWorld {
    /// Loaded chunk columns.
    chunks: Mutex<HashMap<ChunkPos, Arc<ChunkColumn>>>,
    /// Chunks that need to be uploaded to GPU.
    dirty_chunks: Mutex<Vec<ChunkPos>>,
    /// GPU chunk sections ready for rendering.
    gpu_chunks: Mutex<HashMap<ChunkPos, Vec<mclre_render::resources::GpuChunkSection>>>,
    /// Local player position.
    player_pos: Mutex<Vec3<f32>>,
    /// Local player rotation (yaw, pitch).
    player_rot: Mutex<(f32, f32)>,
}

impl ClientWorld {
    /// Creates a new empty client world.
    pub fn new() -> Self {
        Self {
            chunks: Mutex::new(HashMap::new()),
            dirty_chunks: Mutex::new(Vec::new()),
            gpu_chunks: Mutex::new(HashMap::new()),
            player_pos: Mutex::new(Vec3::new(0.0, 64.0, 0.0)),
            player_rot: Mutex::new((0.0, 0.0)),
        }
    }

    /// Sets the player position.
    pub fn set_player_pos(&self, pos: Vec3<f32>) {
        *self.player_pos.lock() = pos;
    }

    /// Gets the player position.
    pub fn player_pos(&self) -> Vec3<f32> {
        *self.player_pos.lock()
    }

    /// Sets the player rotation (yaw, pitch in radians).
    pub fn set_player_rot(&self, yaw: f32, pitch: f32) {
        *self.player_rot.lock() = (yaw, pitch);
    }

    /// Gets the player rotation (yaw, pitch).
    pub fn player_rot(&self) -> (f32, f32) {
        *self.player_rot.lock()
    }

    /// Adds or updates a chunk.
    pub fn set_chunk(&self, chunk: Arc<ChunkColumn>) {
        let pos = chunk.pos();
        self.chunks.lock().insert(pos, chunk);
        self.dirty_chunks.lock().push(pos);
    }

    /// Gets a chunk.
    pub fn get_chunk(&self, pos: ChunkPos) -> Option<Arc<ChunkColumn>> {
        self.chunks.lock().get(&pos).cloned()
    }

    /// Takes the list of dirty chunks that need GPU upload.
    pub fn take_dirty_chunks(&self) -> Vec<ChunkPos> {
        let mut dirty = self.dirty_chunks.lock();
        std::mem::take(&mut *dirty)
    }

    /// Stores GPU chunk sections for rendering.
    pub fn store_gpu_chunks(
        &self,
        pos: ChunkPos,
        gpu_sections: Vec<mclre_render::resources::GpuChunkSection>,
    ) {
        self.gpu_chunks.lock().insert(pos, gpu_sections);
    }

    /// Gets all GPU chunk sections for rendering.
    pub fn get_gpu_chunks(&self) -> Vec<mclre_render::resources::GpuChunkSection> {
        self.gpu_chunks.lock().values().flatten().cloned().collect()
    }

    /// Creates a view from the player's perspective.
    pub fn create_view(&self, aspect: f32) -> mclre_render::View {
        let pos = self.player_pos();
        let (yaw, pitch) = self.player_rot();
        mclre_render::View::new(
            pos,
            yaw,
            pitch,
            std::f32::consts::FRAC_PI_2,
            aspect,
            0.1,
            256.0,
        )
    }
}

impl Default for ClientWorld {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a flat test world with some initial chunks around the spawn point.
///
/// This creates a simple flat world with:
/// - Bedrock at y=0
/// - Stone from y=1 to y=60
/// - Dirt from y=61 to y=62
/// - Grass block at y=63
/// - Air above
///
/// Returns a tuple of (block_state_model, vec of (ChunkPos, ChunkColumn))
pub fn create_flat_test_world() -> Result<
    (mclre_core::BlockStateModel, Vec<(ChunkPos, ChunkColumn)>),
    mclre_content_vanilla::ContentError,
> {
    let model = block_states()?;

    // Get block state IDs for our flat world blocks
    let air_state = model
        .default_state(mclre_core::BlockId(0))
        .unwrap_or(mclre_core::BlockStateId(0)); // air is index 0
    let stone_state = model
        .default_state(mclre_core::BlockId(1))
        .unwrap_or(mclre_core::BlockStateId(1)); // stone is index 1
    let dirt_state = model
        .default_state(mclre_core::BlockId(2))
        .unwrap_or(mclre_core::BlockStateId(2)); // dirt is index 2
    let grass_state = model
        .default_state(mclre_core::BlockId(3))
        .unwrap_or(mclre_core::BlockStateId(3)); // grass_block is index 3
    let bedrock_state = model
        .default_state(mclre_core::BlockId(4))
        .unwrap_or(mclre_core::BlockStateId(4)); // bedrock is index 4

    let mut columns = Vec::new();

    // Create a 3x3 chunk area around spawn (0,0)
    for cx in -1..=1 {
        for cz in -1..=1 {
            let chunk_pos = ChunkPos::new(cx, cz);
            let mut sections = Vec::new();

            // 16 sections (256 blocks tall)
            for section_y in 0..16 {
                let section_base_y = section_y * 16;
                let section = if section_base_y == 0 {
                    // Bedrock at y=0
                    ChunkSection::filled(bedrock_state)
                } else if section_base_y <= 60 {
                    // Stone from y=1 to y=60
                    ChunkSection::filled(stone_state)
                } else if section_base_y <= 62 {
                    // Dirt from y=61 to y=62
                    ChunkSection::filled(dirt_state)
                } else if section_base_y == 63 {
                    // Grass at y=63
                    ChunkSection::filled(grass_state)
                } else {
                    // Air above
                    ChunkSection::air()
                };
                sections.push(section);
            }

            let column =
                ChunkColumn::from_sections(chunk_pos, sections).expect("valid chunk column");
            columns.push((chunk_pos, column));
        }
    }

    Ok((model, columns))
}

/// Client configuration.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Window title.
    pub title: String,
    /// Window width.
    pub width: u32,
    /// Window height.
    pub height: u32,
    /// Target frame time (for fixed timestep).
    pub target_frame_time: Duration,
    /// World size preset.
    pub world_preset: WorldSizePreset,
    /// Number of split-screen players (1-4).
    pub split_screen_players: usize,
    /// Initial player position.
    pub spawn_pos: Vec3<f32>,
    /// Enable VSync.
    pub enable_vsync: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            title: "Minecraft: Legacy Rust Edition".to_string(),
            width: 1280,
            height: 720,
            target_frame_time: Duration::from_millis(50), // 20 TPS
            world_preset: WorldSizePreset::Infinite,
            split_screen_players: 1,
            spawn_pos: Vec3::new(0.0, 64.0, 0.0),
            enable_vsync: true,
        }
    }
}

/// Commands sent from the game thread to the render thread.
#[derive(Debug)]
pub enum RenderCommand {
    /// Upload a chunk to GPU.
    UploadChunk {
        chunk_pos: mclre_world::ChunkPos,
        sections: Vec<mclre_world::section::ChunkSection>,
    },
    /// Remove a chunk from GPU.
    RemoveChunk(mclre_world::ChunkPos),
    /// Update view uniform.
    UpdateView(mclre_render::View),
    /// Shutdown the renderer.
    Shutdown,
}

/// Events from the render thread.
#[derive(Debug)]
pub enum RenderEvent {
    FrameComplete,
    Error(String),
}

/// The main client struct.
pub struct Client {
    config: ClientConfig,
    world: Arc<ClientWorld>,
    input: Mutex<InputManager>,
    asset_manager: Arc<Mutex<mclre_assets::AssetManager>>,
    block_state_model: Arc<mclre_core::BlockStateModel>,
    window: Option<Arc<winit::window::Window>>,
    network_transports: Vec<Option<Arc<dyn mclre_net::Transport>>>,
}

impl Client {
    /// Creates a new client (window created in run()).
    pub fn new(config: ClientConfig) -> Result<Self, ClientError> {
        let world = Arc::new(ClientWorld::new());
        world.set_player_pos(config.spawn_pos);

        // Create initial flat test world
        let (block_state_model, columns) =
            create_flat_test_world().expect("Failed to create flat test world");

        // Populate the world with initial chunks
        for (pos, column) in columns {
            world.set_chunk(Arc::new(column));
        }

        let input = Mutex::new(InputManager::new());

        // Create asset manager
        let asset_manager = Arc::new(Mutex::new(mclre_assets::AssetManager::new()));

        // Initialize network transports for split-screen (up to 4 players)
        let max_players = config.split_screen_players.clamp(1, 4);
        let network_transports = vec![None; max_players];

        Ok(Self {
            config,
            world,
            input,
            asset_manager,
            block_state_model: Arc::new(block_state_model),
            window: None,
            network_transports,
        })
    }

    /// Returns a mutable reference to the network transports.
    pub fn network_transports_mut(&mut self) -> &mut Vec<Option<Arc<dyn mclre_net::Transport>>> {
        &mut self.network_transports
    }

    /// Renders a frame using the provided renderer.
    fn render_frame_with_renderer(&self, renderer: &mut mclre_render::Renderer) {
        // Take dirty chunks and upload to GPU, storing the results
        let dirty_chunks = self.world.take_dirty_chunks();
        for pos in dirty_chunks {
            if let Some(chunk) = self.world.get_chunk(pos) {
                let mut gpu_sections = Vec::new();
                let chunk_pos_vec = pos.into(); // Convert ChunkPos to Vec3<i32>
                for section in chunk.sections() {
                    if section.is_empty() {
                        continue;
                    }
                    let gpu_chunk = mclre_render::resources::GpuChunkSection::from_section(
                        renderer.device(),
                        renderer.queue(),
                        section,
                        chunk_pos_vec,
                        Some(&self.block_state_model),
                    );
                    gpu_sections.push(gpu_chunk);
                }
                if !gpu_sections.is_empty() {
                    self.world.store_gpu_chunks(pos, gpu_sections);
                }
            }
        }

        // Get view
        let view = self
            .world
            .create_view(self.config.width as f32 / self.config.height as f32);

        // Get all GPU chunks for rendering
        let chunks = self.world.get_gpu_chunks();

        // Render frame
        if let Err(e) = renderer.render(&view, &chunks) {
            eprintln!("Render error: {}", e);
        }
    }

    /// Runs the main game loop.
    pub fn run(mut self) -> Result<(), ClientError> {
        let event_loop = EventLoop::new()?;
        let window = Arc::new(
            event_loop.create_window(
                winit::window::WindowAttributes::default()
                    .with_title(&self.config.title)
                    .with_inner_size(winit::dpi::LogicalSize::new(
                        self.config.width,
                        self.config.height,
                    )),
            )?,
        );

        // Store window in client
        self.window = Some(window.clone());

        // Create renderer in main thread (where window was created)
        let mut renderer = pollster::block_on(mclre_render::Renderer::new(window.clone()))
            .expect("Failed to create renderer");

        // Load pre-built texture atlas from converted game-assets
        {
            let mut asset_manager = self.asset_manager.lock();
            asset_manager.add_root(std::path::PathBuf::from("game-assets"));

            // Load the pre-built texture atlas from the converted assets
            let atlas_path = "textures/block/atlas.png";
            let atlas_id = mclre_assets::Identifier::new("minecraft", "atlas").unwrap();

            if let Ok(region) = asset_manager.load_into_atlas(atlas_id, "textures/block/atlas.png")
            {
                println!(
                    "Loaded pre-built atlas: region ({}, {}) {}x{}",
                    region.x, region.y, region.width, region.height
                );
            } else {
                eprintln!("Failed to load pre-built atlas, falling back to individual textures");
            }

            // Load UV coordinates from manifest.json for individual texture lookup
            let manifest_path = std::path::PathBuf::from("game-assets/manifest.json");
            let uvs: std::collections::HashMap<String, [f32; 4]> = if manifest_path.exists() {
                let manifest_data = std::fs::read_to_string(&manifest_path).unwrap_or_default();
                let manifest: serde_json::Value =
                    serde_json::from_str(&manifest_data).unwrap_or(serde_json::json!({}));
                let mut uvs = std::collections::HashMap::new();
                if let Some(textures) = manifest.get("textures").and_then(|t| t.as_object()) {
                    for (name, region) in textures {
                        if let (Some(u_min), Some(v_min), Some(u_max), Some(v_max)) = (
                            region.get("u_min").and_then(|v| v.as_f64()),
                            region.get("v_min").and_then(|v| v.as_f64()),
                            region.get("u_max").and_then(|v| v.as_f64()),
                            region.get("v_max").and_then(|v| v.as_f64()),
                        ) {
                            uvs.insert(
                                name.clone(),
                                [u_min as f32, v_min as f32, u_max as f32, v_max as f32],
                            );
                        }
                    }
                }
                println!("Loaded {} texture UV regions from manifest", uvs.len());
                uvs
            } else {
                std::collections::HashMap::new()
            };

            // Upload atlas to GPU
            let atlas_texture = asset_manager.atlas_texture().clone();
            let uvs_count = uvs.len();
            let padding = 1;
            if let Err(e) = renderer.set_texture_atlas(&atlas_texture, uvs, padding) {
                eprintln!("Failed to set texture atlas: {}", e);
            } else {
                println!(
                    "Texture atlas set successfully on renderer with {} UV regions",
                    uvs_count
                );
            }
        }

        // Setup input manager
        self.input.lock().set_mouse_captured(true);
        window
            .set_cursor_grab(winit::window::CursorGrabMode::Confined)
            .ok();
        window.set_cursor_visible(false);

        // Store window in client
        self.window = Some(window.clone());

        let mut last_frame = Instant::now();
        let mut accumulator = Duration::ZERO;
        let fixed_dt = self.config.target_frame_time;

        let mut client = self;

        event_loop.run(move |event, event_loop| {
            event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        event_loop.exit();
                    }
                    WindowEvent::Resized(size) => {
                        // Handle resize
                        renderer.resize(size.width, size.height);
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        client.input.lock().process_keyboard(event);
                    }
                    WindowEvent::MouseInput { button, state, .. } => {
                        client.input.lock().process_mouse_button(button, state);
                    }
                    WindowEvent::MouseWheel { delta, .. } => {
                        client.input.lock().process_mouse_scroll(delta);
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        // Handled by mouse delta in input manager
                    }
                    WindowEvent::Focused(focused) => {
                        if !focused {
                            client.input.lock().set_mouse_captured(false);
                            client.window.as_ref().map(|w| w.set_cursor_visible(true));
                            client.window.as_ref().map(|w| {
                                w.set_cursor_grab(winit::window::CursorGrabMode::None).ok()
                            });
                        }
                    }
                    _ => {}
                },
                Event::DeviceEvent { event, .. } => {
                    use winit::event::DeviceEvent;
                    if let DeviceEvent::MouseMotion { delta } = event {
                        client
                            .input
                            .lock()
                            .process_mouse_motion(delta.0 as f32, delta.1 as f32);
                    }
                }
                Event::AboutToWait => {
                    // Fixed timestep update
                    let now = Instant::now();
                    let frame_time = now - last_frame;
                    last_frame = now;
                    accumulator += frame_time;

                    // Process input
                    {
                        let mut input = client.input.lock();
                        let (dx, dy) = input.take_mouse_delta();
                        if dx != 0.0 || dy != 0.0 {
                            let mut rot = client.world.player_rot.lock();
                            rot.0 += dx; // yaw
                            rot.1 = (rot.1 - dy)
                                .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2); // pitch
                        }
                        input.end_frame();
                    }

                    // Fixed timestep simulation
                    while accumulator >= fixed_dt {
                        // Fixed update (20 TPS)
                        client.fixed_update(fixed_dt);
                        accumulator -= fixed_dt;
                    }

                    // Render
                    client.render_frame_with_renderer(&mut renderer);

                    if let Some(w) = &client.window {
                        w.request_redraw();
                    }
                }
                _ => {}
            }
        })?;

        Ok(())
    }

    /// Fixed timestep update (20 TPS).
    fn fixed_update(&self, dt: Duration) {
        let dt_secs = dt.as_secs_f32();

        // Process input actions
        let input = self.input.lock();

        // Movement
        let mut pos = self.world.player_pos.lock();
        let yaw = self.world.player_rot.lock().0;
        let mut velocity = Vec3::new(0.0, 0.0, 0.0);
        let speed = 4.317; // Blocks per second (walking)
        let sprint_multiplier = if input.is_pressed(Action::Sprint) {
            1.3
        } else {
            1.0
        };

        let forward = Vec3::new(yaw.sin(), 0.0, -yaw.cos()).normalize();
        let right = Vec3::new(yaw.cos(), 0.0, yaw.sin()).normalize();

        if input.is_pressed(Action::MoveForward) {
            velocity += forward;
        }
        if input.is_pressed(Action::MoveBackward) {
            velocity -= forward;
        }
        if input.is_pressed(Action::MoveLeft) {
            velocity -= right;
        }
        if input.is_pressed(Action::MoveRight) {
            velocity += right;
        }
        if input.is_pressed(Action::Jump) && pos.y == pos.y.floor() {
            velocity.y = 6.0; // Jump velocity
        }

        if velocity.length() > 0.0 {
            velocity = velocity.normalize() * speed * sprint_multiplier;
            pos.x += velocity.x * dt_secs;
            pos.z += velocity.z * dt_secs;
            pos.y += velocity.y * dt_secs;
        }

        // Simple gravity
        if pos.y > 0.0 {
            pos.y -= 0.08 * dt_secs; // Gravity
            if pos.y < 0.0 {
                pos.y = 0.0;
            }
        }
    }
}

/// Client errors.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("window error: {0}")]
    Window(#[from] winit::error::OsError),
    #[error("event loop error: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),
    #[error("render error: {0}")]
    Render(#[from] mclre_render::RendererError),
    #[error("channel error: {0}")]
    Channel(#[from] crossbeam_channel::SendError<RenderCommand>),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_world_creation() {
        let world = ClientWorld::new();
        assert_eq!(world.player_pos(), Vec3::new(0.0, 64.0, 0.0));
    }

    #[test]
    fn client_config_default() {
        let config = ClientConfig::default();
        assert_eq!(config.width, 1280);
        assert_eq!(config.height, 720);
    }
}
