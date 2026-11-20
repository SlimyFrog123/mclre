use std::sync::Arc;
use std::time::Duration;
use crossbeam_channel::{Receiver, Sender, unbounded};
use bytes::BytesMut;
use mclre_net::{Transport, TransportError};
use mclre_protocol::{Encode, ProtocolError};
use mclre_sim::{SimRunner, SimWorld, GameMode};
use mclre_identity::PlayerIdentity;
use mclre_core::Vec3;
use thiserror::Error;
use tracing::{debug, error, info, warn};

/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub world_seed: i64,
    pub max_players: u32,
    pub motd: String,
    pub difficulty: mclre_sim::GameMode,
    pub online_mode: bool,
    pub view_distance: u8,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            world_seed: 0,
            max_players: 20,
            motd: "MCLRE Server".to_string(),
            difficulty: GameMode::Survival,
            online_mode: false,
            view_distance: 10,
        }
    }
}

/// Server errors.
#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),

    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    #[error("Simulation error: {0}")]
    Simulation(#[from] mclre_sim::SimError),

    #[error("Player error: {0}")]
    Player(String),

    #[error("World error: {0}")]
    World(String),

    #[error("Server not running")]
    NotRunning,
}

/// Connection state for a client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Handshake,
    Login,
    Configuration,
    Play,
}

/// Client connection handler.
pub struct ClientConnection {
    pub identity: PlayerIdentity,
    pub transport: Box<dyn Transport + Send + Sync>,
    pub state: ConnectionState,
    pub entity_id: i32,
    pub position: Vec3<f32>,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub gamemode: GameMode,
    pub pending_packets: Vec<BytesMut>,
}

impl ClientConnection {
    /// Create a new client connection.
    pub fn new(identity: PlayerIdentity, transport: Box<dyn Transport + Send + Sync>, entity_id: i32) -> Self {
        Self {
            identity,
            transport,
            state: ConnectionState::Handshake,
            entity_id,
            position: Vec3::new(0.0, 64.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            on_ground: true,
            gamemode: GameMode::Survival,
            pending_packets: Vec::new(),
        }
    }

    /// Send a packet to the client.
    pub fn send_packet<P: Encode>(&mut self, packet: &P) -> Result<(), ServerError> {
        let mut buf = BytesMut::new();
        packet.encode(&mut buf)?;
        self.transport.send(buf)?;
        Ok(())
    }

    /// Try to receive a packet from the client.
    pub fn try_recv_packet(&mut self) -> Result<Option<BytesMut>, ServerError> {
        self.transport.try_recv().map_err(ServerError::Transport)
    }

    /// Check if the connection is active.
    pub fn is_connected(&self) -> bool {
        self.transport.is_connected()
    }
}

/// Server handles the main server loop and client connections.
pub struct Server {
    config: ServerConfig,
    sim_runner: Arc<parking_lot::Mutex<SimRunner>>,
    sim_world: Arc<parking_lot::Mutex<SimWorld>>,
    clients: Arc<parking_lot::Mutex<Vec<ClientConnection>>>,
    next_entity_id: Arc<parking_lot::Mutex<i32>>,
    running: Arc<parking_lot::Mutex<bool>>,
    shutdown_tx: Option<Sender<()>>,
}

impl Server {
    /// Create a new server with the given configuration.
    pub fn new(config: ServerConfig) -> Self {
        let sim_world = Arc::new(parking_lot::Mutex::new(SimWorld::new(config.world_seed)));
        let sim_runner = Arc::new(parking_lot::Mutex::new(SimRunner::new(
            // We need to create a new SimWorld for the runner
            // In practice, the runner and server share the same world
            // For now, we'll create a separate one and sync via the shared sim_world
            SimWorld::new(config.world_seed)
        )));
        
        Self {
            config,
            sim_runner,
            sim_world,
            clients: Arc::new(parking_lot::Mutex::new(Vec::new())),
            next_entity_id: Arc::new(parking_lot::Mutex::new(1)),
            running: Arc::new(parking_lot::Mutex::new(false)),
            shutdown_tx: None,
        }
    }

