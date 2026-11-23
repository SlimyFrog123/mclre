//! QUIC-based LAN transport for multiplayer.
//!
//! Uses quinn (QUIC implementation) for reliable, ordered, encrypted communication
//! over UDP. Suitable for LAN multiplayer with automatic NAT traversal.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use bytes::BytesMut;
use quinn::{ClientConfig, Connection, Endpoint, ServerConfig, RecvStream, SendStream};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rcgen::{Certificate, CertificateParams, DistinguishedName, DnType, KeyPair, SanType, SignatureAlgorithm};
use thiserror::Error;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info, trace, warn};

use super::{BytesMut, Transport, TransportError};

/// Errors specific to QUIC transport.
#[derive(Debug, Error)]
pub enum QuicTransportError {
    #[error("QUIC connection error: {0}")]
    Connection(#[from] quinn::ConnectionError),

    #[error("QUIC endpoint error: {0}")]
    Endpoint(quinn::EndpointError),

    #[error("Certificate generation error: {0}")]
    Certificate(#[from] rcgen::Error),

    #[error("Rustls error: {0}")]
    Rustls(#[from] rustls::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Connection not established")]
    NotConnected,

    #[error("Stream error: {0}")]
    Stream(String),

    #[error("Transport closed")]
    Closed,
}

impl From<QuicTransportError> for super::TransportError {
    fn from(err: QuicTransportError) -> Self {
        match err {
            QuicTransportError::Io(e) => TransportError::Io(e),
            QuicTransportError::Connection(e) => TransportError::RecvFailed(e.to_string()),
            QuicTransportError::Endpoint(e) => TransportError::RecvFailed(e.to_string()),
            QuicTransportError::Stream(e) => TransportError::RecvFailed(e),
            QuicTransportError::Closed => TransportError::Closed,
            e => TransportError::RecvFailed(e.to_string()),
        }
    }
}

/// QUIC transport for LAN multiplayer.
/// Wraps a QUIC connection with packet send/receive channels.
pub struct QuicTransport {
    connection: Arc<Connection>,
    send_tx: mpsc::UnboundedSender<BytesMut>,
    recv_rx: Arc<Mutex<mpsc::UnboundedReceiver<BytesMut>>>,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
    closed: Arc<Mutex<bool>>,
}

impl QuicTransport {
    /// Creates a new QUIC transport from an established connection.
    fn new(
        connection: Connection,
        local_addr: SocketAddr,
        remote_addr: SocketAddr,
    ) -> (Self, mpsc::UnboundedReceiver<BytesMut>) {
        let (send_tx, send_rx) = mpsc::unbounded_channel::<BytesMut>();
        let (recv_tx, recv_rx) = mpsc::unbounded_channel::<BytesMut>();

        let transport = Self {
            connection: Arc::new(connection),
            send_tx,
            recv_rx: Arc::new(Mutex::new(recv_rx)),
            local_addr,
            remote_addr,
            closed: Arc::new(Mutex::new(false)),
        };

        // Spawn task to handle outgoing packets
        let conn = transport.connection.clone();
        let closed = transport.closed.clone();
        let send_tx = transport.send_tx.clone();
        tokio::spawn(async move {
            Self::handle_outgoing(conn, closed, send_rx, send_tx).await;
        });

        // Spawn task to handle incoming packets
        let conn = transport.connection.clone();
        let closed = transport.closed.clone();
        tokio::spawn(async move {
            Self::handle_incoming(conn, closed, recv_tx).await;
        });

        (transport, send_rx)
    }

    /// Handles outgoing packets from the send channel.
    async fn handle_outgoing(
        connection: Arc<Connection>,
        closed: Arc<Mutex<bool>>,
        mut send_rx: mpsc::UnboundedReceiver<BytesMut>,
        _send_tx: mpsc::UnboundedSender<BytesMut>,
    ) {
        let mut send_stream = match connection.open_uni().await {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to open outgoing stream: {}", e);
                return;
            }
        };

        while let Some(buf) = send_rx.recv().await {
            if *closed.lock().await {
                break;
            }

            if let Err(e) = send_stream.write_all(&buf).await {
                error!("Failed to write to QUIC stream: {}", e);
                break;
            }

            if let Err(e) = send_stream.finish() {
                error!("Failed to finish QUIC stream: {}", e);
                break;
            }

            // Open a new stream for the next packet
            match connection.open_uni().await {
                Ok(s) => send_stream = s,
                Err(e) => {
                    error!("Failed to open new outgoing stream: {}", e);
                    break;
                }
            }
        }
    }

    /// Handles incoming packets from QUIC streams.
    async fn handle_incoming(
        connection: Arc<Connection>,
        closed: Arc<Mutex<bool>>,
        recv_tx: mpsc::UnboundedSender<BytesMut>,
    ) {
        while !*closed.lock().await {
            let stream = match connection.accept_uni().await {
                Ok(s) => s,
                Err(e) => {
                    if *closed.lock().await {
                        break;
                    }
                    warn!("Failed to accept incoming stream: {}", e);
                    continue;
                }
            };

            let recv_tx = recv_tx.clone();
            tokio::spawn(async move {
                match stream.read_to_end(1024 * 1024).await {
                    Ok(buf) => {
                        let bytes = BytesMut::from(buf.as_slice());
                        if recv_tx.send(bytes).is_err() {
                            debug!("Receiver dropped, stopping incoming handler");
                        }
                    }
                    Err(e) => {
                        if !matches!(e, quinn::ReadError::ConnectionLost(_)) {
                            error!("Failed to read from QUIC stream: {}", e);
                        }
                    }
                };
            });
        }
    }

    /// Returns the local socket address.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Returns the remote socket address.
    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }
}

impl Transport for QuicTransport {
    fn send(&self, buf: BytesMut) -> Result<(), TransportError> {
        self.send_tx.send(buf)
            .map_err(|e| TransportError::SendFailed(e.to_string()))?;
        Ok(())
    }

