//! Mod registry for tracking loaded mods and their capabilities.

use std::collections::HashMap;
use std::sync::Arc;

use mclre_core::Identifier;
use mclre_mod_api::ModMetadata;
use parking_lot::RwLock;
use thiserror::Error;

use crate::manifest::ModManifest;
use crate::permissions::PermissionManager;

/// Errors that can occur in the mod registry
#[derive(Debug, Error)]
pub enum ModRegistryError {
    #[error("Mod not found: {0}")]
    ModNotFound(String),
    #[error("Mod already registered: {0}")]
    AlreadyRegistered(String),
    #[error("Invalid mod ID: {0}")]
    InvalidId(String),
    #[error("Registry locked")]
    Locked,
}

/// Registry entry for a loaded mod
#[derive(Debug, Clone)]
pub struct ModRegistryEntry {
    /// Mod manifest
    pub manifest: crate::manifest::ModManifest,
    /// Whether the mod is currently active
    pub active: bool,
    /// Load time (Unix timestamp)
    pub load_time: u64,
    /// Mod-specific configuration
    pub config: std::collections::HashMap<String, String>,
}

/// Mod registry for tracking all loaded mods
pub struct ModRegistry {
    /// Registered mods
    mods: RwLock<HashMap<Identifier, ModRegistryEntry>>,
    /// Permission manager
    permissions: Arc<crate::permissions::PermissionManager>,
    /// Event bus for mod communication
    event_bus: Arc<crate::event_bus::EventBus>,
}

impl ModRegistry {
    /// Create a new mod registry
    pub fn new() -> Self {
        Self {
            mods: RwLock::new(HashMap::new()),
            permissions: Arc::new(crate::permissions::PermissionManager::new()),
            event_bus: Arc::new(crate::event_bus::EventBus::new()),
        }
    }

    /// Register a mod
    pub fn register_mod(
        &self,
        manifest: crate::manifest::ModManifest,
    ) -> Result<(), ModRegistryError> {
        let id = manifest.id().clone();
        let mut mods = self.mods.write();

        if mods.contains_key(&id) {
            return Err(ModRegistryError::AlreadyRegistered(id.to_string()));
        }

        let entry = ModRegistryEntry {
            manifest,
            active: true,
            load_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            config: HashMap::new(),
        };

        mods.insert(id, entry);
        Ok(())
    }

    /// Unregister a mod
    pub fn unregister_mod(&self, mod_id: &Identifier) -> Result<(), ModRegistryError> {
        let mut mods = self.mods.write();
        mods.remove(mod_id)
            .ok_or(ModRegistryError::ModNotFound(mod_id.to_string()))?;
        Ok(())
    }

    /// Get a mod entry
    pub fn get_mod(&self, mod_id: &Identifier) -> Option<ModRegistryEntry> {
        self.mods.read().get(mod_id).cloned()
    }

    /// Get mod metadata
    pub fn get_metadata(&self, mod_id: &Identifier) -> Option<ModMetadata> {
        self.mods
            .read()
            .get(mod_id)
            .map(|entry| entry.manifest.metadata.clone())
    }

    /// Get mod manifest
    pub fn get_manifest(&self, mod_id: &Identifier) -> Option<crate::manifest::ModManifest> {
        self.mods
            .read()
            .get(mod_id)
            .map(|entry| entry.manifest.clone())
    }

    /// Check if a mod is registered
    pub fn is_registered(&self, mod_id: &Identifier) -> bool {
        self.mods.read().contains_key(mod_id)
    }

    /// Check if a mod is active
    pub fn is_active(&self, mod_id: &Identifier) -> bool {
        self.mods
            .read()
            .get(mod_id)
            .map(|e| e.active)
            .unwrap_or(false)
    }

    /// Set mod active state
    pub fn set_active(&self, mod_id: &Identifier, active: bool) -> Result<(), ModRegistryError> {
        let mut mods = self.mods.write();
        mods.get_mut(mod_id)
            .ok_or(ModRegistryError::ModNotFound(mod_id.to_string()))?
            .active = active;
        Ok(())
    }

    /// Get all registered mod IDs
    pub fn all_mods(&self) -> Vec<Identifier> {
        self.mods.read().keys().cloned().collect()
    }

    /// Get all active mod IDs
    pub fn active_mods(&self) -> Vec<Identifier> {
        self.mods
            .read()
            .values()
            .filter(|e| e.active)
            .map(|e| e.manifest.id().clone())
            .collect()
    }

    /// Get the permission manager
    pub fn permissions(&self) -> Arc<crate::permissions::PermissionManager> {
        self.permissions.clone()
    }

    /// Get the event bus
    pub fn event_bus(&self) -> Arc<crate::event_bus::EventBus> {
        self.event_bus.clone()
    }

    /// Set mod configuration value
    pub fn set_config(
        &self,
        mod_id: &Identifier,
        key: String,
        value: String,
    ) -> Result<(), ModRegistryError> {
        self.mods
            .write()
            .get_mut(mod_id)
            .ok_or(ModRegistryError::ModNotFound(mod_id.to_string()))?
            .config
            .insert(key, value);
        Ok(())
    }