    /// Start the server (blocks until shutdown).
    pub fn run(&mut self) -> Result<(), ServerError> {
        *self.running.lock() = true;
        
        let (shutdown_tx, shutdown_rx) = unbounded();
        self.shutdown_tx = Some(shutdown_tx);
        
        info!("Starting MCLRE server on seed {}", self.config.world_seed);
        
        // Spawn simulation thread
        let sim_runner = self.sim_runner.clone();
        let sim_handle = std::thread::spawn(move || {
            sim_runner.lock().run();
        });
        
        // Main server loop
        self.server_loop(shutdown_rx)?;
        
        // Stop simulation
        self.sim_runner.lock().stop();
        sim_handle.join().ok();
        
        info!("Server stopped");
        Ok(())
    }

    /// Main server loop.
    fn server_loop(&mut self, shutdown_rx: Receiver<()>) -> Result<(), ServerError> {
        let tick_interval = Duration::from_millis(50); // 20 TPS
        
        loop {
            // Check for shutdown signal
            if shutdown_rx.try_recv().is_ok() {
                break;
            }
            
            let start = std::time::Instant::now();
            
            // Process client connections
            self.process_clients()?;
            
            // Sleep to maintain tick rate
            let elapsed = start.elapsed();
            if elapsed < tick_interval {
                std::thread::sleep(tick_interval - elapsed);
            } else {
                warn!("Server tick took {}ms (budget: 50ms)", elapsed.as_millis());
            }
        }
        
        Ok(())
    }

    /// Process all connected clients.
    fn process_clients(&mut self) -> Result<(), ServerError> {
        // First, collect all packets to process
        let mut packets_to_handle = Vec::new();
        let mut disconnected_clients = Vec::new();
        let mut packets_to_send = Vec::new();
        
        {
            let mut clients = self.clients.lock();
            let mut i = 0;
            
            while i < clients.len() {
                let client = &mut clients[i];
                
                if !client.is_connected() {
                    info!("Client {} disconnected", client.identity.username);
                    disconnected_clients.push(client.identity.clone());
                    clients.remove(i);
                    continue;
                }
                
                // Try to receive packets
                while let Some(buf) = client.try_recv_packet()? {
                    packets_to_handle.push((client.identity.clone(), buf));
                }
                
                // Collect packets to send
                for buf in client.pending_packets.drain(..) {
                    packets_to_send.push(buf);
                }
                
                i += 1;
            }
        } // Lock released here
        
        // Handle received packets
        for (identity, buf) in packets_to_handle {
            // Find and extract the client
            let client = {
                let mut clients = self.clients.lock();
                if let Some(index) = clients.iter().position(|c| c.identity == identity) {
                    Some(clients.remove(index))
                } else {
                    None
                }
            };
            
            if let Some(mut client) = client {
                self.handle_packet(&mut client, buf)?;
                // Put the client back
                self.clients.lock().push(client);
            }
        }
        
        // Send queued packets
        for buf in packets_to_send {
            // In a real implementation, we'd send to specific clients
            // For now, broadcast to all (placeholder)
            let clients = self.clients.lock();
            for client in clients.iter() {
                let _ = client.transport.send(buf.clone());
            }
        }
        
        // Remove disconnected clients from simulation
        for identity in disconnected_clients {
            self.sim_world.lock().remove_player(&identity);
        }
        
        Ok(())
    }

    /// Handle an incoming packet from a client.
    fn handle_packet(&mut self, client: &mut ClientConnection, buf: BytesMut) -> Result<(), ServerError> {
        // TODO: Implement packet handling based on connection state
        // For now, just log
        debug!("Received packet from {}: {} bytes", client.identity.username, buf.len());
        Ok(())
    }

    /// Add a new client connection (for integrated server).
    pub fn add_client(&mut self, identity: PlayerIdentity, transport: Box<dyn Transport + Send + Sync>) -> i32 {
        let entity_id = {
            let mut next_id = self.next_entity_id.lock();
            let id = *next_id;
            *next_id += 1;
            id
        };
        
        let mut client = ClientConnection::new(identity.clone(), transport, entity_id);
        
        // Add to simulation
        self.sim_world.lock().add_player(identity.clone(), entity_id, client.position);
        
        self.clients.lock().push(client);
        
        info!("Client {} joined with entity ID {}", identity.username, entity_id);
        entity_id
    }

