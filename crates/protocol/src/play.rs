use crate::{Encode, Decode, ProtocolError, read_string, read_uuid, write_string, write_uuid, read_varint, write_varint};
use bytes::{Buf, BufMut, BytesMut};
use uuid::Uuid;

/// Play state packet IDs.
pub mod packet_id {
    // Client -> Server
    pub const TELEPORT_CONFIRM: u8 = 0x00;
    pub const CHAT_MESSAGE: u8 = 0x01;
    pub const CLIENT_COMMAND: u8 = 0x02;
    pub const CLIENT_INFORMATION: u8 = 0x03;
    pub const PING: u8 = 0x04;
    pub const BLOCK_DIG: u8 = 0x05;
    pub const PLAYER_ACTION: u8 = 0x06;
    pub const PLAYER_POSITION: u8 = 0x07;
    pub const PLAYER_POSITION_AND_LOOK: u8 = 0x08;
    pub const PLAYER_LOOK: u8 = 0x09;
    pub const VEHICLE_MOVE: u8 = 0x0A;
    pub const STEER_BOAT: u8 = 0x0B;
    pub const PLAYER_ABILITIES: u8 = 0x0C;
    pub const PLAYER_DIGGING: u8 = 0x0D;
    pub const PLAYER_BLOCK_PLACEMENT: u8 = 0x0E;
    pub const HELD_ITEM_CHANGE: u8 = 0x0F;
    pub const CREATIVE_INVENTORY_ACTION: u8 = 0x10;
    pub const SWING_ARM: u8 = 0x11;
    pub const PLAYER_COMMAND: u8 = 0x12;
    pub const RESOURCE_PACK_STATUS: u8 = 0x13;
    pub const SET_CREATIVE_SLOT: u8 = 0x14;
    pub const PLAYER_INTERACTION: u8 = 0x15;

    // Server -> Client
    pub const SPAWN_ENTITY: u8 = 0x00;
    pub const SPAWN_PLAYER: u8 = 0x01;
    pub const SPAWN_LIVING_ENTITY: u8 = 0x02;
    pub const SPAWN_PROJECTILE: u8 = 0x03;
    pub const SPAWN_EXPERIENCE_ORB: u8 = 0x04;
    pub const ENTITY_ANIMATION: u8 = 0x05;
    pub const ENTITY_STATUS: u8 = 0x06;
    pub const ENTITY_RELATIVE_MOVE: u8 = 0x07;
    pub const ENTITY_LOOK: u8 = 0x08;
    pub const ENTITY_LOOK_AND_RELATIVE_MOVE: u8 = 0x09;
    pub const ENTITY_TELEPORT: u8 = 0x0A;
    pub const ENTITY_HEAD_ROTATION: u8 = 0x0B;
    pub const ENTITY_METADATA: u8 = 0x0C;
    pub const ATTACH_ENTITY: u8 = 0x0D;
    pub const ENTITY_VELOCITY: u8 = 0x0E;
    pub const ENTITY_EQUIPMENT: u8 = 0x0F;
    pub const SET_EXPERIENCE: u8 = 0x10;
    pub const UPDATE_HEALTH: u8 = 0x11;
    pub const RESPAWN: u8 = 0x12;
    pub const ENTITY_EFFECT: u8 = 0x13;
    pub const REMOVE_ENTITY_EFFECT: u8 = 0x14;
    pub const SET_RESPAWN_SCREEN: u8 = 0x15;
    pub const PLAYER_INFO: u8 = 0x16;
    pub const SYNC_PLAYER_POSITION: u8 = 0x17;
    pub const ENTITY_SOUND_EFFECT: u8 = 0x18;
    pub const SOUND_EFFECT: u8 = 0x19;
    pub const STOP_SOUND: u8 = 0x1A;
    pub const WORLD_EVENT: u8 = 0x1B;
    pub const EXPLOSION: u8 = 0x1C;
    pub const UNLOAD_CHUNK: u8 = 0x1D;
    pub const CHANGE_GAME_STATE: u8 = 0x1E;
    pub const KEEP_ALIVE: u8 = 0x1F;
    pub const CHUNK_DATA: u8 = 0x20;
    pub const EFFECT: u8 = 0x21;
    pub const PARTICLE: u8 = 0x22;
    pub const JOIN_GAME: u8 = 0x23;
    pub const MAP_DATA: u8 = 0x24;
    pub const ENTITY_PROPERTIES: u8 = 0x25;
    pub const PLAYER_LIST_ITEM: u8 = 0x26;
    pub const ENTITY_NBT_QUERY_RESPONSE: u8 = 0x27;
    pub const INVENTORY_CONTENT: u8 = 0x28;
    pub const INVENTORY_SLOT: u8 = 0x29;
    pub const OPEN_SCREEN: u8 = 0x2A;
    pub const SET_SLOT: u8 = 0x2B;
    pub const SET_CONTAINER_CONTENT: u8 = 0x2C;
    pub const MERCHANT_OFFERS: u8 = 0x2D;
    pub const UPDATE_ATTRIBUTES: u8 = 0x2E;
    pub const UPDATE_RECIPES: u8 = 0x2F;
    pub const UPDATE_TAGS: u8 = 0x30;
}

