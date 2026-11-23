use std::fmt;
use uuid::Uuid;

/// A unique identifier for a local player slot (0..3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalSlot(u8);

impl LocalSlot {
    pub const PRIMARY: LocalSlot = LocalSlot(0);
    pub const MAX_SLOTS: u8 = 4;

    pub fn new(slot: u8) -> Result<Self, IdentityError> {
        if slot >= Self::MAX_SLOTS {
            return Err(IdentityError::InvalidSlot(slot));
        }
        Ok(LocalSlot(slot))
    }

    pub fn as_u8(self) -> u8 {
        self.0
    }

    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for LocalSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "slot{}", self.0)
    }
}

/// Unique identity for a player (local or remote).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlayerIdentity {
    pub uuid: Uuid,
    pub username: String,
    pub local_slot: Option<LocalSlot>,
    pub is_guest: bool,
}

impl PlayerIdentity {
    pub fn new(uuid: Uuid, username: String) -> Self {
        Self {
            uuid,
            username,
            local_slot: None,
            is_guest: false,
        }
    }

    pub fn primary(username: String) -> Self {
        let uuid = Uuid::now_v7();
        Self::new(uuid, username)
    }

    pub fn guest(primary: &PlayerIdentity, slot: LocalSlot) -> Self {
        let uuid = Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            format!("{}-{}", primary.uuid, slot).as_bytes(),
        );
        let username = format!("{}-{}", primary.username, slot);
        Self {
            uuid,
            username,
            local_slot: Some(slot),
            is_guest: true,
        }
    }

    pub fn is_primary(&self) -> bool {
        self.local_slot == Some(LocalSlot::PRIMARY) || self.local_slot.is_none()
    }
}

/// Trait for providing player identity from launch arguments.
pub trait IdentityProvider: Send + Sync {
    fn primary(&self) -> Result<PlayerIdentity, IdentityError>;
    fn guest(
        &self,
        primary: &PlayerIdentity,
        slot: LocalSlot,
    ) -> Result<PlayerIdentity, IdentityError>;
}

/// Identity provider backed by command-line arguments.
#[derive(Debug, Clone)]
pub struct LaunchArgIdentity {
    pub username: String,
    pub uuid: Option<Uuid>,
}

impl LaunchArgIdentity {
    pub fn new(username: String, uuid: Option<Uuid>) -> Self {
        Self { username, uuid }
    }

    pub fn from_env() -> Self {
        let username = std::env::var("MCLRE_USERNAME").unwrap_or_else(|_| "Player".to_string());
        let uuid = std::env::var("MCLRE_UUID")
            .ok()
            .and_then(|s| Uuid::parse_str(&s).ok());
        Self::new(username, uuid)
    }

    pub fn dev_defaults() -> Self {
        Self::new("DevPlayer".to_string(), Some(Uuid::now_v7()))
    }
}

impl IdentityProvider for LaunchArgIdentity {
    fn primary(&self) -> Result<PlayerIdentity, IdentityError> {
        if self.username.is_empty() {
            return Err(IdentityError::EmptyUsername);
        }
        let uuid = self.uuid.unwrap_or_else(Uuid::now_v7);
        Ok(PlayerIdentity::new(uuid, self.username.clone()))
    }

    fn guest(
        &self,
        primary: &PlayerIdentity,
        slot: LocalSlot,
    ) -> Result<PlayerIdentity, IdentityError> {
        Ok(PlayerIdentity::guest(primary, slot))
    }
}

/// Identity provider that refuses all identities (for production multiplayer).
#[derive(Debug, Clone)]
pub struct RefusingIdentityProvider;

impl IdentityProvider for RefusingIdentityProvider {
    fn primary(&self) -> Result<PlayerIdentity, IdentityError> {
        Err(IdentityError::Refused)
    }

    fn guest(
        &self,
        _primary: &PlayerIdentity,
        _slot: LocalSlot,
    ) -> Result<PlayerIdentity, IdentityError> {
        Err(IdentityError::Refused)
    }
}