    /// Remove a client connection.
    pub fn remove_client(&mut self, identity: &PlayerIdentity) {
        let mut clients = self.clients.lock();
        if let Some(pos) = clients.iter().position(|c| c.identity == *identity) {
            let client = clients.remove(pos);
            self.sim_world.lock().remove_player(identity);
            info!("Client {} left", client.identity.username);
        }
    }

    /// Get the simulation world.
    pub fn sim_world(&self) -> Arc<parking_lot::Mutex<SimWorld>> {
        self.sim_world.clone()
    }

    /// Get the server configuration.
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Shutdown the server.
    pub fn shutdown(&self) {
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(());
        }
    }
}

/// Integrated server for singleplayer (uses in-memory transport).
pub struct IntegratedServer {
    server: Server,
    client_transport: Option<Box<dyn Transport + Send + Sync>>,
}

impl IntegratedServer {
    /// Create a new integrated server.
    pub fn new(config: ServerConfig) -> Self {
        let server = Server::new(config);
        Self {
            server,
            client_transport: None,
        }
    }

    /// Start the integrated server with a client transport.
    pub fn start_with_client(&mut self, client_transport: Box<dyn Transport + Send + Sync>, identity: PlayerIdentity) -> Result<i32, ServerError> {
        self.client_transport = Some(client_transport);
        
        let entity_id = self.server.add_client(identity, self.client_transport.take().unwrap());
        
        // Run the server in the background
        let mut server = std::mem::replace(&mut self.server, Server::new(ServerConfig::default()));
        
        std::thread::spawn(move || {
            let _ = server.run();
        });
        
        Ok(entity_id)
    }

    /// Get the server instance.
    pub fn server(&self) -> &Server {
        &self.server
    }

    /// Get mutable server instance.
    pub fn server_mut(&mut self) -> &mut Server {
        &mut self.server
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mclre_net::InMemoryTransport;
    use mclre_identity::PlayerIdentity;
    use mclre_core::Vec3;

    #[test]
    fn test_server_creation() {
        let config = ServerConfig::default();
        let server = Server::new(config);
        assert_eq!(server.config().max_players, 20);
    }

    #[test]
    fn test_integrated_server_creation() {
        let config = ServerConfig::default();
        let integrated = IntegratedServer::new(config);
        assert_eq!(integrated.server().config().max_players, 20);
    }

    #[test]
    fn test_client_connection() {
        let (client_transport, server_transport) = InMemoryTransport::pair();
        let identity = PlayerIdentity::new(uuid::Uuid::now_v7(), "TestPlayer".to_string());
        
        let mut server = Server::new(ServerConfig::default());
        let entity_id = server.add_client(identity.clone(), Box::new(server_transport));
        
        assert_eq!(entity_id, 1);
        
        // Check player was added to simulation
        let sim_world = server.sim_world();
        {
            let mut sim = sim_world.lock();
            let player = sim.get_player(&identity);
            assert!(player.is_some());
            assert_eq!(player.unwrap().entity_id, 1);
        }
    }

    #[test]
    fn test_multiple_clients() {
        let (_client1_tx, server1_rx) = InMemoryTransport::pair();
        let (_client2_tx, server2_rx) = InMemoryTransport::pair();
        
        let identity1 = PlayerIdentity::new(uuid::Uuid::now_v7(), "Player1".to_string());
        let identity2 = PlayerIdentity::new(uuid::Uuid::now_v7(), "Player2".to_string());
        
        let mut server = Server::new(ServerConfig::default());
        let id1 = server.add_client(identity1.clone(), Box::new(server1_rx));
        let id2 = server.add_client(identity2.clone(), Box::new(server2_rx));
        
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        
        // Both players should be in simulation
        let sim_world = server.sim_world();
        {
            let mut sim = sim_world.lock();
            assert!(sim.get_player(&identity1).is_some());
            assert!(sim.get_player(&identity2).is_some());
        }
    }
}