/// Keep Alive packet (both directions).
#[derive(Debug, Clone, PartialEq)]
pub struct KeepAlive {
    pub id: i64,
}

impl crate::Encode for KeepAlive {
    fn encode(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::ProtocolError> {
        buf.put_i64(self.id);
        Ok(())
    }
}

impl<'a> crate::Decode<'a> for KeepAlive {
    fn decode(buf: &mut bytes::BytesMut) -> Result<Self, crate::ProtocolError> {
        if buf.remaining() < 8 {
            return Err(crate::ProtocolError::UnexpectedEof);
        }
        let id = buf.get_i64();
        Ok(Self { id })
    }
}

/// Chat Message packet (client -> server).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub message: String,
}

impl crate::Encode for ChatMessage {
    fn encode(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::ProtocolError> {
        crate::write_string(buf, &self.message);
        Ok(())
    }
}

impl<'a> crate::Decode<'a> for ChatMessage {
    fn decode(buf: &mut bytes::BytesMut) -> Result<Self, crate::ProtocolError> {
        let message = crate::read_string(buf)?;
        Ok(Self { message })
    }
}

/// Player Position packet (client -> server).
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub on_ground: bool,
}

impl crate::Encode for PlayerPosition {
    fn encode(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::ProtocolError> {
        buf.put_f64(self.x);
        buf.put_f64(self.y);
        buf.put_f64(self.z);
        buf.put_u8(self.on_ground as u8);
        Ok(())
    }
}

impl<'a> crate::Decode<'a> for PlayerPosition {
    fn decode(buf: &mut bytes::BytesMut) -> Result<Self, crate::ProtocolError> {
        if buf.remaining() < 25 {
            return Err(crate::ProtocolError::UnexpectedEof);
        }
        let x = buf.get_f64();
        let y = buf.get_f64();
        let z = buf.get_f64();
        let on_ground = buf.get_u8() != 0;
        Ok(Self { x, y, z, on_ground })
    }
}

/// Player Position and Look packet (client -> server).
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerPositionAndLook {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

impl crate::Encode for PlayerPositionAndLook {
    fn encode(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::ProtocolError> {
        buf.put_f64(self.x);
        buf.put_f64(self.y);
        buf.put_f64(self.z);
        buf.put_f32(self.yaw);
        buf.put_f32(self.pitch);
        buf.put_u8(self.on_ground as u8);
        Ok(())
    }
}

impl<'a> crate::Decode<'a> for PlayerPositionAndLook {
    fn decode(buf: &mut bytes::BytesMut) -> Result<Self, crate::ProtocolError> {
        if buf.remaining() < 33 {
            return Err(crate::ProtocolError::UnexpectedEof);
        }
        let x = buf.get_f64();
        let y = buf.get_f64();
        let z = buf.get_f64();
        let yaw = buf.get_f32();
        let pitch = buf.get_f32();
        let on_ground = buf.get_u8() != 0;
        Ok(Self { x, y, z, yaw, pitch, on_ground })
    }
}

/// Player Look packet (client -> server).
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerLook {
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

impl crate::Encode for PlayerLook {
    fn encode(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::ProtocolError> {
        buf.put_f32(self.yaw);
        buf.put_f32(self.pitch);
        buf.put_u8(self.on_ground as u8);
        Ok(())
    }
}

impl<'a> crate::Decode<'a> for PlayerLook {
    fn decode(buf: &mut bytes::BytesMut) -> Result<Self, crate::ProtocolError> {
        if buf.remaining() < 9 {
            return Err(crate::ProtocolError::UnexpectedEof);
        }
        let yaw = buf.get_f32();
        let pitch = buf.get_f32();
        let on_ground = buf.get_u8() != 0;
        Ok(Self { yaw, pitch, on_ground })
    }
}

/// Player Digging packet (client -> server).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerDigging {
    pub status: DigStatus,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub face: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DigStatus {
    StartedDigging = 0,
    CancelledDigging = 1,
    FinishedDigging = 2,
    DropItemStack = 3,
    DropItem = 4,
    ShootArrow = 5,
    SwapItemInHand = 6,
}

impl crate::Encode for PlayerDigging {
    fn encode(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::ProtocolError> {
        buf.put_u8(self.status as u8);
        buf.put_i32(self.x);
        buf.put_i32(self.y);
        buf.put_i32(self.z);
        buf.put_u8(self.face);
        Ok(())
    }
}

impl<'a> crate::Decode<'a> for PlayerDigging {
    fn decode(buf: &mut bytes::BytesMut) -> Result<Self, crate::ProtocolError> {
        if buf.remaining() < 14 {
            return Err(crate::ProtocolError::UnexpectedEof);
        }
        let status = match buf.get_u8() {
            0 => DigStatus::StartedDigging,
            1 => DigStatus::CancelledDigging,
            2 => DigStatus::FinishedDigging,
            3 => DigStatus::DropItemStack,
            4 => DigStatus::DropItem,
            5 => DigStatus::ShootArrow,
            6 => DigStatus::SwapItemInHand,
            _ => return Err(crate::ProtocolError::DecodeFailed("invalid dig status".into())),
        };
        let x = buf.get_i32();
        let y = buf.get_i32();
        let z = buf.get_i32();
        let face = buf.get_u8();
        Ok(Self { status, x, y, z, face })
    }
}

/// Player Block Placement packet (client -> server).
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerBlockPlacement {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub face: u8,
    pub hand: Hand,
    pub cursor_x: f32,
    pub cursor_y: f32,
    pub cursor_z: f32,
    pub inside_block: bool,
    pub sequence: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Hand {
    MainHand = 0,
    OffHand = 1,
}

impl crate::Encode for PlayerBlockPlacement {
    fn encode(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::ProtocolError> {
        buf.put_i32(self.x);
        buf.put_i32(self.y);
        buf.put_i32(self.z);
        buf.put_u8(self.face);
        buf.put_u8(self.hand as u8);
        buf.put_f32(self.cursor_x);
        buf.put_f32(self.cursor_y);
        buf.put_f32(self.cursor_z);
        buf.put_u8(self.inside_block as u8);
        buf.put_u32(self.sequence);
        Ok(())
    }
}

impl<'a> crate::Decode<'a> for PlayerBlockPlacement {
    fn decode(buf: &mut bytes::BytesMut) -> Result<Self, crate::ProtocolError> {
        if buf.remaining() < 30 {
            return Err(crate::ProtocolError::UnexpectedEof);
        }
        let x = buf.get_i32();
        let y = buf.get_i32();
        let z = buf.get_i32();
        let face = buf.get_u8();
        let hand = match buf.get_u8() {
            0 => Hand::MainHand,
            1 => Hand::OffHand,
            _ => return Err(crate::ProtocolError::DecodeFailed("invalid hand".into())),
        };
        let cursor_x = buf.get_f32();
        let cursor_y = buf.get_f32();
        let cursor_z = buf.get_f32();
        let inside_block = buf.get_u8() != 0;
        let sequence = buf.get_u32();
        Ok(Self {
            x,
            y,
            z,
            face,
            hand,
            cursor_x,
            cursor_y,
            cursor_z,
            inside_block,
            sequence,
        })
    }
}

/// Held Item Change packet (client -> server).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldItemChange {
    pub slot: u16,
}

impl crate::Encode for HeldItemChange {
    fn encode(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::ProtocolError> {
        buf.put_u16(self.slot);
        Ok(())
    }
}

impl<'a> crate::Decode<'a> for HeldItemChange {
    fn decode(buf: &mut bytes::BytesMut) -> Result<Self, crate::ProtocolError> {
        if buf.remaining() < 2 {
            return Err(crate::ProtocolError::UnexpectedEof);
        }
        let slot = buf.get_u16();
        Ok(Self { slot })
    }
}

/// Join Game packet (server -> client).
#[derive(Debug, Clone, PartialEq)]
pub struct JoinGame {
    pub entity_id: i32,
    pub is_hardcore: bool,
    pub dimension: Dimension,
    pub difficulty: Difficulty,
    pub max_players: u8,
    pub level_type: String,
    pub reduced_debug_info: bool,
    pub enable_respawn_screen: bool,
    pub is_debug: bool,
    pub is_flat: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Dimension {
    Overworld = 0,
    TheNether = -1,
    TheEnd = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Difficulty {
    Peaceful = 0,
    Easy = 1,
    Normal = 2,
    Hard = 3,
}

impl crate::Encode for JoinGame {
    fn encode(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::ProtocolError> {
        buf.put_i32(self.entity_id);
        buf.put_u8(self.is_hardcore as u8);
        buf.put_i32(self.dimension as i32);
        buf.put_u8(self.difficulty as u8);
        buf.put_u8(self.max_players);
        crate::write_string(buf, &self.level_type);
        buf.put_u8(self.reduced_debug_info as u8);
        buf.put_u8(self.enable_respawn_screen as u8);
        buf.put_u8(self.is_debug as u8);
        buf.put_u8(self.is_flat as u8);
        Ok(())
    }
}

impl<'a> crate::Decode<'a> for JoinGame {
    fn decode(buf: &mut bytes::BytesMut) -> Result<Self, crate::ProtocolError> {
        // Minimum: 4 (entity_id) + 1 (hardcore) + 4 (dimension) + 1 (difficulty) + 1 (max_players) 
        // + 1 (empty string varint) + 4 (bools) = 16 bytes
        if buf.remaining() < 16 {
            return Err(crate::ProtocolError::UnexpectedEof);
        }
        let entity_id = buf.get_i32();
        let is_hardcore = buf.get_u8() != 0;
        let dimension = match buf.get_i32() {
            0 => Dimension::Overworld,
            -1 => Dimension::TheNether,
            1 => Dimension::TheEnd,
            _ => return Err(crate::ProtocolError::DecodeFailed("invalid dimension".into())),
        };
        let difficulty = match buf.get_u8() {
            0 => Difficulty::Peaceful,
            1 => Difficulty::Easy,
            2 => Difficulty::Normal,
            3 => Difficulty::Hard,
            _ => return Err(crate::ProtocolError::DecodeFailed("invalid difficulty".into())),
        };
        let max_players = buf.get_u8();
        let level_type = crate::read_string(buf)?;
        let reduced_debug_info = buf.get_u8() != 0;
        let enable_respawn_screen = buf.get_u8() != 0;
        let is_debug = buf.get_u8() != 0;
        let is_flat = buf.get_u8() != 0;
        Ok(Self {
            entity_id,
            is_hardcore,
            dimension,
            difficulty,
            max_players,
            level_type,
            reduced_debug_info,
            enable_respawn_screen,
            is_debug,
            is_flat,
        })
    }
}

/// Chunk Data packet (server -> client) - simplified placeholder.
#[derive(Debug, Clone)]
pub struct ChunkData {
    pub x: i32,
    pub z: i32,
    pub data: Vec<u8>,
}

impl crate::Encode for ChunkData {
    fn encode(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::ProtocolError> {
        buf.put_i32(self.x);
        buf.put_i32(self.z);
        crate::write_varint(buf, self.data.len() as i32);
        buf.put_slice(&self.data);
        Ok(())
    }
}

impl<'a> crate::Decode<'a> for ChunkData {
    fn decode(buf: &mut bytes::BytesMut) -> Result<Self, crate::ProtocolError> {
        let x = buf.get_i32();
        let z = buf.get_i32();
        let len = crate::read_varint(buf)? as usize;
        if buf.remaining() < len {
            return Err(crate::ProtocolError::UnexpectedEof);
        }
        let data = buf.copy_to_bytes(len).to_vec();
        Ok(Self { x, z, data })
    }
}

/// Player Info Update packet (server -> client).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerInfoUpdate {
    pub action: PlayerInfoAction,
    pub players: Vec<PlayerInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PlayerInfoAction {
    AddPlayer = 0,
    UpdateGamemode = 1,
    UpdateLatency = 2,
    UpdateDisplayName = 3,
    RemovePlayer = 4,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerInfo {
    pub uuid: Uuid,
    pub name: String,
    pub properties: Vec<ProfileProperty>,
    pub gamemode: GameMode,
    pub latency: i32,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum GameMode {
    Survival = 0,
    Creative = 1,
    Adventure = 2,
    Spectator = 3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileProperty {
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
}

impl crate::Encode for PlayerInfoUpdate {
    fn encode(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::ProtocolError> {
        buf.put_u8(self.action as u8);
        crate::write_varint(buf, self.players.len() as i32);
        for player in &self.players {
            crate::write_uuid(buf, player.uuid);
            crate::write_string(buf, &player.name);
            crate::write_varint(buf, player.properties.len() as i32);
            for prop in &player.properties {
                crate::write_string(buf, &prop.name);
                crate::write_string(buf, &prop.value);
                match &prop.signature {
                    Some(s) => { buf.put_u8(1); crate::write_string(buf, s); }
                    None => buf.put_u8(0),
                }
            }
            buf.put_i32(player.gamemode as i32);
            buf.put_i32(player.latency);
            match &player.display_name {
                Some(name) => { buf.put_u8(1); crate::write_string(buf, name); }
                None => buf.put_u8(0),
            }
        }
        Ok(())
    }
}

impl<'a> crate::Decode<'a> for PlayerInfoUpdate {
    fn decode(buf: &mut bytes::BytesMut) -> Result<Self, crate::ProtocolError> {
        let action = match buf.get_u8() {
            0 => PlayerInfoAction::AddPlayer,
            1 => PlayerInfoAction::UpdateGamemode,
            2 => PlayerInfoAction::UpdateLatency,
            3 => PlayerInfoAction::UpdateDisplayName,
            4 => PlayerInfoAction::RemovePlayer,
            _ => return Err(crate::ProtocolError::DecodeFailed("invalid player info action".into())),
        };
        let count = crate::read_varint(buf)? as usize;
        let mut players = Vec::with_capacity(count);
        for _ in 0..count {
            let uuid = crate::read_uuid(buf)?;
            let name = crate::read_string(buf)?;
            let prop_count = crate::read_varint(buf)? as usize;
            let mut properties = Vec::with_capacity(prop_count);
            for _ in 0..prop_count {
                let name = crate::read_string(buf)?;
                let value = crate::read_string(buf)?;
                let has_sig = buf.get_u8() != 0;
                let signature = if has_sig { Some(crate::read_string(buf)?) } else { None };
                properties.push(ProfileProperty { name, value, signature });
            }
            let gamemode = match buf.get_i32() {
                0 => GameMode::Survival,
                1 => GameMode::Creative,
                2 => GameMode::Adventure,
                3 => GameMode::Spectator,
                _ => return Err(crate::ProtocolError::DecodeFailed("invalid gamemode".into())),
            };
            let latency = buf.get_i32();
            let has_display = buf.get_u8() != 0;
            let display_name = if has_display { Some(crate::read_string(buf)?) } else { None };
            players.push(PlayerInfo { uuid, name, properties, gamemode, latency, display_name });
        }
        Ok(Self { action, players })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Encode, Decode};
    use bytes::BytesMut;
    use uuid::Uuid;

    #[test]
    fn test_keep_alive_roundtrip() {
        let packet = KeepAlive { id: 12345 };
        let mut buf = BytesMut::new();
        packet.encode(&mut buf).unwrap();
        let mut decode_buf = buf.clone();
        let decoded = KeepAlive::decode(&mut decode_buf).unwrap();
        assert_eq!(decoded, packet);
    }

    #[test]
    fn test_player_position_roundtrip() {
        let packet = PlayerPosition { x: 100.5, y: 64.0, z: -50.2, on_ground: true };
        let mut buf = BytesMut::new();
        packet.encode(&mut buf).unwrap();
        let mut decode_buf = buf.clone();
        let decoded = PlayerPosition::decode(&mut decode_buf).unwrap();
        assert_eq!(decoded, packet);
    }

    #[test]
    fn test_chat_message_roundtrip() {
        let packet = ChatMessage { message: "Hello, world!".to_string() };
        let mut buf = BytesMut::new();
        packet.encode(&mut buf).unwrap();
        let mut decode_buf = buf.clone();
        let decoded = ChatMessage::decode(&mut decode_buf).unwrap();
        assert_eq!(decoded, packet);
    }

    #[test]
    fn test_held_item_change_roundtrip() {
        let packet = HeldItemChange { slot: 3 };
        let mut buf = BytesMut::new();
        packet.encode(&mut buf).unwrap();
        let mut decode_buf = buf.clone();
        let decoded = HeldItemChange::decode(&mut decode_buf).unwrap();
        assert_eq!(decoded, packet);
    }

    #[test]
    fn test_player_digging_roundtrip() {
        let packet = PlayerDigging {
            status: DigStatus::StartedDigging,
            x: 10, y: 64, z: 20,
            face: 1,
        };
        let mut buf = BytesMut::new();
        packet.encode(&mut buf).unwrap();
        let mut decode_buf = buf.clone();
        let decoded = PlayerDigging::decode(&mut decode_buf).unwrap();
        assert_eq!(decoded, packet);
    }

    #[test]
    fn test_join_game_roundtrip() {
        let packet = JoinGame {
            entity_id: 1,
            is_hardcore: false,
            dimension: Dimension::Overworld,
            difficulty: Difficulty::Normal,
            max_players: 20,
            level_type: "default".to_string(),
            reduced_debug_info: false,
            enable_respawn_screen: true,
            is_debug: false,
            is_flat: false,
        };
        let mut buf = BytesMut::new();
        packet.encode(&mut buf).unwrap();
        let mut decode_buf = buf.clone();
        let decoded = JoinGame::decode(&mut decode_buf).unwrap();
        assert_eq!(decoded, packet);
    }
}