    fn try_recv(&self) -> Result<Option<BytesMut>, TransportError> {
        let mut rx = self.recv_rx.blocking_lock();
        match rx.try_recv() {
            Ok(buf) => {
                trace!("Received QUIC packet of {} bytes", buf.len());
                Ok(Some(buf))
            }
            Err(mpsc::error::TryRecvError::Empty) => Ok(None),
            Err(mpsc::error::TryRecvError::Disconnected) => {
                Err(TransportError::Closed)
            }
        }
    }

    fn is_connected(&self) -> bool {
        self.connection.closed().is_none() && !*self.closed.blocking_lock()
    }

    fn close(&self) -> Result<(), TransportError> {
        *self.closed.blocking_lock() = true;
        self.connection.close(0u32.into(), b"");
        debug!("QUIC transport closed");
        Ok(())
    }
}

/// Configuration for QUIC endpoints.
#[derive(Debug, Clone)]
pub struct QuicConfig {
    /// Bind address for the endpoint.
    pub bind_addr: SocketAddr,
    /// Server name for certificate verification (client only).
    pub server_name: Option<String>,
    /// Maximum idle timeout.
    pub max_idle_timeout: Duration,
    /// Keep-alive interval.
    pub keep_alive_interval: Duration,
}

impl Default for QuicConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:25565".parse().unwrap(),
            server_name: None,
            max_idle_timeout: Duration::from_secs(30),
            keep_alive_interval: Duration::from_secs(10),
        }
    }
}

/// Generates a self-signed certificate for QUIC.
pub fn generate_self_signed_cert() -> Result<(Certificate, PrivateKeyDer<'static>), rcgen::Error> {
    let mut params = CertificateParams::new(vec!["localhost".to_string()])?;
    params.not_before = std::time::SystemTime::now();
    params.not_after = params.not_before + std::time::Duration::from_secs(3650 * 24 * 60 * 60); // 10 years
    
    let key_pair = KeyPair::generate_for(&SignatureAlgorithm::ED25519)?;
    params.key_pair = Some(key_pair);
    
    let cert = Certificate::from_params(params)?;
    let key_pair = cert.get_key_pair().clone();
    let private_key = PrivateKeyDer::try_from(key_pair.serialize_der())?;
    
    Ok((cert, private_key))
}

