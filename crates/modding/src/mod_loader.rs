//! Mod loading and lifecycle management.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use mclre_core::Identifier;
use thiserror::Error;
use tracing::{debug, error, info, warn};

use crate::dependency::{DependencyError, DependencyResolver, ResolvedDependencies};
use crate::event_bus::EventBus;
use crate::manifest::{ModManifest, ModManifestError};
use crate::mod_registry::{ModRegistry, ModRegistryError};
use crate::wasm_host::{WasmError, WasmHost, WasmHostConfig};

/// Errors that can occur during mod loading
#[derive(Debug, Error)]
pub enum ModLoaderError {
    #[error("Manifest error: {0}")]
    Manifest(#[from] ModManifestError),
    #[error("Dependency error: {0}")]
    Dependency(#[from] DependencyError),
    #[error("WASM error: {0}")]
    Wasm(#[from] WasmError),
    #[error("Registry error: {0}")]
    Registry(#[from] ModRegistryError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Mod directory not found: {0}")]
    ModDirectoryNotFound(String),
    #[error("Mod already loaded: {0}")]
    AlreadyLoaded(String),
    #[error("Mod not loaded: {0}")]
    NotLoaded(String),
    #[error("Incompatible MCLRE version: required {required}, current {current}")]
    IncompatibleVersion { required: String, current: String },
    #[error("Failed to load mod: {0}")]
    LoadFailed(String),
}

/// Mod loader for managing mod lifecycle
pub struct ModLoader {
    /// Mods directory
    mods_dir: PathBuf,
    /// Dependency resolver
    resolver: crate::dependency::DependencyResolver,
    /// WASM host
    wasm_host: WasmHost,
    /// Event bus
    event_bus: Arc<EventBus>,
    /// Mod registry
    registry: ModRegistry,
    /// Loaded manifests
    manifests: HashMap<Identifier, ModManifest>,
    /// MCLRE version
    mclre_version: semver::Version,
    /// Server random for deterministic operations
    server_random: mclre_core::GameRandom,
}

impl ModLoader {
    /// Create a new mod loader
    pub fn new(
        mods_dir: PathBuf,
        mclre_version: semver::Version,
        server_random: mclre_core::GameRandom,
    ) -> Result<Self, ModLoaderError> {
        let wasm_host = WasmHost::new(WasmHostConfig::default())?;
        let event_bus = Arc::new(EventBus::new());
        let registry = ModRegistry::new();

        Ok(Self {
            mods_dir,
            resolver: crate::dependency::DependencyResolver::new(),
            wasm_host,
            event_bus,
            registry,
            manifests: HashMap::new(),
            mclre_version,
            server_random,
        })
    }

    /// Discover and load all mods in the mods directory
    pub fn discover_and_load_all(&mut self) -> Result<Vec<Identifier>, ModLoaderError> {
        // Scan mods directory
        let entries = std::fs::read_dir(&self.mods_dir)
            .map_err(|e| ModLoaderError::ModDirectoryNotFound(e.to_string()))?;

        let mut manifests = Vec::new();

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                match ModManifest::load_from_dir(&path) {
                    Ok(manifest) => {
                        manifests.push(manifest);
                    }
                    Err(e) => {
                        warn!("Failed to load manifest from {}: {}", path.display(), e);
                    }
                }
            }
        }

        // Register all manifests with resolver
        for manifest in &manifests {
            self.resolver.add_mod(
                manifest.id().clone(),
                manifest.version().clone(),
                manifest.metadata.clone(),
            );
        }

        // Check MCLRE version compatibility
        let incompatible = self.resolver.check_compatibility(&self.mclre_version);
        for msg in &incompatible {
            warn!("Mod compatibility issue: {}", msg);
        }

        // Resolve dependencies for all mods
        let mod_ids: Vec<_> = manifests.iter().map(|m| m.id().clone()).collect();
        let resolved = self.resolver.resolve(&mod_ids)?;

        // Load mods in dependency order
        let mut loaded = Vec::new();
        for mod_id in &resolved.load_order {
            if let Some(manifest) = manifests.iter().find(|m| m.id() == mod_id) {
                match self.load_mod(manifest) {
                    Ok(_) => {
                        loaded.push(mod_id.clone());
                        info!("Loaded mod: {}", mod_id);
                    }
                    Err(e) => {
                        error!("Failed to load mod {}: {}", mod_id, e);
                        // Continue loading other mods
                    }
                }
            }
        }

        Ok(loaded)
    }

    /// Load a single mod from its manifest
    pub fn load_mod(&mut self, manifest: &ModManifest) -> Result<(), ModLoaderError> {
        let id = manifest.id().clone();

        // Check if already loaded
        if self.manifests.contains_key(&id) {
            return Err(ModLoaderError::AlreadyLoaded(id.to_string()));
        }

        // Check MCLRE version compatibility
        let req = semver::VersionReq::parse(&manifest.metadata.mclre_version)
            .map_err(|e| ModLoaderError::LoadFailed(e.to_string()))?;

        if !req.matches(&self.mclre_version) {
            return Err(ModLoaderError::IncompatibleVersion {
                required: manifest.metadata.mclre_version.clone(),
                current: self.mclre_version.to_string(),
            });
        }

        // Load WASM module
        self.wasm_host
            .load_mod(manifest)
            .map_err(|e| ModLoaderError::Wasm(e))?;

        // Register with registry
        self.registry.register_mod(manifest.clone())?;

        // Store manifest
        self.manifests.insert(id.clone(), manifest.clone());

        // Initialize mod (call init function)
        self.wasm_host
            .call_function(&id.to_string(), "mod_init", &[])?;

        Ok(())
    }

    /// Unload a mod
    pub fn unload_mod(&mut self, mod_id: &Identifier) -> Result<(), ModLoaderError> {
        // Check if loaded
        if !self.manifests.contains_key(mod_id) {
            return Err(ModLoaderError::NotLoaded(mod_id.to_string()));
        }

        // Call shutdown function if it exists
        let _ = self
            .wasm_host
            .call_function(&mod_id.to_string(), "mod_shutdown", &[]);

        // Unload from WASM host
        self.wasm_host.unload_mod(&mod_id.to_string())?;

        // Unregister from registry
        self.registry.unregister_mod(mod_id)?;

        // Unsubscribe from events
        self.event_bus.unsubscribe_all(&mod_id.to_string());

        // Remove manifest
        self.manifests.remove(mod_id);

        info!("Unloaded mod: {}", mod_id);
        Ok(())
    }

    /// Reload a mod (unload then load)
    pub fn reload_mod(&mut self, mod_id: &Identifier) -> Result<(), ModLoaderError> {
        let manifest = self
            .manifests
            .get(mod_id)
            .cloned()
            .ok_or_else(|| ModLoaderError::NotLoaded(mod_id.to_string()))?;

        self.unload_mod(mod_id)?;
        self.load_mod(&manifest)
    }

    /// Get a loaded mod's manifest
    pub fn get_manifest(&self, mod_id: &Identifier) -> Option<&ModManifest> {
        self.manifests.get(mod_id)
    }

    /// Get all loaded mod IDs
    pub fn loaded_mods(&self) -> Vec<Identifier> {
        self.manifests.keys().cloned().collect()
    }

    /// Check if a mod is loaded
    pub fn is_loaded(&self, mod_id: &Identifier) -> bool {
        self.manifests.contains_key(mod_id)
    }

    /// Get the event bus
    pub fn event_bus(&self) -> Arc<EventBus> {
        self.event_bus.clone()
    }

    /// Get the WASM host
    pub fn wasm_host(&self) -> &WasmHost {
        &self.wasm_host
    }

    /// Get the mod registry
    pub fn registry(&self) -> &ModRegistry {
        &self.registry
    }

    /// Tick all loaded mods
    pub fn tick_all(&mut self) -> Result<(), ModLoaderError> {
        self.wasm_host.tick_all()?;
        Ok(())
    }

    /// Get the MCLRE version
    pub fn mclre_version(&self) -> &semver::Version {
        &self.mclre_version
    }
}

impl Default for ModLoader {
    fn default() -> Self {
        // This is just for testing - real usage requires proper initialization
        Self::new(
            PathBuf::from("mods"),
            semver::Version::parse("1.0.0").unwrap(),
            mclre_core::GameRandom::with_seed(0),
        )
        .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_mod_loader_creation() {
        let dir = tempdir().unwrap();
        let loader = ModLoader::new(
            dir.path().to_path_buf(),
            semver::Version::parse("1.0.0").unwrap(),
            mclre_core::GameRandom::with_seed(0),
        );
        loader.expect("ModLoader::new should succeed");
    }
}
