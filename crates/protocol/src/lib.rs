use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::io;
use thiserror::Error;

pub mod configuration;
pub mod handshake;
pub mod login;
pub mod play;

/// Protocol version for LCE baseline (TU19)
pub const PROTOCOL_VERSION: i32 = 1;

/// Maximum packet size (1MB)
pub const MAX_PACKET_SIZE: usize = 1024 * 1024;

/// Trait for encoding packets to bytes.
pub trait Encode {
    fn encode(&self, buf: &mut BytesMut) -> Result<(), ProtocolError>;
}

/// Trait for decoding packets from bytes.
pub trait Decode<'a>: Sized {
    fn decode(buf: &mut BytesMut) -> Result<Self, ProtocolError>;
}

/// Protocol errors.
#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Invalid packet ID: {0}")]
    InvalidPacketId(u32),

    #[error("Packet too large: {size} > {max}", size = .0, max = MAX_PACKET_SIZE)]
    PacketTooLarge(usize),

    #[error("Unexpected EOF")]
    UnexpectedEof,

    #[error("Invalid string: {0}")]
    InvalidString(#[from] std::string::FromUtf8Error),

    #[error("Invalid varint")]
    InvalidVarInt,

    #[error("Packet decode failed: {0}")]
    DecodeFailed(String),

    #[error("Unknown packet state: {0}")]
    UnknownState(u8),
}

/// Reads a varint from the buffer.
pub fn read_varint(buf: &mut BytesMut) -> Result<i32, ProtocolError> {
    let mut result = 0i32;
    let mut shift = 0;

    loop {
        if buf.is_empty() {
            return Err(ProtocolError::UnexpectedEof);
        }

        let byte = buf.get_u8();
        result |= ((byte & 0x7F) as i32) << shift;

        if (byte & 0x80) == 0 {
            return Ok(result);
        }

        shift += 7;
        if shift >= 32 {
            return Err(ProtocolError::InvalidVarInt);
        }
    }
}

/// Writes a varint to the buffer.
pub fn write_varint(buf: &mut BytesMut, value: i32) {
    let mut value = value as u32;
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.put_u8(byte);
        if value == 0 {
            break;
        }
    }
}

/// Reads a string from the buffer (varint length + UTF-8 bytes).
pub fn read_string(buf: &mut BytesMut) -> Result<String, ProtocolError> {
    let len = read_varint(buf)? as usize;
    if buf.remaining() < len {
        return Err(ProtocolError::UnexpectedEof);
    }
    let bytes = buf.copy_to_bytes(len);
    String::from_utf8(bytes.to_vec()).map_err(ProtocolError::InvalidString)
}

/// Writes a string to the buffer (varint length + UTF-8 bytes).
pub fn write_string(buf: &mut BytesMut, s: &str) {
    write_varint(buf, s.len() as i32);
    buf.put_slice(s.as_bytes());
}

/// Reads a UUID from the buffer (16 bytes).
pub fn read_uuid(buf: &mut BytesMut) -> Result<uuid::Uuid, ProtocolError> {
    if buf.remaining() < 16 {
        return Err(ProtocolError::UnexpectedEof);
    }
    let bytes = buf.copy_to_bytes(16);
    let uuid = uuid::Uuid::from_slice(&bytes)
        .map_err(|_| ProtocolError::DecodeFailed("invalid UUID".into()))?;
    Ok(uuid)
}

/// Writes a UUID to the buffer (16 bytes).
pub fn write_uuid(buf: &mut BytesMut, uuid: uuid::Uuid) {
    buf.put_slice(uuid.as_bytes());
}

pub mod prelude {
    pub use crate::{
        Decode, Encode, ProtocolError, read_string, read_uuid, read_varint, write_string,
        write_uuid, write_varint,
    };
    pub use bytes::{Buf, BufMut, Bytes, BytesMut};
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn test_varint_roundtrip() {
        // VarInts are unsigned in Minecraft protocol; ZigZag is used for signed values
        let test_values = [0, 1, 127, 128, 255, 25565, 2097151, 2147483647];
        for value in test_values {
            let mut buf = BytesMut::new();
            write_varint(&mut buf, value);
            let decoded = read_varint(&mut buf).unwrap();
            assert_eq!(decoded, value, "failed for value: {}", value);
        }
    }

    #[test]
    fn test_string_roundtrip() {
        let test_strings = [
            "",
            "hello",
            "hello world",
            "🎮",
            "Minecraft: Legacy Rust Edition",
        ];
        for s in test_strings {
            let mut buf = BytesMut::new();
            write_string(&mut buf, s);
            let decoded = read_string(&mut buf).unwrap();
            assert_eq!(decoded, s);
        }
    }

    #[test]
    fn test_uuid_roundtrip() {
        let uuid = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let mut buf = BytesMut::new();
        write_uuid(&mut buf, uuid);
        let decoded = read_uuid(&mut buf).unwrap();
        assert_eq!(decoded, uuid);
    }
}