/// Creates a QUIC client endpoint configuration.
pub fn make_client_config(server_name: Option<String>) -> Result<ClientConfig, QuicTransportError> {
    let (cert, private_key) = generate_self_signed_cert()?;
    
    let mut roots = rustls::RootCertStore::empty();
    roots.add(cert.into())?;
    
    let client_crypto = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    
    let mut client_config = ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(client_crypto)
            .map_err(|e| QuicTransportError::Rustls(e))?
    ));
    let mut transport_config = quinn::TransportConfig::default();
    transport_config.max_idle_timeout(Some(std::time::Duration::from_secs(30).try_into().unwrap()));
    transport_config.keep_alive_interval(Some(Duration::from_secs(10)));
    client_config.transport_config(Arc::new(transport_config));
    
    Ok(client_config)
}

/// Creates a QUIC server endpoint configuration.
pub fn make_server_config() -> Result<ServerConfig, QuicTransportError> {
    let (cert, private_key) = generate_self_signed_cert()?;
    
    let cert_der = cert.serialize_der()?;
    let cert = CertificateDer::from(cert_der);
    
    let server_crypto = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], private_key)?;
    
    let mut server_config = ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)
            .map_err(|e| QuicTransportError::Rustls(e))?
    ));
    let mut transport_config = quinn::TransportConfig::default();
    transport_config.max_idle_timeout(Some(std::time::Duration::from_secs(30).try_into().unwrap()));
    transport_config.keep_alive_interval(Some(Duration::from_secs(10)));
    server_config.transport_config(Arc::new(transport_config));
    
    Ok(server_config)
}

/// QUIC server transport for accepting multiple client connections.
pub struct QuicServer {
    endpoint: Endpoint,
    connections: Arc<Mutex<HashMap<SocketAddr, Arc<Connection>>>>,
    closed: Arc<Mutex<bool>>,
}

impl QuicServer {
    /// Creates a new QUIC server.
    pub async fn new(config: QuicConfig) -> Result<Self, QuicTransportError> {
        let server_config = make_server_config()?;
        let endpoint = Endpoint::server(server_config, config.bind_addr)?;
        
        Ok(Self {
            endpoint,
            connections: Arc::new(Mutex::new(HashMap::new())),
            closed: Arc::new(Mutex::new(false)),
        })
    }

    /// Returns the local address the server is bound to.
    pub fn local_addr(&self) -> SocketAddr {
        self.endpoint.local_addr().unwrap()
    }

    /// Accepts a new client connection.
    pub async fn accept(&self) -> Result<QuicTransport, QuicTransportError> {
        let connection = self.endpoint.accept().await
            .ok_or(QuicTransportError::NotConnected)?;
        
        let conn = connection.await?;
        let remote_addr = conn.remote_address();
        let local_addr = conn.local_address().unwrap_or(self.endpoint.local_addr().unwrap());
        
        // Store connection
        self.connections.lock().await.insert(remote_addr, Arc::new(conn.clone()));
        
        let (transport, _send_rx) = QuicTransport::new(conn, local_addr, remote_addr);
        Ok(transport)
    }

    /// Closes the server and all connections.
    pub async fn close(&self) {
        *self.closed.lock().await = true;
        
        let conns = self.connections.lock().await;
        for (_, conn) in conns.iter() {
            conn.close(0u32.into(), b"server shutting down");
        }
        
        self.endpoint.close(0u32.into(), b"");
    }

    /// Returns the number of active connections.
    pub async fn connection_count(&self) -> usize {
        self.connections.lock().await.len()
    }
}

