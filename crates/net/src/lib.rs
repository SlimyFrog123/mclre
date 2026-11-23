use bytes::BytesMut;
use crossbeam_channel::{self, Receiver, Sender, TryRecvError};
use mclre_protocol::{Decode, Encode, ProtocolError};
use std::io;
use thiserror::Error;
use tracing::{debug, info, trace, warn};

mod udp_transport;
pub use udp_transport::*;
mod upnp;
pub use upnp::*;

// Re-export ServiceInfo from mdns_sd for client use
pub use mdns_sd::ServiceInfo;

/// Transport trait for network communication.
/// Implementations handle sending/receiving raw packet bytes.
pub trait Transport: Send + Sync {
    /// Send a packet buffer to the remote endpoint.
    fn send(&self, buf: BytesMut) -> Result<(), TransportError>;

    /// Send a packet buffer to a specific remote address (for UDP server mode).
    /// Default implementation delegates to `send` for transports that don't support multiple remotes.
    fn send_to(&self, buf: BytesMut, _remote: std::net::SocketAddr) -> Result<(), TransportError> {
        self.send(buf)
    }

    /// Try to receive a packet buffer from the remote endpoint.
    /// Returns Ok(Some(buf)) if a packet is available,
    /// Ok(None) if no packet is available yet (non-blocking),
    /// Err if the connection is closed or errored.
    fn try_recv(&self) -> Result<Option<BytesMut>, TransportError>;

    /// Check if the transport is connected.
    fn is_connected(&self) -> bool;

    /// Close the transport.
    fn close(&self) -> Result<(), TransportError>;
}

/// Errors that can occur during transport operations.
#[derive(Debug, Error)]
pub enum TransportError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Connection closed")]
    Closed,

    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Receive failed: {0}")]
    RecvFailed(String),
}

/// Trait for starting and managing an integrated server.
/// Allows the client to start the server when "Play Game" is clicked without depending on server internals.
pub trait ServerHandle: Send + Sync {
    /// Start the server in a background thread.
    /// Returns a handle to stop the server.
    fn start(&mut self) -> Box<dyn FnOnce() + Send>;

    /// Add a client to the server (called before starting).
    fn add_client(
        &mut self,
        transport: Box<dyn Transport + Send + Sync>,
        identity: mclre_identity::PlayerIdentity,
    ) -> Result<i32, crate::TransportError>;
}

/// In-memory transport for integrated server (singleplayer).
/// Uses crossbeam channels for zero-copy message passing between client and server.
pub struct InMemoryTransport {
    /// Sender for outgoing packets (client->server or server->client)
    outgoing: Sender<BytesMut>,
    /// Receiver for incoming packets (server->client or client->server)
    incoming: Receiver<BytesMut>,
    /// Whether this endpoint is the client (true) or server (false)
    is_client: bool,
}

/// A pair of connected in-memory transports (client, server).
pub type InMemoryTransportPair = (InMemoryTransport, InMemoryTransport);

impl InMemoryTransport {
    /// Create a new pair of connected in-memory transports.
    /// Returns (client_transport, server_transport).
    pub fn pair() -> InMemoryTransportPair {
        // Channel 1: Client -> Server
        let (client_to_server_tx, client_to_server_rx) = crossbeam_channel::unbounded::<BytesMut>();
        // Channel 2: Server -> Client
        let (server_to_client_tx, server_to_client_rx) = crossbeam_channel::unbounded::<BytesMut>();

        let client = InMemoryTransport {
            outgoing: client_to_server_tx,
            incoming: server_to_client_rx,
            is_client: true,
        };

        let server = InMemoryTransport {
            outgoing: server_to_client_tx,
            incoming: client_to_server_rx,
            is_client: false,
        };

        (client, server)
    }

    /// Create a client-side in-memory transport (for testing).
    pub fn client() -> (Self, InMemoryTransport) {
        Self::pair()
    }

    /// Create a server-side in-memory transport (for testing).
    pub fn server() -> (InMemoryTransport, Self) {
        let (client, server) = Self::pair();
        (server, client)
    }
}

impl Transport for InMemoryTransport {
    fn send(&self, buf: BytesMut) -> Result<(), TransportError> {
        trace!("Sending packet of {} bytes", buf.len());
        self.outgoing
            .send(buf)
            .map_err(|e| TransportError::SendFailed(e.to_string()))
    }

    fn try_recv(&self) -> Result<Option<BytesMut>, TransportError> {
        match self.incoming.try_recv() {
            Ok(buf) => {
                trace!("Received packet of {} bytes", buf.len());
                Ok(Some(buf))
            }
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(TransportError::Closed),
        }
    }

    fn is_connected(&self) -> bool {
        !self.incoming.is_empty() || !self.outgoing.is_full()
    }

    fn close(&self) -> Result<(), TransportError> {
        debug!("Closing in-memory transport");
        // Dropping the senders will cause the receivers to return Disconnected
        Ok(())
    }
}

/// Packet codec for encoding/decoding packets with length prefix.
pub struct PacketCodec;

impl PacketCodec {
    /// Encode a packet to a length-prefixed buffer.
    pub fn encode<P: Encode>(packet: &P) -> Result<BytesMut, ProtocolError> {
        let mut buf = BytesMut::new();
        packet.encode(&mut buf)?;
        Ok(buf)
    }

    /// Decode a packet from a buffer.
    pub fn decode<'a, P: Decode<'a>>(buf: &mut BytesMut) -> Result<P, ProtocolError> {
        P::decode(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mclre_protocol::play::KeepAlive;

    #[test]
    fn test_in_memory_transport_pair() {
        let (client, server) = InMemoryTransport::pair();

        assert!(client.is_connected());
        assert!(server.is_connected());

        // Client sends to server
        let mut buf = BytesMut::new();
        let packet = KeepAlive { id: 42 };
        packet.encode(&mut buf).unwrap();

        client.send(buf.clone()).unwrap();

        // Server receives
        let received = server.try_recv().unwrap().unwrap();
        let decoded = KeepAlive::decode(&mut received.clone()).unwrap();
        assert_eq!(decoded.id, 42);

        // Server sends back
        let mut buf2 = BytesMut::new();
        let packet2 = KeepAlive { id: 43 };
        packet2.encode(&mut buf2).unwrap();
        server.send(buf2).unwrap();

        // Client receives
        let received2 = client.try_recv().unwrap().unwrap();
        let decoded2 = KeepAlive::decode(&mut received2.clone()).unwrap();
        assert_eq!(decoded2.id, 43);
    }

    #[test]
    fn test_in_memory_transport_try_recv_empty() {
        let (client, _server) = InMemoryTransport::pair();
        let result = client.try_recv().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_packet_codec() {
        let packet = KeepAlive { id: 12345 };
        let encoded = PacketCodec::encode(&packet).unwrap();
        let mut decode_buf = encoded.clone();
        let decoded = PacketCodec::decode::<KeepAlive>(&mut decode_buf).unwrap();
        assert_eq!(decoded, packet);
    }
}