    /// Get mod configuration value
    pub fn get_config(&self, mod_id: &Identifier, key: &str) -> Option<String> {
        self.mods
            .read()
            .get(mod_id)
            .and_then(|e| e.config.get(key).cloned())
    }

    /// Get all configuration for a mod
    pub fn get_all_config(&self, mod_id: &Identifier) -> Option<HashMap<String, String>> {
        self.mods.read().get(mod_id).map(|e| e.config.clone())
    }

    /// Get mod count
    pub fn mod_count(&self) -> usize {
        self.mods.read().len()
    }

    /// Get active mod count
    pub fn active_mod_count(&self) -> usize {
        self.mods.read().values().filter(|e| e.active).count()
    }
}

impl Default for ModRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ModRegistry {
    fn clone(&self) -> Self {
        Self {
            mods: RwLock::new(self.mods.read().clone()),
            permissions: self.permissions.clone(),
            event_bus: self.event_bus.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::PermissionSet;
    use mclre_core::Identifier;
    use mclre_mod_api::ModMetadata;
    use semver::Version;

    fn create_test_manifest(id: &str) -> crate::manifest::ModManifest {
        let identifier = Identifier::parse(id).unwrap();
        let metadata = ModMetadata {
            id: identifier.clone(),
            name: format!("Mod {}", id),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            authors: vec!["Test".to_string()],
            license: "MIT".to_string(),
            mclre_version: "^1.0.0".to_string(),
            dependencies: vec![],
        };

        crate::manifest::ModManifest {
            metadata,
            dependencies: vec![],
            permissions: PermissionSet::new(),
            entry_point: "mod_init".to_string(),
            config_schema: None,
            api_version: "1.0.0".to_string(),
            mod_dir: std::path::PathBuf::from("/tmp/test"),
            wasm_path: std::path::PathBuf::from("/tmp/test/mod.wasm"),
        }
    }

    #[test]
    fn test_register_mod() {
        let registry = ModRegistry::new();
        let manifest = create_test_manifest("test:mod1");

        assert!(registry.register_mod(manifest.clone()).is_ok());
        assert!(registry.is_registered(&Identifier::parse("test:mod1").unwrap()));
        assert!(
            registry
                .get_manifest(&Identifier::parse("test:mod1").unwrap())
                .is_some()
        );
    }

    #[test]
    fn test_duplicate_registration() {
        let registry = ModRegistry::new();
        let manifest = create_test_manifest("test:mod1");

        assert!(registry.register_mod(manifest.clone()).is_ok());
        assert!(registry.register_mod(manifest).is_err());
    }

    #[test]
    fn test_unregister_mod() {
        let registry = ModRegistry::new();
        let manifest = create_test_manifest("test:mod1");

        registry.register_mod(manifest).unwrap();
        assert!(registry.is_registered(&Identifier::parse("test:mod1").unwrap()));

        registry
            .unregister_mod(&Identifier::parse("test:mod1").unwrap())
            .unwrap();
        assert!(!registry.is_registered(&Identifier::parse("test:mod1").unwrap()));
    }

    #[test]
    fn test_active_state() {
        let registry = ModRegistry::new();
        let manifest = create_test_manifest("test:mod1");

        registry.register_mod(manifest).unwrap();
        let id = Identifier::parse("test:mod1").unwrap();

        assert!(registry.is_active(&id));
        registry.set_active(&id, false).unwrap();
        assert!(!registry.is_active(&id));
        registry.set_active(&id, true).unwrap();
        assert!(registry.is_active(&id));
    }

    #[test]
    fn test_config() {
        let registry = ModRegistry::new();
        let manifest = create_test_manifest("test:mod1");
        let id = Identifier::parse("test:mod1").unwrap();

        registry.register_mod(manifest).unwrap();

        registry
            .set_config(&id, "key1".to_string(), "value1".to_string())
            .unwrap();
        assert_eq!(registry.get_config(&id, "key1"), Some("value1".to_string()));
        assert_eq!(registry.get_config(&id, "nonexistent"), None);

        let all = registry.get_all_config(&id).unwrap();
        assert_eq!(all.get("key1"), Some(&"value1".to_string()));
    }

    #[test]
    fn test_active_mods() {
        let registry = ModRegistry::new();

        let manifest1 = create_test_manifest("test:mod1");
        let manifest2 = create_test_manifest("test:mod2");

        registry.register_mod(manifest1).unwrap();
        registry.register_mod(manifest2).unwrap();

        let id1 = Identifier::parse("test:mod1").unwrap();
        let id2 = Identifier::parse("test:mod2").unwrap();

        assert_eq!(registry.active_mod_count(), 2);
        assert_eq!(registry.active_mods().len(), 2);

        registry.set_active(&id1, false).unwrap();
        assert_eq!(registry.active_mod_count(), 1);
        assert_eq!(registry.active_mods().len(), 1);
        assert!(registry.active_mods().contains(&id2));
    }
}