/// Trait for checking entitlements (skin, capes, DLC, etc.).
pub trait EntitlementProvider: Send + Sync {
    fn has_skin(&self, uuid: Uuid) -> bool;
    fn has_cape(&self, uuid: Uuid) -> bool;
    fn has_dlc(&self, uuid: Uuid, dlc_id: &str) -> bool;
}

/// Accept-all entitlement provider (for development/singleplayer).
#[derive(Debug, Clone)]
pub struct AcceptAllEntitlements;

impl EntitlementProvider for AcceptAllEntitlements {
    fn has_skin(&self, _uuid: Uuid) -> bool {
        true
    }
    fn has_cape(&self, _uuid: Uuid) -> bool {
        true
    }
    fn has_dlc(&self, _uuid: Uuid, _dlc_id: &str) -> bool {
        true
    }
}

/// Refusing entitlement provider (for production).
#[derive(Debug, Clone)]
pub struct RefusingEntitlements;

impl EntitlementProvider for RefusingEntitlements {
    fn has_skin(&self, _uuid: Uuid) -> bool {
        false
    }
    fn has_cape(&self, _uuid: Uuid) -> bool {
        false
    }
    fn has_dlc(&self, _uuid: Uuid, _dlc_id: &str) -> bool {
        false
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("empty username")]
    EmptyUsername,
    #[error("invalid local slot: {0}")]
    InvalidSlot(u8),
    #[error("identity refused (no credentials provided)")]
    Refused,
    #[error("invalid UUID: {0}")]
    InvalidUuid(#[from] uuid::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_slot_valid() {
        assert!(LocalSlot::new(0).is_ok());
        assert!(LocalSlot::new(3).is_ok());
        assert!(LocalSlot::new(4).is_err());
        assert!(LocalSlot::new(255).is_err());
    }

    #[test]
    fn test_primary_identity() {
        let provider = LaunchArgIdentity::new("TestUser".to_string(), Some(Uuid::nil()));
        let identity = provider.primary().unwrap();
        assert_eq!(identity.username, "TestUser");
        assert!(identity.is_primary());
    }

    #[test]
    fn test_guest_identity() {
        let provider = LaunchArgIdentity::new("TestUser".to_string(), Some(Uuid::nil()));
        let primary = provider.primary().unwrap();
        let guest = provider
            .guest(&primary, LocalSlot::new(1).unwrap())
            .unwrap();
        assert_eq!(guest.username, "TestUser-slot1");
        assert!(guest.is_guest);
        assert!(!guest.is_primary());
        assert_eq!(guest.local_slot, Some(LocalSlot::new(1).unwrap()));
    }

    #[test]
    fn test_guest_uuid_derivation() {
        let primary_uuid = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
        let primary = PlayerIdentity::new(primary_uuid, "Primary".to_string());
        let guest = PlayerIdentity::guest(&primary, LocalSlot::new(1).unwrap());
        // UUID should be deterministically derived
        let expected = Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            "00000000-0000-0000-0000-000000000000-slot1".as_bytes(),
        );
        assert_eq!(guest.uuid, expected);
    }

    #[test]
    fn test_refusing_provider() {
        let provider = RefusingIdentityProvider;
        assert!(provider.primary().is_err());
        assert!(
            provider
                .guest(
                    &PlayerIdentity::new(Uuid::nil(), "test".to_string()),
                    LocalSlot::PRIMARY
                )
                .is_err()
        );
    }

    #[test]
    fn test_accept_all_entitlements() {
        let provider = AcceptAllEntitlements;
        let uuid = Uuid::now_v7();
        assert!(provider.has_skin(uuid));
        assert!(provider.has_cape(uuid));
        assert!(provider.has_dlc(uuid, "some_dlc"));
    }

    #[test]
    fn test_refusing_entitlements() {
        let provider = RefusingEntitlements;
        let uuid = Uuid::now_v7();
        assert!(!provider.has_skin(uuid));
        assert!(!provider.has_cape(uuid));
        assert!(!provider.has_dlc(uuid, "some_dlc"));
    }

    #[test]
    fn test_local_slot_display() {
        assert_eq!(LocalSlot::PRIMARY.to_string(), "slot0");
        assert_eq!(LocalSlot::new(3).unwrap().to_string(), "slot3");
    }
}
