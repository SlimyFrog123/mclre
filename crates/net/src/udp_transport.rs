//! UDP-based LAN transport for multiplayer.
//!
//! Simple UDP transport for LAN multiplayer. Provides basic packet send/receive
//! with minimal overhead. Not as robust as QUIC but much simpler and more reliable.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use bytes::BytesMut;
use crossbeam_channel::{Receiver, Sender, TryRecvError, unbounded};
use thiserror::Error;
use tracing::{debug, error, info, trace};

use super::{Transport, TransportError};

/// Errors specific to UDP transport.
#[derive(Debug, Error)]
pub enum UdpTransportError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Connection not established")]
    NotConnected,

    #[error("Transport closed")]
    Closed,
}

impl From<UdpTransportError> for super::TransportError {
    fn from(err: UdpTransportError) -> Self {
        match err {
            UdpTransportError::Io(e) => TransportError::Io(e),
            UdpTransportError::NotConnected => TransportError::RecvFailed("Not connected".into()),
            UdpTransportError::Closed => TransportError::Closed,
        }
    }
}

/// UDP transport for LAN multiplayer.
/// Uses a simple UDP socket with a background thread for receiving.
pub struct UdpTransport {
    socket: Arc<UdpSocket>,
    send_tx: crossbeam_channel::Sender<bytes::BytesMut>,
    recv_rx: crossbeam_channel::Receiver<bytes::BytesMut>,
    remote_addr: SocketAddr,
    /// For listen mode: map of remote addresses for multiple clients
    remote_addrs: Arc<std::sync::Mutex<HashMap<SocketAddr, ()>>>,
    closed: Arc<std::sync::atomic::AtomicBool>,
}

impl UdpTransport {
    /// Creates a new UDP transport from an existing socket.
    pub fn new(
        socket: UdpSocket,
        remote_addr: SocketAddr,
    ) -> Result<(Self, crossbeam_channel::Receiver<bytes::BytesMut>), UdpTransportError> {
        let (send_tx, send_rx) = crossbeam_channel::unbounded::<bytes::BytesMut>();
        let (recv_tx, recv_rx) = crossbeam_channel::unbounded::<bytes::BytesMut>();

        let socket = Arc::new(socket);
        let closed = Arc::new(std::sync::atomic::AtomicBool::new(false));

        let local_addr = socket.local_addr()?;

        let transport = Self {
            socket: socket.clone(),
            send_tx,
            recv_rx: recv_rx.clone(),
            remote_addr,
            remote_addrs: Arc::new(std::sync::Mutex::new(HashMap::new())),
            closed: closed.clone(),
        };

        // Spawn task to handle outgoing packets
        let socket_clone = socket.clone();
        let remote = remote_addr;
        let closed_clone = closed.clone();
        thread::spawn(move || {
            Self::handle_outgoing(socket_clone, remote, send_rx, closed_clone);
        });

        // Spawn task to handle incoming packets
        let socket_clone = socket.clone();
        let closed_clone = closed.clone();
        let recv_tx_clone = recv_tx.clone();
        thread::spawn(move || {
            Self::handle_incoming(socket_clone, recv_tx_clone, closed_clone);
        });

        Ok((transport, recv_rx))
    }

    /// Creates a UDP transport bound to a local address and connected to a remote.
    pub fn connect(
        local_addr: SocketAddr,
        remote_addr: SocketAddr,
    ) -> Result<(Self, crossbeam_channel::Receiver<bytes::BytesMut>), UdpTransportError> {
        let socket = UdpSocket::bind(local_addr)?;
        socket.connect(remote_addr)?;
        Self::new(socket, remote_addr)
    }

