//! Client library: ClientWorld mirror, prediction, interpolation, local players, UI state machine.

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender};
use mclre_core::Vec3;
use mclre_input::{Action, InputManager};
use mclre_world::{ChunkColumn, ChunkPos, WorldSizePreset};
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
    /// Initial player position.
    pub spawn_pos: Vec3<f32>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            title: "MCLRE".to_string(),
            width: 1280,
            height: 720,
            target_frame_time: Duration::from_millis(50), // 20 TPS
            world_preset: WorldSizePreset::Infinite,
            spawn_pos: Vec3::new(0.0, 64.0, 0.0),
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
    input: Mutex<InputManager<'static>>,
    render_sender: Sender<RenderCommand>,
    render_receiver: Receiver<RenderEvent>,
    window: Arc<winit::window::Window>,
}

impl Client {
    /// Creates a new client.
    pub fn new(config: ClientConfig) -> Result<Self, ClientError> {
        let event_loop = EventLoop::new()?;
        let window = Arc::new(
            event_loop.create_window(
                winit::window::WindowAttributes::default()
                    .with_title(&config.title)
                    .with_inner_size(winit::dpi::LogicalSize::new(config.width, config.height)),
            )?,
        );

        let (render_sender, render_receiver) = crossbeam_channel::bounded(64);
        let (event_sender, event_receiver) = crossbeam_channel::bounded(64);

        let world = Arc::new(ClientWorld::new());
        world.set_player_pos(config.spawn_pos);

        let input = Mutex::new(InputManager::new());
        input.lock().set_mouse_captured(true);
        window
            .set_cursor_grab(winit::window::CursorGrabMode::Confined)
            .ok();
        window.set_cursor_visible(false);

        let render_sender_clone = render_sender.clone();
        let world_clone = world.clone();
        let window_for_renderer = window.clone();

        // Spawn render thread
        std::thread::spawn(move || {
            Self::render_thread(
                window_for_renderer,
                render_receiver,
                event_sender,
                world_clone,
            );
        });

        Ok(Self {
            config,
            world,
            input: Mutex::new(InputManager::new()),
            render_sender,
            render_receiver: event_receiver,
            window,
        })
    }

    /// The render thread function.
    fn render_thread(
        window: Arc<winit::window::Window>,
        receiver: Receiver<RenderCommand>,
        sender: Sender<RenderEvent>,
        world: Arc<ClientWorld>,
    ) {
        // Create renderer (blocking on async)
        let renderer = pollster::block_on(mclre_render::Renderer::new(window.clone()))
            .expect("Failed to create renderer");

        let mut renderer = renderer;
        let mut dirty_uploads: HashMap<
            mclre_world::ChunkPos,
            mclre_render::resources::GpuChunkSection,
        > = HashMap::new();

        loop {
            match receiver.recv_timeout(Duration::from_millis(16)) {
                Ok(cmd) => match cmd {
                    RenderCommand::UploadChunk {
                        chunk_pos,
                        sections,
                    } => {
                        // Upload all chunk sections to GPU
                        for (section_index, section) in sections.iter().enumerate() {
                            if section.is_empty() {
                                continue;
                            }
                            let gpu_chunk = mclre_render::resources::GpuChunkSection::from_section(
                                renderer.device(),
                                renderer.queue(),
                                section,
                                chunk_pos.into(),
                            );
                            dirty_uploads.insert(chunk_pos, gpu_chunk);
                        }
                    }
                    RenderCommand::RemoveChunk(pos) => {
                        dirty_uploads.remove(&pos);
                    }
                    RenderCommand::UpdateView(_view) => {
                        // View is passed to render() call
                    }
                    RenderCommand::Shutdown => {
                        break;
                    }
                },
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    // No commands, render a frame
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    break;
                }
            }

            // Get current view from world
            let view = world.create_view(renderer.width() as f32 / renderer.height() as f32);

            // Get chunks to render
            let chunks: Vec<_> = dirty_uploads.values().cloned().collect();

            // Render frame
            if let Err(e) = renderer.render(&view, &chunks) {
                let _ = sender.send(RenderEvent::Error(e.to_string()));
            }

            let _ = sender.send(RenderEvent::FrameComplete);
        }

        let _ = sender.send(RenderEvent::FrameComplete);
    }

    /// Runs the main game loop.
    pub fn run(self) -> Result<(), ClientError> {
        let event_loop = EventLoop::new()?;
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
                        // TODO: Handle resize
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
                            client.window.set_cursor_visible(true);
                            client
                                .window
                                .set_cursor_grab(winit::window::CursorGrabMode::None)
                                .ok();
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
                    client.render_frame();

                    client.window.request_redraw();
                }
                _ => {}
            }
        })?;

        Ok(())
    }

    /// Fixed timestep update (20 TPS).
    fn fixed_update(&self, _dt: Duration) {
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
            pos.x += velocity.x * 0.05; // 20 TPS = 0.05s per tick
            pos.z += velocity.z * 0.05;
            pos.y += velocity.y * 0.05;
        }

        // Simple gravity
        if pos.y > 0.0 {
            pos.y -= 0.08 * 0.05; // Gravity
            if pos.y < 0.0 {
                pos.y = 0.0;
            }
        }
    }

    /// Renders a frame.
    fn render_frame(&self) {
        // Take dirty chunks and upload
        let dirty_chunks = self.world.take_dirty_chunks();
        for pos in dirty_chunks {
            if let Some(chunk) = self.world.get_chunk(pos) {
                // Upload chunk sections
                let sections: Vec<_> = chunk.sections().iter().cloned().collect();
                let _ = self.render_sender.send(RenderCommand::UploadChunk {
                    chunk_pos: pos,
                    sections,
                });
            }
        }

        // Get view
        let view = self
            .world
            .create_view(self.config.width as f32 / self.config.height as f32);

        let _ = self.render_sender.send(RenderCommand::UpdateView(view));

        // Wait for frame complete
        let _ = self.render_receiver.recv();
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