/// QUIC client transport for connecting to a server.
pub struct QuicClient {
    endpoint: Endpoint,
    connection: Option<Arc<Connection>>,
}

impl QuicClient {
    /// Creates a new QUIC client.
    pub async fn new(config: QuicConfig) -> Result<Self, QuicTransportError> {
        let client_config = make_client_config(config.server_name.clone())?;
        let mut endpoint = Endpoint::client(config.bind_addr)?;
        endpoint.set_default_client_config(client_config);
        
        Ok(Self {
            endpoint,
            connection: None,
        })
    }

    /// Connects to a server.
    pub async fn connect(&mut self, server_addr: SocketAddr) -> Result<QuicTransport, QuicTransportError> {
        let connection = self.endpoint.connect(server_addr, "localhost")?.await?;
        let conn = Arc::new(connection);
        let remote_addr = conn.remote_address();
        let local_addr = conn.local_address().unwrap_or(self.endpoint.local_addr().unwrap());
        
        self.connection = Some(conn.clone());
        
        let (transport, _send_rx) = QuicTransport::new((*conn).clone(), local_addr, remote_addr);
        Ok(transport)
    }

    /// Closes the client.
    pub async fn close(&mut self) {
        if let Some(conn) = self.connection.take() {
            conn.close(0u32.into(), b"client disconnecting");
        }
        self.endpoint.close(0u32.into(), b"");
    }
}

/// LAN discovery using mDNS.
pub struct LanDiscovery {
    service_name: String,
    service_type: String,
    port: u16,
    txt_data: HashMap<String, String>,
}

impl LanDiscovery {
    /// Creates a new LAN discovery service.
    pub fn new(service_name: String, port: u16) -> Self {
        Self {
            service_name,
            service_type: "_mclre._udp.local.".to_string(),
            port,
            txt_data: HashMap::new(),
        }
    }

    /// Adds TXT record data.
    pub fn with_txt(mut self, key: String, value: String) -> Self {
        self.txt_data.insert(key, value);
        self
    }

    /// Starts advertising the service on the network.
    pub async fn advertise(&self) -> Result<mdns_sd::ServiceDaemon, mdns_sd::Error> {
        let daemon = mdns_sd::ServiceDaemon::new()?;
        
        let service = mdns_sd::ServiceInfo::new(
            &self.service_type,
            &self.service_name,
            &format!("{}.local.", self.service_name),
            "0.0.0.0".parse().unwrap(),
            self.port,
            Some(&self.txt_data),
        )?;
        
        daemon.register(service)?;
        info!("Advertising LAN game: {} on port {}", self.service_name, self.port);
        
        Ok(daemon)
    }

    /// Discovers services on the network.
    pub async fn discover(service_type: &str, timeout: Duration) -> Result<Vec<mdns_sd::ServiceInfo>, mdns_sd::Error> {
        let daemon = mdns_sd::ServiceDaemon::new()?;
        let receiver = daemon.browse(service_type)?;
        
        let mut services = Vec::new();
        let start = std::time::Instant::now();
        
        while start.elapsed() < timeout {
            if let Ok(event) = receiver.recv_timeout(Duration::from_millis(100)) {
                if let mdns_sd::ServiceEvent::ServiceResolved(info) = event {
                    services.push(info);
                }
            }
        }
        
        Ok(services)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_generate_self_signed_cert() {
        let (cert, private_key) = generate_self_signed_cert().unwrap();
        assert!(!cert.serialize_der().unwrap().is_empty());
        assert!(!private_key.secret_der().is_empty());
    }

    #[test]
    fn test_quic_config_default() {
        let config = QuicConfig::default();
        assert_eq!(config.bind_addr.port(), 25565);
        assert_eq!(config.max_idle_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_lan_discovery_new() {
        let discovery = LanDiscovery::new("Test Game".to_string(), 25565);
        assert_eq!(discovery.port, 25565);
        assert_eq!(discovery.service_name, "Test Game");
    }
}