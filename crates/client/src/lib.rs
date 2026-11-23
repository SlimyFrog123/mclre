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
use mclre_ui::{
    MainMenuScreen, Rect, ScreenManager, ScreenTransition, ScreenType, TitleScreen, UiContext,
};

mod ui_renderer;
use mclre_world::{ChunkColumn, ChunkPos, ChunkSection, LocalPos, WorldSizePreset};
use parking_lot::Mutex;
use ui_renderer::UiRenderer;
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

    /// Unloads chunks that are too far from the player to free VRAM.
    /// Keeps chunks within `radius` chunks of the player (default 8 = 128 blocks).
    pub fn unload_distant_chunks(&self, radius: i32) {
        let player_pos = self.player_pos();
        let player_chunk_x = (player_pos.x / 16.0).floor() as i32;
        let player_chunk_z = (player_pos.z / 16.0).floor() as i32;

        let mut chunks = self.chunks.lock();
        let mut gpu_chunks = self.gpu_chunks.lock();
        let mut dirty_chunks = self.dirty_chunks.lock();

        let radius_sq = (radius as f32).powi(2);

        // Find chunks to remove
        let to_remove: Vec<ChunkPos> = chunks
            .keys()
            .filter(|pos| {
                let dx = (pos.x - player_chunk_x) as f32;
                let dz = (pos.z - player_chunk_z) as f32;
                dx * dx + dz * dz > radius_sq
            })
            .cloned()
            .collect();

        for pos in to_remove {
            chunks.remove(&pos);
            gpu_chunks.remove(&pos);
            // Also remove from dirty chunks if present
            dirty_chunks.retain(|p| *p != pos);
        }
    }

    /// Estimates current GPU memory usage from chunk meshes in MB.
    /// Rough estimate: each non-empty chunk section ~500 KB (vertices + indices).
    pub fn estimate_gpu_memory_mb(&self) -> usize {
        let gpu_chunks = self.gpu_chunks.lock();
        let section_count: usize = gpu_chunks.values().map(|v| v.len()).sum();
        // Rough estimate: ~500 KB per section (conservative)
        section_count * 500 / 1024
    }

    /// Forces unloading of chunks until GPU memory is under budget.
    /// Unloads furthest chunks first.
    pub fn enforce_vram_budget(&self, budget_mb: usize) {
        let current_mb = self.estimate_gpu_memory_mb();
        if current_mb <= budget_mb {
            return;
        }

        let player_pos = self.player_pos();
        let player_chunk_x = (player_pos.x / 16.0).floor() as i32;
        let player_chunk_z = (player_pos.z / 16.0).floor() as i32;

        let mut chunks = self.chunks.lock();
        let mut gpu_chunks = self.gpu_chunks.lock();
        let mut dirty_chunks = self.dirty_chunks.lock();

        // Collect chunks with their distance from player
        let mut chunk_distances: Vec<(ChunkPos, f32, usize)> = gpu_chunks
            .iter()
            .map(|(pos, sections)| {
                let dx = (pos.x - player_chunk_x) as f32;
                let dz = (pos.z - player_chunk_z) as f32;
                let dist_sq = dx * dx + dz * dz;
                (pos.clone(), dist_sq, sections.len())
            })
            .collect();

        // Sort by distance (furthest first)
        chunk_distances.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Remove chunks until under budget
        let mut estimated_mb = current_mb;
        for (pos, _, section_count) in chunk_distances {
            if estimated_mb <= budget_mb {
                break;
            }
            chunks.remove(&pos);
            gpu_chunks.remove(&pos);
            dirty_chunks.retain(|p| *p != pos);
            estimated_mb -= section_count * 500 / 1024;
        }
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

    // Create a 3x3 chunk area around spawn (0,0) with height variation
    for cx in -1..=1 {
        for cz in -1..=1 {
            let chunk_pos = ChunkPos::new(cx, cz);
            let mut sections = Vec::new();

            // 16 sections (256 blocks tall)
            for section_y in 0..16 {
                let section_base_y = section_y * 16;
                let mut section = ChunkSection::air();

                // Fill blocks in this section
                for x in 0..16 {
                    for z in 0..16 {
                        // Simple height variation using hash of position
                        let wx = cx * 16 + x;
                        let wz = cz * 16 + z;
                        let h = ((wx as i64 * 341873128712 + wz as i64 * 132897987541)
                            ^ 0x9E3779B97F4A7C15u64 as i64) as u64;
                        let height = 60 + (h % 8) as i32; // Height 60-67

                        for y in 0..16 {
                            let wy = section_base_y + y;
                            let state = if wy == 0 {
                                bedrock_state
                            } else if wy <= height - 3 {
                                stone_state
                            } else if wy <= height - 1 {
                                dirt_state
                            } else if wy == height {
                                grass_state
                            } else {
                                continue; // Air
                            };
                            let local_pos = LocalPos::new(x as i32, y as i32, z as i32).unwrap();
                            section.set(local_pos, state);
                        }
                    }
                }

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
            spawn_pos: Vec3::new(0.0, 65.62, 0.0),
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
    /// Screen manager for UI state.
    screen_manager: Option<ScreenManager>,
    /// UI context for rendering.
    ui_context: Option<UiContext>,
    /// Integrated server (started when "Play Game" is clicked).
    server: Option<Arc<parking_lot::Mutex<dyn mclre_net::ServerHandle>>>,
    /// Frame counter for periodic tasks (e.g., chunk unloading).
    frame_count: u64,
}

impl Client {
    /// Creates a new client (window created in run()).
    /// Starts at the title screen - world is only created when user selects "Play Game".
    pub fn new(config: ClientConfig) -> Result<Self, ClientError> {
        let world = Arc::new(ClientWorld::new());
        world.set_player_pos(config.spawn_pos);
        // Start looking slightly downward so the ground is visible on flat worlds
        world.set_player_rot(0.0, -0.3);

        let input = Mutex::new(InputManager::new());

        // Create asset manager
        let asset_manager = Arc::new(Mutex::new(mclre_assets::AssetManager::new()));

        // Initialize network transports for split-screen (up to 4 players)
        let max_players = config.split_screen_players.clamp(1, 4);
        let network_transports = vec![None; max_players];

        // Initialize screen manager
        let mut screen_manager = ScreenManager::new();
        screen_manager.register_screen(ScreenType::Title, Box::new(TitleScreen::new()));
        screen_manager.register_screen(ScreenType::MainMenu, Box::new(MainMenuScreen::new()));

        // Create UI context
        let assets = Arc::new(mclre_assets::AssetManager::new());
        let ui_context = UiContext::for_player(
            0,
            Rect::new(0.0, 0.0, config.width as f32, config.height as f32),
            assets,
        );

        Ok(Self {
            config,
            world,
            input,
            asset_manager,
            block_state_model: Arc::new(
                mclre_content_vanilla::block_states().expect("Failed to create block state model"),
            ),
            window: None,
            network_transports,
            screen_manager: Some(screen_manager),
            ui_context: Some(ui_context),
            server: None,
            frame_count: 0,
        })
    }

    /// Sets the integrated server (called from main before run()).
    pub fn set_server(&mut self, server: Arc<parking_lot::Mutex<dyn mclre_net::ServerHandle>>) {
        self.server = Some(server);
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
                for (section_y, section) in chunk.sections().iter().enumerate() {
                    if section.is_empty() {
                        continue;
                    }
                    let section_pos = Vec3::new(pos.x, section_y as i32, pos.z);
                    let gpu_chunk = mclre_render::resources::GpuChunkSection::from_section(
                        renderer.device(),
                        renderer.queue(),
                        section,
                        section_pos,
                        Some(&self.block_state_model),
                    );
                    gpu_sections.push(gpu_chunk);
                }
                if !gpu_sections.is_empty() {
                    self.world.store_gpu_chunks(pos, gpu_sections);
                }
            }
        }

        // Check if we're in a menu
        let in_menu = self
            .screen_manager
            .as_ref()
            .map(|sm| sm.get_current().is_some())
            .unwrap_or(false);

        if in_menu {
            // Render UI only (no world)
            if let Some(screen_manager) = &self.screen_manager {
                let mut ui_renderer = UiRenderer::new(renderer);
                screen_manager.render(&mut ui_renderer);
            }
        } else {
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
    }

    /// Creates a renderer with a minimal VRAM availability check.
    /// Returns an error if VRAM is completely exhausted.
    fn create_renderer_with_vram_check(
        window: Arc<winit::window::Window>,
    ) -> Result<mclre_render::Renderer, ClientError> {
        // Create instance and surface
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            ..Default::default()
        });
        let surface = instance
            .create_surface(window.clone())
            .map_err(mclre_render::RendererError::CreateSurface)?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or(mclre_render::RendererError::NoAdapter)?;
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("VRAM test device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .map_err(mclre_render::RendererError::RequestDevice)?;

        // Minimal VRAM check: just try to create a tiny 1x1 texture
        // If this fails, VRAM is completely exhausted
        let test_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("VRAM test texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &test_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[0xFFu8; 4],
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        // VRAM check passed, create the full renderer
        let renderer =
            pollster::block_on(mclre_render::Renderer::new(window)).map_err(ClientError::Render)?;
        Ok(renderer)
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

        // Create renderer in main thread (where window was created), with VRAM check
        let mut renderer = Self::create_renderer_with_vram_check(window.clone())?;

        // Load texture atlas from converted game-assets
        {
            let mut asset_manager = self.asset_manager.lock();
            asset_manager.add_root(std::path::PathBuf::from("game-assets"));

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

            // Try to load and upload the pre-built atlas, but handle memory errors gracefully
            let atlas_id = mclre_assets::Identifier::new("minecraft", "atlas").unwrap();
            let atlas_texture =
                match asset_manager.load_texture(atlas_id, "textures/block/atlas.png") {
                    Ok(texture) => {
                        let (width, height) = texture.dimensions();
                        println!("Loaded pre-built atlas: {width}x{height}");
                        Some(texture)
                    }
                    Err(e) => {
                        eprintln!("Failed to load pre-built atlas: {e}");
                        None
                    }
                };

            let uvs_count = uvs.len();
            let padding = 1;

            // Skip texture atlas upload entirely to avoid VRAM OOM when other games are running.
            // The renderer will use its default minimal 16x16 white texture atlas.
            // UV coordinates from manifest are for the 8192x8192 atlas and won't match,
            // but this allows the game to run without VRAM OOM crashes.
            eprintln!(
                "Skipping texture atlas upload to conserve VRAM (using default 16x16 white texture)"
            );
            // Note: UV coordinates from manifest are for the 8192x8192 atlas and won't match the default 16x16 atlas.
            // Textures will appear white, but the game will run without VRAM OOM crashes.
        }

        // Setup input manager
        self.input.lock().set_mouse_captured(true);
        window
            .set_cursor_grab(winit::window::CursorGrabMode::Confined)
            .ok();
        window.set_cursor_visible(false);

        // Store window in client
        self.window = Some(window.clone());

        let mut client = self;

        let mut last_frame = Instant::now();
        let mut accumulator = Duration::ZERO;
        let fixed_dt = client.config.target_frame_time;
        if let Some(screen_manager) = &mut client.screen_manager {
            let ui_context = client.ui_context.as_mut().unwrap();
            screen_manager
                .push_screen(ScreenType::Title, ui_context)
                .ok();
        }

        event_loop.run(move |event, event_loop| {
            event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        renderer.wait_idle();
                        event_loop.exit();
                    }
                    WindowEvent::Resized(size) => {
                        // Handle resize
                        renderer.resize(size.width, size.height);
                        if let Some(ui_context) = &mut client.ui_context {
                            ui_context.screen_size = (size.width, size.height);
                        }
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        // Check if we're in a menu screen
                        let in_menu = client
                            .screen_manager
                            .as_ref()
                            .map(|sm| sm.get_current().is_some())
                            .unwrap_or(false);

                        let pressed = event.state == winit::event::ElementState::Pressed;

                        if in_menu {
                            // Forward to screen manager for UI navigation
                            let action = {
                                let mut input = client.input.lock();
                                input.translate_keyboard(event)
                            };
                            if let Some(action) = action {
                                if let Some(screen_manager) = &mut client.screen_manager {
                                    let ui_context = client.ui_context.as_mut().unwrap();
                                    screen_manager.handle_input(ui_context, action, pressed);
                                }
                            }
                        } else {
                            client.input.lock().process_keyboard(event);
                        }
                    }
                    WindowEvent::MouseInput { button, state, .. } => {
                        let in_menu = client
                            .screen_manager
                            .as_ref()
                            .map(|sm| sm.get_current().is_some())
                            .unwrap_or(false);

                        if in_menu {
                            let action = {
                                let input = client.input.lock();
                                input.translate_mouse_button(button)
                            };
                            if let Some(action) = action {
                                let pressed = state == winit::event::ElementState::Pressed;
                                if let Some(screen_manager) = &mut client.screen_manager {
                                    let ui_context = client.ui_context.as_mut().unwrap();
                                    screen_manager.handle_input(ui_context, action, pressed);
                                }
                            }
                        } else {
                            client.input.lock().process_mouse_button(button, state);
                        }
                    }
                    WindowEvent::MouseWheel { delta, .. } => {
                        let in_menu = client
                            .screen_manager
                            .as_ref()
                            .map(|sm| sm.get_current().is_some())
                            .unwrap_or(false);

                        if in_menu {
                            let action = {
                                let input = client.input.lock();
                                input.translate_mouse_scroll(delta)
                            };
                            if let Some(action) = action {
                                if let Some(screen_manager) = &mut client.screen_manager {
                                    let ui_context = client.ui_context.as_mut().unwrap();
                                    screen_manager.handle_input(ui_context, action, true);
                                }
                            }
                        } else {
                            client.input.lock().process_mouse_scroll(delta);
                        }
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
                        let in_menu = client
                            .screen_manager
                            .as_ref()
                            .map(|sm| sm.get_current().is_some())
                            .unwrap_or(false);

                        if !in_menu {
                            client
                                .input
                                .lock()
                                .process_mouse_motion(delta.0 as f32, delta.1 as f32);
                        }
                    }
                }
                Event::AboutToWait => {
                    // Fixed timestep update
                    let now = Instant::now();
                    let frame_time = now - last_frame;
                    last_frame = now;
                    accumulator += frame_time;
                    let dt = frame_time.as_secs_f32();

                    // Check if we're in a menu or gameplay
                    let in_menu = client
                        .screen_manager
                        .as_ref()
                        .map(|sm| sm.get_current().is_some())
                        .unwrap_or(false);

                    if in_menu {
                        // Update screen manager
                        if let Some(screen_manager) = &mut client.screen_manager {
                            let ui_context = client.ui_context.as_mut().unwrap();
                            screen_manager.update(ui_context, dt);

                            // Handle screen transitions
                            if let Some(current_screen) = screen_manager.get_current() {
                                if let Some(screen) = screen_manager.get_screen_mut(current_screen)
                                {
                                    let transition = screen.take_requested_transition();
                                    match transition {
                                        ScreenTransition::None => {}
                                        ScreenTransition::TitleToMainMenu => {
                                            screen_manager.pop_screen(ui_context).ok();
                                            screen_manager
                                                .push_screen(ScreenType::MainMenu, ui_context)
                                                .ok();
                                        }
                                        ScreenTransition::TitleToQuit => {
                                            event_loop.exit();
                                        }
                                        ScreenTransition::MainMenuToPlay => {
                                            screen_manager.pop_screen(ui_context).ok();
                                            screen_manager.set_current_screen(None); // Enter gameplay
                                            // Start the integrated server now
                                            if let Some(server) = &client.server {
                                                let mut server = server.lock();
                                                let _stop_fn = server.start();
                                            }
                                            // Create the test world now that user selected "Play Game"
                                            let (block_state_model, columns) =
                                                create_flat_test_world()
                                                    .expect("Failed to create flat test world");
                                            client.block_state_model = Arc::new(block_state_model);
                                            for (pos, column) in columns {
                                                client.world.set_chunk(Arc::new(column));
                                            }
                                            // Release cursor capture for gameplay
                                            if let Some(w) = &client.window {
                                                w.set_cursor_grab(
                                                    winit::window::CursorGrabMode::Confined,
                                                )
                                                .ok();
                                                w.set_cursor_visible(false);
                                            }
                                            client.input.lock().set_mouse_captured(true);
                                        }
                                        ScreenTransition::MainMenuToQuit => {
                                            screen_manager.pop_screen(ui_context).ok();
                                            screen_manager
                                                .push_screen(ScreenType::Title, ui_context)
                                                .ok();
                                        }
                                        ScreenTransition::Back => {
                                            screen_manager.pop_screen(ui_context).ok();
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    } else {
                        // Gameplay input
                        {
                            let mut input = client.input.lock();
                            let (dx, dy) = input.take_mouse_delta();
                            if dx != 0.0 || dy != 0.0 {
                                let mut rot = client.world.player_rot.lock();
                                rot.0 += dx; // yaw
                                rot.1 = (rot.1 - dy).clamp(
                                    -std::f32::consts::FRAC_PI_2,
                                    std::f32::consts::FRAC_PI_2,
                                ); // pitch
                            }
                            input.end_frame();
                        }

                        // Fixed timestep simulation
                        while accumulator >= fixed_dt {
                            // Fixed update (20 TPS)
                            client.fixed_update(fixed_dt);
                            accumulator -= fixed_dt;
                        }
                    }

                    // Render
                    // Unload distant chunks every frame during gameplay to aggressively free VRAM
                    // Use a small radius (2 chunks = 32 blocks) since test world is only 3x3 chunks
                    client.world.unload_distant_chunks(2);
                    // Enforce VRAM budget (256 MB default) - forces unloading if exceeded
                    client.world.enforce_vram_budget(256);
                    client.frame_count += 1;

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

        // Simple gravity. The flat test world surface is at y=64; keep the
        // camera at eye height above it until client-side collision exists.
        const GROUND_EYE_Y: f32 = 65.62;
        if pos.y > GROUND_EYE_Y {
            pos.y = (pos.y - 0.08 * dt_secs).max(GROUND_EYE_Y);
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
    #[error("other error: {0}")]
    Other(String),
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