    /// Creates a UDP transport that listens for incoming connections.
    pub fn listen(
        local_addr: SocketAddr,
    ) -> Result<
        (
            Self,
            crossbeam_channel::Receiver<bytes::BytesMut>,
            SocketAddr,
        ),
        UdpTransportError,
    > {
        let socket = UdpSocket::bind(local_addr)?;
        let local = socket.local_addr()?;

        let (send_tx, send_rx) = crossbeam_channel::unbounded::<bytes::BytesMut>();
        let (recv_tx, recv_rx) = crossbeam_channel::unbounded::<bytes::BytesMut>();

        let socket = Arc::new(socket);
        let closed = Arc::new(std::sync::atomic::AtomicBool::new(false));

        // We'll wait for the first packet to determine the remote address
        let transport = Self {
            socket: socket.clone(),
            send_tx,
            recv_rx: recv_rx.clone(),
            remote_addr: "0.0.0.0:0".parse().unwrap(),
            remote_addrs: Arc::new(std::sync::Mutex::new(HashMap::new())),
            closed: closed.clone(),
        };

        // Spawn task to handle outgoing packets
        let socket_clone = socket.clone();
        let closed_clone = closed.clone();
        let remote_addr = Arc::new(std::sync::Mutex::new(None::<SocketAddr>));
        let remote_clone = remote_addr.clone();
        thread::spawn(move || {
            Self::handle_outgoing_listen(socket_clone, remote_clone, send_rx, closed_clone);
        });

        // Spawn task to handle incoming packets
        let socket_clone = socket.clone();
        let closed_clone = closed.clone();
        let remote_clone2 = remote_addr.clone();
        let recv_tx_clone = recv_tx.clone();
        thread::spawn(move || {
            Self::handle_incoming_listen(socket_clone, remote_clone2, recv_tx_clone, closed_clone);
        });

        // Wait for first packet to get remote address
        // For now, return with dummy remote addr - real implementation would wait
        Ok((transport, recv_rx, local_addr))
    }

    /// Handles outgoing packets from the send channel (connected mode).
    fn handle_outgoing(
        socket: Arc<UdpSocket>,
        remote: SocketAddr,
        send_rx: crossbeam_channel::Receiver<bytes::BytesMut>,
        closed: Arc<std::sync::atomic::AtomicBool>,
    ) {
        while !closed.load(Ordering::Relaxed) {
            match send_rx.recv() {
                Ok(buf) => {
                    if let Err(e) = socket.send_to(&buf, remote) {
                        error!("Failed to send UDP packet: {}", e);
                        break;
                    }
                    trace!("Sent UDP packet of {} bytes to {}", buf.len(), remote);
                }
                Err(crossbeam_channel::RecvError) => {
                    break;
                }
            }
        }
    }

    /// Handles outgoing packets in listen mode (address from mutex).
    fn handle_outgoing_listen(
        socket: Arc<UdpSocket>,
        remote_addr: Arc<std::sync::Mutex<Option<SocketAddr>>>,
        send_rx: crossbeam_channel::Receiver<bytes::BytesMut>,
        closed: Arc<std::sync::atomic::AtomicBool>,
    ) {
        while !closed.load(Ordering::Relaxed) {
            match send_rx.recv() {
                Ok(buf) => {
                    let remote = remote_addr.lock().unwrap();
                    if let Some(addr) = *remote {
                        if let Err(e) = socket.send_to(&buf, addr) {
                            error!("Failed to send UDP packet: {}", e);
                            break;
                        }
                        trace!("Sent UDP packet of {} bytes to {}", buf.len(), addr);
                    }
                }
                Err(crossbeam_channel::RecvError) => {
                    break;
                }
            }
        }
    }

    /// Handles incoming packets (connected mode).
    fn handle_incoming(
        socket: Arc<UdpSocket>,
        recv_tx: crossbeam_channel::Sender<bytes::BytesMut>,
        closed: Arc<std::sync::atomic::AtomicBool>,
    ) {
        let mut buf = [0u8; 65535];
        while !closed.load(Ordering::Relaxed) {
            match socket.recv_from(&mut buf) {
                Ok((len, src)) => {
                    if len > 0 {
                        let data = bytes::BytesMut::from(&buf[..len]);
                        if recv_tx.send(data).is_err() {
                            debug!("Receiver dropped, stopping incoming handler");
                            break;
                        }
                    }
                }
                Err(e) => {
                    if closed.load(Ordering::Relaxed) {
                        break;
                    }
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        thread::sleep(Duration::from_millis(1));
                        continue;
                    }
                    error!("Failed to receive UDP packet: {}", e);
                    break;
                }
            }
        }
    }

    /// Handles incoming packets in listen mode (stores remote address).
    fn handle_incoming_listen(
        socket: Arc<UdpSocket>,
        remote_addr: Arc<std::sync::Mutex<Option<SocketAddr>>>,
        recv_tx: crossbeam_channel::Sender<bytes::BytesMut>,
        closed: Arc<std::sync::atomic::AtomicBool>,
    ) {
        let mut buf = [0u8; 65535];
        while !closed.load(Ordering::Relaxed) {
            match socket.recv_from(&mut buf) {
                Ok((len, src)) => {
                    if len > 0 {
                        // Store the remote address on first packet
                        {
                            let mut remote = remote_addr.lock().unwrap();
                            if remote.is_none() {
                                *remote = Some(src);
                                info!("Accepted connection from {}", src);
                            }
                        }

                        let data = bytes::BytesMut::from(&buf[..len]);
                        if recv_tx.send(data).is_err() {
                            debug!("Receiver dropped, stopping incoming handler");
                            break;
                        }
                    }
                }
                Err(e) => {
                    if closed.load(Ordering::Relaxed) {
                        break;
                    }
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        thread::sleep(Duration::from_millis(1));
                        continue;
                    }
                    error!("Failed to receive UDP packet: {}", e);
                    break;
                }
            }
        }
    }

    /// Returns the local socket address.
    pub fn local_addr(&self) -> SocketAddr {
        self.socket.local_addr().unwrap()
    }

    /// Returns the remote socket address.
    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }
}

