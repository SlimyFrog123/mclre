use crate::{Decode, Encode, ProtocolError, read_string, read_uuid, write_string, write_uuid};
use bytes::{Buf, BufMut, BytesMut};
use uuid::Uuid;

/// Login state packet IDs (server -> client).
pub mod serverbound {
    pub const LOGIN_START: u8 = 0x00;
    pub const ENCRYPTION_RESPONSE: u8 = 0x01;
    pub const LOGIN_ACKNOWLEDGED: u8 = 0x02;
}

/// Login state packet IDs (client -> server).
pub mod clientbound {
    pub const DISCONNECT: u8 = 0x00;
    pub const ENCRYPTION_REQUEST: u8 = 0x01;
    pub const LOGIN_SUCCESS: u8 = 0x02;
    pub const SET_COMPRESSION: u8 = 0x03;
    pub const LOGIN_PLUGIN_REQUEST: u8 = 0x04;
}

/// Login Start packet (client -> server).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginStart {
    pub username: String,
    pub uuid: Option<Uuid>,
}

impl crate::Encode for LoginStart {
    fn encode(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::ProtocolError> {
        crate::write_string(buf, &self.username);
        match &self.uuid {
            Some(uuid) => {
                buf.put_u8(1);
                crate::write_uuid(buf, *uuid);
            }
            None => {
                buf.put_u8(0);
            }
        }
        Ok(())
    }
}

impl<'a> crate::Decode<'a> for LoginStart {
    fn decode(buf: &mut bytes::BytesMut) -> Result<Self, crate::ProtocolError> {
        let username = crate::read_string(buf)?;
        let has_uuid = buf.get_u8();
        let uuid = if has_uuid != 0 {
            Some(crate::read_uuid(buf)?)
        } else {
            None
        };
        Ok(Self { username, uuid })
    }
}

/// Encryption Request packet (server -> client).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptionRequest {
    pub server_id: String,
    pub public_key: Vec<u8>,
    pub verify_token: Vec<u8>,
}

impl crate::Encode for EncryptionRequest {
    fn encode(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::ProtocolError> {
        crate::write_string(buf, &self.server_id);
        crate::write_varint(buf, self.public_key.len() as i32);
        buf.put_slice(&self.public_key);
        crate::write_varint(buf, self.verify_token.len() as i32);
        buf.put_slice(&self.verify_token);
        Ok(())
    }
}

impl<'a> crate::Decode<'a> for EncryptionRequest {
    fn decode(buf: &mut bytes::BytesMut) -> Result<Self, crate::ProtocolError> {
        let server_id = crate::read_string(buf)?;
        let pub_key_len = crate::read_varint(buf)? as usize;
        if buf.remaining() < pub_key_len {
            return Err(crate::ProtocolError::UnexpectedEof);
        }
        let public_key = buf.copy_to_bytes(pub_key_len).to_vec();
        let verify_token_len = crate::read_varint(buf)? as usize;
        if buf.remaining() < verify_token_len {
            return Err(crate::ProtocolError::UnexpectedEof);
        }
        let verify_token = buf.copy_to_bytes(verify_token_len).to_vec();
        Ok(Self {
            server_id,
            public_key,
            verify_token,
        })
    }
}

/// Login Success packet (server -> client).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginSuccess {
    pub uuid: Uuid,
    pub username: String,
    pub properties: Vec<ProfileProperty>,
}

/// Player profile property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileProperty {
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
}

impl crate::Encode for LoginSuccess {
    fn encode(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::ProtocolError> {
        crate::write_uuid(buf, self.uuid);
        crate::write_string(buf, &self.username);
        crate::write_varint(buf, self.properties.len() as i32);
        for prop in &self.properties {
            crate::write_string(buf, &prop.name);
            crate::write_string(buf, &prop.value);
            match &prop.signature {
                Some(sig) => {
                    buf.put_u8(1);
                    crate::write_string(buf, sig);
                }
                None => buf.put_u8(0),
            }
        }
        Ok(())
    }
}

impl<'a> crate::Decode<'a> for LoginSuccess {
    fn decode(buf: &mut bytes::BytesMut) -> Result<Self, crate::ProtocolError> {
        let uuid = crate::read_uuid(buf)?;
        let username = crate::read_string(buf)?;
        let prop_count = crate::read_varint(buf)? as usize;
        let mut properties = Vec::with_capacity(prop_count);
        for _ in 0..prop_count {
            let name = crate::read_string(buf)?;
            let value = crate::read_string(buf)?;
            let has_sig = buf.get_u8();
            let signature = if has_sig != 0 {
                Some(crate::read_string(buf)?)
            } else {
                None
            };
            properties.push(ProfileProperty {
                name,
                value,
                signature,
            });
        }
        Ok(Self {
            uuid,
            username,
            properties,
        })
    }
}

/// Disconnect packet (both directions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisconnectPacket {
    pub reason: String,
}

impl crate::Encode for DisconnectPacket {
    fn encode(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::ProtocolError> {
        crate::write_string(buf, &self.reason);
        Ok(())
    }
}

impl<'a> crate::Decode<'a> for DisconnectPacket {
    fn decode(buf: &mut bytes::BytesMut) -> Result<Self, crate::ProtocolError> {
        let reason = crate::read_string(buf)?;
        Ok(Self { reason })
    }
}

/// Set Compression packet (server -> client).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetCompression {
    pub threshold: i32,
}

impl crate::Encode for SetCompression {
    fn encode(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::ProtocolError> {
        crate::write_varint(buf, self.threshold);
        Ok(())
    }
}

impl<'a> crate::Decode<'a> for SetCompression {
    fn decode(buf: &mut bytes::BytesMut) -> Result<Self, crate::ProtocolError> {
        let threshold = crate::read_varint(buf)?;
        Ok(Self { threshold })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Decode, Encode};
    use bytes::BytesMut;
    use uuid::Uuid;

    #[test]
    fn test_login_start_roundtrip() {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let packet = LoginStart {
            username: "TestPlayer".to_string(),
            uuid: Some(uuid),
        };

        let mut buf = BytesMut::new();
        packet.encode(&mut buf).unwrap();

        let mut decode_buf = buf.clone();
        let decoded = LoginStart::decode(&mut decode_buf).unwrap();
        assert_eq!(decoded, packet);
    }

    #[test]
    fn test_disconnect_roundtrip() {
        let packet = DisconnectPacket {
            reason: "Disconnected by server".to_string(),
        };

        let mut buf = BytesMut::new();
        packet.encode(&mut buf).unwrap();

        let mut decode_buf = buf.clone();
        let decoded = DisconnectPacket::decode(&mut decode_buf).unwrap();
        assert_eq!(decoded, packet);
    }

    #[test]
    fn test_set_compression_roundtrip() {
        let packet = SetCompression { threshold: 256 };

        let mut buf = BytesMut::new();
        packet.encode(&mut buf).unwrap();

        let mut decode_buf = buf.clone();
        let decoded = SetCompression::decode(&mut decode_buf).unwrap();
        assert_eq!(decoded, packet);
    }
}
