//! Fuzz target for protocol decoder - M9 acceptance criteria
//!
//! This fuzz target tests the protocol packet decoding to ensure
//! robustness against malformed input.

#![no_main]
use libfuzzer_sys::fuzz_target;
use mclre_protocol::{Decode, ProtocolError};

fuzz_target!(|data: &[u8]| {
    // Try to decode as various packet types
    // We'll test a few key packet types that are commonly exposed to network input
    
    // Test HandshakePacket
    if let Ok((_, _)) = mclre_protocol::handshake::HandshakePacket::decode(data) {
        // Successfully decoded
    }
    
    // Test LoginStart
    if let Ok((_, _)) = mclre_protocol::login::LoginStart::decode(data) {
        // Successfully decoded
    }
    
    // Test SetCompression
    if let Ok((_, _)) = mclre_protocol::login::SetCompression::decode(data) {
        // Successfully decoded
    }
    
    // Test KeepAlive (play state)
    if let Ok((_, _)) = mclre_protocol::play::KeepAlive::decode(data) {
        // Successfully decoded
    }
    
    // Test PlayerPosition
    if let Ok((_, _)) = mclre_protocol::play::PlayerPosition::decode(data) {
        // Successfully decoded
    }
    
    // Test PlayerPositionAndLook
    if let Ok((_, _)) = mclre_protocol::play::PlayerPositionAndLook::decode(data) {
        // Successfully decoded
    }
    
    // Test PlayerDigging
    if let Ok((_, _)) = mclre_protocol::play::PlayerDigging::decode(data) {
        // Successfully decoded
    }
    
    // Test PlayerBlockPlacement
    if let Ok((_, _)) = mclre_protocol::play::PlayerBlockPlacement::decode(data) {
        // Successfully decoded
    }
    
    // Test HeldItemChange
    if let Ok((_, _)) = mclre_protocol::play::HeldItemChange::decode(data) {
        // Successfully decoded
    }
    
    // Test ChatMessage
    if let Ok((_, _)) = mclre_protocol::play::ChatMessage::decode(data) {
        // Successfully decoded
    }
});