use crate::{
    Decode, Encode, ProtocolError, read_string, read_uuid, write_string, write_uuid, write_varint,
};
use bytes::{Buf, BufMut, BytesMut};
use uuid::Uuid;

/// Configuration state packet IDs.
pub mod packet_id {
    // Client -> Server
    pub const CLIENT_INFORMATION: u8 = 0x00;
    pub const PLUGIN_MESSAGE: u8 = 0x01;
    pub const ACKNOWLEDGE_CONFIGURATION: u8 = 0x02;
    pub const FINISH_CONFIGURATION: u8 = 0x03;
    pub const SELECT_KNOWN_PACKS: u8 = 0x04;
    pub const MOD_LIST: u8 = 0x05;

    // Server -> Client
    pub const CLIENTBOUND_PLUGIN_MESSAGE: u8 = 0x00;
    pub const SYNC_PLAYER_POSITION: u8 = 0x01;
    pub const ADD_RESOURCE_PACK: u8 = 0x02;
    pub const REMOVE_RESOURCE_PACK: u8 = 0x03;
    pub const ADD_TAGS: u8 = 0x04;
    pub const FINISH_CONFIGURATION_SB: u8 = 0x05;
    pub const REGISTRY_DATA: u8 = 0x06;
    pub const UPDATE_TAGS: u8 = 0x07;
    pub const MOD_LIST_SB: u8 = 0x08;
    pub const MOD_MISMATCH: u8 = 0x09;
}

