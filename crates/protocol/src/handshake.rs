use crate::{Decode, Encode, ProtocolError};
use bytes::{Buf, BufMut, BytesMut};
use uuid::Uuid;

/// Handshake state packet IDs.
pub mod packet_id {
    pub const HANDSHAKE: u8 = 0x00;
}

/// Handshake packet (client -> server).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakePacket {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: NextState,
}

/// Next state after handshake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NextState {
    Status = 1,
    Login = 2,
}

impl Encode for HandshakePacket {
    fn encode(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::ProtocolError> {
        crate::write_varint(buf, self.protocol_version);
        crate::write_string(buf, &self.server_address);
        buf.put_u16(self.server_port);
        crate::write_varint(buf, self.next_state as i32);
        Ok(())
    }
}

impl<'a> Decode<'a> for HandshakePacket {
    fn decode(buf: &mut bytes::BytesMut) -> Result<Self, crate::ProtocolError> {
        let protocol_version = crate::read_varint(buf)?;
        let server_address = crate::read_string(buf)?;
        let server_port = buf.get_u16();
        let next_state = match crate::read_varint(buf)? {
            1 => NextState::Status,
            2 => NextState::Login,
            _ => {
                return Err(crate::ProtocolError::DecodeFailed(
                    "invalid next state".into(),
                ));
            }
        };
        Ok(Self {
            protocol_version,
            server_address,
            server_port,
            next_state,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Decode, Encode};
    use bytes::BytesMut;

    #[test]
    fn test_handshake_roundtrip() {
        let packet = HandshakePacket {
            protocol_version: 1,
            server_address: "localhost".to_string(),
            server_port: 25565,
            next_state: NextState::Login,
        };

        let mut buf = bytes::BytesMut::new();
        packet.encode(&mut buf).unwrap();

        let mut decode_buf = buf.clone();
        let decoded = HandshakePacket::decode(&mut decode_buf).unwrap();
        assert_eq!(decoded, packet);
    }
}