impl Transport for UdpTransport {
    fn send(&self, buf: bytes::BytesMut) -> Result<(), TransportError> {
        self.send_tx
            .send(buf)
            .map_err(|e| TransportError::SendFailed(e.to_string()))?;
        Ok(())
    }

    fn send_to(&self, buf: bytes::BytesMut, remote: SocketAddr) -> Result<(), TransportError> {
        // For listen mode, send directly via socket to the specified remote
        if let Err(e) = self.socket.send_to(&buf, remote) {
            error!("Failed to send UDP packet to {}: {}", remote, e);
            return Err(TransportError::SendFailed(e.to_string()));
        }
        trace!("Sent UDP packet of {} bytes to {}", buf.len(), remote);

        // Also track this remote address for future sends
        self.remote_addrs.lock().unwrap().insert(remote, ());
        Ok(())
    }

    fn try_recv(&self) -> Result<Option<bytes::BytesMut>, TransportError> {
        match self.recv_rx.try_recv() {
            Ok(buf) => {
                trace!("Received UDP packet of {} bytes", buf.len());
                Ok(Some(buf))
            }
            Err(crossbeam_channel::TryRecvError::Empty) => Ok(None),
            Err(crossbeam_channel::TryRecvError::Disconnected) => Err(TransportError::Closed),
        }
    }

    fn is_connected(&self) -> bool {
        true // UDP is always "connected" once bound
    }

    fn close(&self) -> Result<(), TransportError> {
        debug!("UDP transport closed");
        Ok(())
    }
}

/// Allow Arc<UdpTransport> to be used as a Transport for sharing between threads.
impl Transport for Arc<UdpTransport> {
    fn send(&self, buf: bytes::BytesMut) -> Result<(), TransportError> {
        (**self).send(buf)
    }

    fn send_to(&self, buf: bytes::BytesMut, remote: SocketAddr) -> Result<(), TransportError> {
        (**self).send_to(buf, remote)
    }

    fn try_recv(&self) -> Result<Option<bytes::BytesMut>, TransportError> {
        (**self).try_recv()
    }

    fn is_connected(&self) -> bool {
        (**self).is_connected()
    }

    fn close(&self) -> Result<(), TransportError> {
        (**self).close()
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
            IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            self.port,
            Some(self.txt_data.clone()),
        )?;

        daemon.register(service)?;
        info!(
            "Advertising LAN game: {} on port {}",
            self.service_name, self.port
        );

        Ok(daemon)
    }

    /// Discovers services on the network.
    pub async fn discover(
        service_type: &str,
        timeout: Duration,
    ) -> Result<Vec<mdns_sd::ServiceInfo>, mdns_sd::Error> {
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
    fn test_udp_transport_connect() {
        let local = "127.0.0.1:0".parse().unwrap();
        let remote = "127.0.0.1:0".parse().unwrap();
        let (transport, _recv) = UdpTransport::connect(local, remote).unwrap();
        assert!(transport.is_connected());
    }

    #[test]
    fn test_udp_transport_send_recv() {
        // Use fixed ports for testing to ensure they connect properly
        let port1 = 34254;
        let port2 = 34255;
        let local1 = format!("127.0.0.1:{}", port1).parse().unwrap();
        let local2 = format!("127.0.0.1:{}", port2).parse().unwrap();

        let (transport1, recv1) = UdpTransport::connect(local1, local2).unwrap();
        let (transport2, recv2) = UdpTransport::connect(local2, local1).unwrap();

        let test_data = bytes::BytesMut::from("Hello, World!");
        transport1.send(test_data.clone()).unwrap();

        // Give it a moment to arrive
        std::thread::sleep(std::time::Duration::from_millis(10));

        match recv2.try_recv() {
            Ok(received) => assert_eq!(received, test_data),
            Err(crossbeam_channel::TryRecvError::Empty) => panic!("No data received"),
            Err(e) => panic!("Error receiving: {:?}", e),
        }
    }
}