/// Client Information packet (client -> server).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientInformation {
    pub locale: String,
    pub view_distance: u8,
    pub chat_mode: ChatMode,
    pub chat_colors: bool,
    pub displayed_skin_parts: u8,
    pub main_hand: MainHand,
    pub enable_text_filtering: bool,
    pub allow_server_listings: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChatMode {
    Enabled = 0,
    CommandsOnly = 1,
    Hidden = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MainHand {
    Left = 0,
    Right = 1,
}

impl crate::Encode for ClientInformation {
    fn encode(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::ProtocolError> {
        crate::write_string(buf, &self.locale);
        buf.put_u8(self.view_distance);
        buf.put_u8(self.chat_mode as u8);
        buf.put_u8(self.chat_colors as u8);
        buf.put_u8(self.displayed_skin_parts);
        buf.put_u8(self.main_hand as u8);
        buf.put_u8(self.enable_text_filtering as u8);
        buf.put_u8(self.allow_server_listings as u8);
        Ok(())
    }
}

impl<'a> crate::Decode<'a> for ClientInformation {
    fn decode(buf: &mut bytes::BytesMut) -> Result<Self, crate::ProtocolError> {
        let locale = crate::read_string(buf)?;
        let view_distance = buf.get_u8();
        let chat_mode = match buf.get_u8() {
            0 => ChatMode::Enabled,
            1 => ChatMode::CommandsOnly,
            2 => ChatMode::Hidden,
            _ => {
                return Err(crate::ProtocolError::DecodeFailed(
                    "invalid chat mode".into(),
                ));
            }
        };
        let chat_colors = buf.get_u8() != 0;
        let displayed_skin_parts = buf.get_u8();
        let main_hand = match buf.get_u8() {
            0 => MainHand::Left,
            1 => MainHand::Right,
            _ => {
                return Err(crate::ProtocolError::DecodeFailed(
                    "invalid main hand".into(),
                ));
            }
        };
        let enable_text_filtering = buf.get_u8() != 0;
        let allow_server_listings = buf.get_u8() != 0;
        Ok(Self {
            locale,
            view_distance,
            chat_mode,
            chat_colors,
            displayed_skin_parts,
            main_hand,
            enable_text_filtering,
            allow_server_listings,
        })
    }
}

/// Finish Configuration packet (both directions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishConfiguration;

impl crate::Encode for FinishConfiguration {
    fn encode(&self, _buf: &mut bytes::BytesMut) -> Result<(), crate::ProtocolError> {
        Ok(())
    }
}

impl<'a> crate::Decode<'a> for FinishConfiguration {
    fn decode(_buf: &mut bytes::BytesMut) -> Result<Self, crate::ProtocolError> {
        Ok(Self)
    }
}

/// Registry Data packet (server -> client).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryData {
    pub registry_id: String,
    pub entries: Vec<RegistryEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryEntry {
    pub identifier: String,
    pub data: Vec<u8>,
}

impl crate::Encode for RegistryData {
    fn encode(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::ProtocolError> {
        crate::write_string(buf, &self.registry_id);
        crate::write_varint(buf, self.entries.len() as i32);
        for entry in &self.entries {
            crate::write_string(buf, &entry.identifier);
            crate::write_varint(buf, entry.data.len() as i32);
            buf.put_slice(&entry.data);
        }
        Ok(())
    }
}

impl<'a> crate::Decode<'a> for RegistryData {
    fn decode(buf: &mut bytes::BytesMut) -> Result<Self, crate::ProtocolError> {
        let registry_id = crate::read_string(buf)?;
        let entry_count = crate::read_varint(buf)? as usize;
        let mut entries = Vec::with_capacity(entry_count);
        for _ in 0..entry_count {
            let identifier = crate::read_string(buf)?;
            let data_len = crate::read_varint(buf)? as usize;
            if buf.remaining() < data_len {
                return Err(crate::ProtocolError::UnexpectedEof);
            }
            let data = buf.copy_to_bytes(data_len).to_vec();
            entries.push(RegistryEntry { identifier, data });
        }
        Ok(Self {
            registry_id,
            entries,
        })
    }
}

/// Mod entry for mod list synchronization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModEntry {
    pub id: String,
    pub version: String,
    pub checksum: String, // SHA256 of WASM file
}

/// Mod List packet (server -> client): sends the list of required mods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModList {
    pub mods: Vec<ModEntry>,
    pub required: bool, // If true, client must have all mods
}

impl crate::Encode for ModList {
    fn encode(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::ProtocolError> {
        crate::write_varint(buf, self.mods.len() as i32);
        for mod_entry in &self.mods {
            crate::write_string(buf, &mod_entry.id);
            crate::write_string(buf, &mod_entry.version);
            crate::write_string(buf, &mod_entry.checksum);
        }
        buf.put_u8(if self.required { 1 } else { 0 });
        Ok(())
    }
}

impl<'a> crate::Decode<'a> for ModList {
    fn decode(buf: &mut bytes::BytesMut) -> Result<Self, crate::ProtocolError> {
        let count = crate::read_varint(buf)? as usize;
        let mut mods = Vec::with_capacity(count);
        for _ in 0..count {
            let id = crate::read_string(buf)?;
            let version = crate::read_string(buf)?;
            let checksum = crate::read_string(buf)?;
            mods.push(ModEntry {
                id,
                version,
                checksum,
            });
        }
        let required = buf.get_u8() != 0;
        Ok(Self { mods, required })
    }
}

/// Mod Mismatch packet (server -> client): informs client of missing/mismatched mods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModMismatch {
    pub missing: Vec<ModEntry>,
    pub mismatched: Vec<ModEntry>,
    pub extra: Vec<ModEntry>,
}

impl crate::Encode for ModMismatch {
    fn encode(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::ProtocolError> {
        crate::write_varint(buf, self.missing.len() as i32);
        for mod_entry in &self.missing {
            crate::write_string(buf, &mod_entry.id);
            crate::write_string(buf, &mod_entry.version);
            crate::write_string(buf, &mod_entry.checksum);
        }
        crate::write_varint(buf, self.mismatched.len() as i32);
        for mod_entry in &self.mismatched {
            crate::write_string(buf, &mod_entry.id);
            crate::write_string(buf, &mod_entry.version);
            crate::write_string(buf, &mod_entry.checksum);
        }
        crate::write_varint(buf, self.extra.len() as i32);
        for mod_entry in &self.extra {
            crate::write_string(buf, &mod_entry.id);
            crate::write_string(buf, &mod_entry.version);
            crate::write_string(buf, &mod_entry.checksum);
        }
        Ok(())
    }
}

impl<'a> crate::Decode<'a> for ModMismatch {
    fn decode(buf: &mut bytes::BytesMut) -> Result<Self, crate::ProtocolError> {
        let missing_count = crate::read_varint(buf)? as usize;
        let mut missing = Vec::with_capacity(missing_count);
        for _ in 0..missing_count {
            let id = crate::read_string(buf)?;
            let version = crate::read_string(buf)?;
            let checksum = crate::read_string(buf)?;
            missing.push(ModEntry {
                id,
                version,
                checksum,
            });
        }
        let mismatched_count = crate::read_varint(buf)? as usize;
        let mut mismatched = Vec::with_capacity(mismatched_count);
        for _ in 0..mismatched_count {
            let id = crate::read_string(buf)?;
            let version = crate::read_string(buf)?;
            let checksum = crate::read_string(buf)?;
            mismatched.push(ModEntry {
                id,
                version,
                checksum,
            });
        }
        let extra_count = crate::read_varint(buf)? as usize;
        let mut extra = Vec::with_capacity(extra_count);
        for _ in 0..extra_count {
            let id = crate::read_string(buf)?;
            let version = crate::read_string(buf)?;
            let checksum = crate::read_string(buf)?;
            extra.push(ModEntry {
                id,
                version,
                checksum,
            });
        }
        Ok(Self {
            missing,
            mismatched,
            extra,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Decode, Encode};
    use bytes::BytesMut;

    #[test]
    fn test_finish_configuration_roundtrip() {
        let packet = FinishConfiguration;
        let mut buf = bytes::BytesMut::new();
        packet.encode(&mut buf).unwrap();

        let mut decode_buf = buf.clone();
        let decoded = FinishConfiguration::decode(&mut decode_buf).unwrap();
        assert_eq!(decoded, packet);
    }

    #[test]
    fn test_mod_list_roundtrip() {
        let packet = ModList {
            mods: vec![ModEntry {
                id: "example:mod".to_string(),
                version: "1.0.0".to_string(),
                checksum: "abc123".to_string(),
            }],
            required: true,
        };
        let mut buf = bytes::BytesMut::new();
        packet.encode(&mut buf).unwrap();

        let mut decode_buf = buf.clone();
        let decoded = ModList::decode(&mut decode_buf).unwrap();
        assert_eq!(decoded, packet);
    }

    #[test]
    fn test_mod_mismatch_roundtrip() {
        let packet = ModMismatch {
            missing: vec![ModEntry {
                id: "example:mod".to_string(),
                version: "1.0.0".to_string(),
                checksum: "abc123".to_string(),
            }],
            mismatched: vec![ModEntry {
                id: "example:mod2".to_string(),
                version: "1.0.0".to_string(),
                checksum: "def456".to_string(),
            }],
            extra: vec![ModEntry {
                id: "example:mod3".to_string(),
                version: "1.0.0".to_string(),
                checksum: "ghi789".to_string(),
            }],
        };
        let mut buf = bytes::BytesMut::new();
        packet.encode(&mut buf).unwrap();

        let mut decode_buf = buf.clone();
        let decoded = ModMismatch::decode(&mut decode_buf).unwrap();
        assert_eq!(decoded, packet);
    }
}
