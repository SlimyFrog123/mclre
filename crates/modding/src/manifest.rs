//! Mod manifest parsing and validation.

use std::path::{Path, PathBuf};

use mclre_core::Identifier;
use mclre_mod_api::{ModDependency, ModMetadata, Permission};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use toml;

/// Errors that can occur when parsing/validating a mod manifest
#[derive(Debug, Error)]
pub enum ModManifestError {
    #[error("Failed to read manifest file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Invalid mod ID format: {0}")]
    InvalidId(String),
    #[error("Invalid version format: {0}")]
    InvalidVersion(String),
    #[error("Invalid dependency: {0}")]
    InvalidDependency(String),
    #[error("Invalid permission: {0}")]
    InvalidPermission(String),
    #[error("Duplicate mod ID: {0}")]
    DuplicateId(String),
    #[error("Missing mod directory: {0}")]
    MissingDirectory(String),
}

/// Raw mod manifest from mod.toml
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RawModManifest {
    /// Mod identifier (namespace:name)
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Version string (semver)
    pub version: String,
    /// Description
    pub description: String,
    /// Authors
    pub authors: Vec<String>,
    /// License (SPDX)
    pub license: String,
    /// Required MCLRE version range
    pub mclre_version: String,
    /// Dependencies
    pub dependencies: Option<Vec<RawModDependency>>,
    /// Permissions requested
    pub permissions: Option<Vec<String>>,
    /// Entry point (WASM module)
    pub entry_point: Option<String>,
    /// Mod configuration schema
    pub config_schema: Option<serde_json::Value>,
    /// Minimum API version
    pub api_version: Option<String>,
}

/// Raw dependency from manifest
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RawModDependency {
    pub id: String,
    pub version: String,
    pub optional: Option<bool>,
}

/// Parsed and validated mod manifest
#[derive(Debug, Clone)]
pub struct ModManifest {
    /// Parsed metadata
    pub metadata: ModMetadata,
    /// Resolved dependencies
    pub dependencies: Vec<ModDependency>,
    /// Permissions granted
    pub permissions: PermissionSet,
    /// Entry point (WASM export name)
    pub entry_point: String,
    /// Configuration schema
    pub config_schema: Option<serde_json::Value>,
    /// Minimum API version required
    pub api_version: String,
    /// Path to mod directory
    pub mod_dir: PathBuf,
    /// Path to WASM file
    pub wasm_path: PathBuf,
}

/// Permission set with validation
#[derive(Debug, Clone, Default)]
pub struct PermissionSet {
    permissions: std::collections::HashSet<Permission>,
}

impl PermissionSet {
    /// Create a new empty permission set
    pub fn new() -> Self {
        Self::default()
    }

    /// Create from a list of permission strings
    pub fn from_strings(perms: &[String]) -> Result<Self, ModManifestError> {
        let mut set = Self::new();
        for perm_str in perms {
            let perm = parse_permission(perm_str)?;
            set.permissions.insert(perm);
        }
        Ok(set)
    }

    /// Check if a permission is granted
    pub fn has(&self, perm: Permission) -> bool {
        self.permissions.contains(&perm)
    }

    /// Get all permissions
    pub fn all(&self) -> impl Iterator<Item = Permission> + '_ {
        self.permissions.iter().copied()
    }
}

/// Parse a permission string
fn parse_permission(s: &str) -> Result<Permission, ModManifestError> {
    match s.to_lowercase().as_str() {
        "world_read" => Ok(Permission::WorldRead),
        "world_write" => Ok(Permission::WorldWrite),
        "player_interact" => Ok(Permission::PlayerInteract),
        "entity_manipulate" => Ok(Permission::EntityManipulate),
        "world_gen" => Ok(Permission::WorldGen),
        "network" => Ok(Permission::Network),
        "config" => Ok(Permission::Config),
        _ => Err(ModManifestError::InvalidPermission(format!(
            "Unknown permission: {}. Valid: world_read, world_write, player_interact, entity_manipulate, world_gen, network, config",
            s
        ))),
    }
}

impl ModManifest {
    /// Load and validate a mod manifest from a directory
    pub fn load_from_dir<P: AsRef<Path>>(mod_dir: P) -> Result<Self, ModManifestError> {
        let mod_dir = mod_dir.as_ref().to_path_buf();

        // Check directory exists
        if !mod_dir.exists() {
            return Err(ModManifestError::MissingDirectory(
                mod_dir.display().to_string(),
            ));
        }

        // Read manifest file
        let manifest_path = mod_dir.join("mod.toml");
        let content = std::fs::read_to_string(&manifest_path)?;
        let raw: RawModManifest = toml::from_str(&content)?;

        // Validate and convert
        Self::from_raw(raw, mod_dir)
    }

    /// Create from raw manifest with validation
    fn from_raw(raw: RawModManifest, mod_dir: PathBuf) -> Result<Self, ModManifestError> {
        // Validate ID
        let id =
            Identifier::parse(&raw.id).map_err(|e| ModManifestError::InvalidId(e.to_string()))?;

        // Validate version
        let version = Version::parse(&raw.version)
            .map_err(|e| ModManifestError::InvalidVersion(e.to_string()))?;

        // Validate MCLRE version requirement
        let _mclre_req = VersionReq::parse(&raw.mclre_version)
            .map_err(|e| ModManifestError::InvalidVersion(e.to_string()))?;

        // Parse dependencies
        let mut dependencies = Vec::new();
        if let Some(raw_deps) = raw.dependencies {
            for raw_dep in raw_deps {
                let dep_id = Identifier::parse(&raw_dep.id)
                    .map_err(|e| ModManifestError::InvalidDependency(e.to_string()))?;
                let _dep_req = VersionReq::parse(&raw_dep.version)
                    .map_err(|e| ModManifestError::InvalidDependency(e.to_string()))?;

                dependencies.push(ModDependency {
                    id: dep_id,
                    version: raw_dep.version,
                    optional: raw_dep.optional.unwrap_or(false),
                });
            }
        }

        // Parse permissions
        let permissions = if let Some(perms) = raw.permissions {
            PermissionSet::from_strings(&perms)?
        } else {
            PermissionSet::new()
        };

        // Determine entry point
        let entry_point = raw.entry_point.unwrap_or_else(|| "mod_init".to_string());

        // API version
        let api_version = raw.api_version.unwrap_or_else(|| "1.0.0".to_string());

        // Find WASM file
        let wasm_path = mod_dir.join("mod.wasm");
        if !wasm_path.exists() {
            // Try alternative names
            let alt_paths = [
                mod_dir.join("mod.wasm"),
                mod_dir.join(format!("{}.wasm", raw.id.replace(':', "_"))),
            ];
            let mut found = false;
            for path in &alt_paths {
                if path.exists() {
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(ModManifestError::MissingDirectory(
                    "WASM file not found (expected mod.wasm)".into(),
                ));
            }
        }

        let metadata = ModMetadata {
            id,
            name: raw.name,
            version: raw.version,
            description: raw.description,
            authors: raw.authors,
            license: raw.license,
            mclre_version: raw.mclre_version,
            dependencies: dependencies.clone(),
        };

        Ok(Self {
            metadata,
            dependencies,
            permissions,
            entry_point,
            config_schema: raw.config_schema,
            api_version,
            mod_dir,
            wasm_path,
        })
    }

    /// Get the mod ID
    pub fn id(&self) -> &Identifier {
        &self.metadata.id
    }

    /// Get the mod name
    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    /// Get the mod version
    pub fn version(&self) -> &Version {
        // We need to store the parsed version
        static PARSED_VERSION: std::sync::OnceLock<Version> = std::sync::OnceLock::new();
        PARSED_VERSION.get_or_init(|| Version::parse(&self.metadata.version).unwrap())
    }

    /// Check if this mod is compatible with a given MCLRE version
    pub fn is_compatible_with(&self, mclre_version: &Version) -> bool {
        VersionReq::parse(&self.metadata.mclre_version)
            .map(|req| req.matches(mclre_version))
            .unwrap_or(false)
    }

    /// Get the mod directory path
    pub fn mod_dir(&self) -> &PathBuf {
        &self.mod_dir
    }

    /// Get the WASM file path
    pub fn wasm_path(&self) -> &PathBuf {
        &self.wasm_path
    }

    /// Get the entry point function name
    pub fn entry_point(&self) -> &str {
        &self.entry_point
    }

    /// Get the required API version
    pub fn api_version(&self) -> &str {
        &self.api_version
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_parse_permission() {
        assert!(matches!(
            parse_permission("world_read"),
            Ok(Permission::WorldRead)
        ));
        assert!(matches!(
            parse_permission("world_write"),
            Ok(Permission::WorldWrite)
        ));
        assert!(matches!(
            parse_permission("player_interact"),
            Ok(Permission::PlayerInteract)
        ));
        assert!(matches!(
            parse_permission("entity_manipulate"),
            Ok(Permission::EntityManipulate)
        ));
        assert!(matches!(
            parse_permission("world_gen"),
            Ok(Permission::WorldGen)
        ));
        assert!(matches!(
            parse_permission("network"),
            Ok(Permission::Network)
        ));
        assert!(matches!(parse_permission("config"), Ok(Permission::Config)));
        assert!(parse_permission("invalid").is_err());
    }

    #[test]
    fn test_permission_set() {
        let perms = vec!["world_read".to_string(), "world_write".to_string()];
        let set = PermissionSet::from_strings(&perms).unwrap();
        assert!(set.has(Permission::WorldRead));
        assert!(set.has(Permission::WorldWrite));
        assert!(!set.has(Permission::PlayerInteract));
    }

    #[test]
    fn test_load_manifest() {
        let dir = tempdir().unwrap();
        let mod_dir = dir.path().join("test_mod");
        fs::create_dir_all(&mod_dir).unwrap();

        let manifest_content = r#"
id = "test:example_mod"
name = "Example Mod"
version = "1.0.0"
description = "A test mod"
authors = ["Test Author"]
license = "MIT"
mclre_version = "^1.0.0"
dependencies = [
    { id = "minecraft:test_dep", version = "^1.0.0", optional = false }
]
permissions = ["world_read", "player_interact"]
entry_point = "mod_init"
api_version = "1.0.0"
"#;
        fs::write(mod_dir.join("mod.toml"), manifest_content).unwrap();
        fs::write(mod_dir.join("mod.wasm"), vec![0, 97, 115, 109]).unwrap(); // Minimal WASM magic

        let manifest = ModManifest::load_from_dir(&mod_dir).unwrap();
        assert_eq!(manifest.metadata.id.to_string(), "test:example_mod");
        assert_eq!(manifest.metadata.name, "Example Mod");
        assert_eq!(manifest.metadata.version, "1.0.0");
        assert_eq!(manifest.dependencies.len(), 1);
        assert!(manifest.permissions.has(Permission::WorldRead));
        assert!(manifest.permissions.has(Permission::PlayerInteract));
        assert_eq!(manifest.entry_point(), "mod_init");
    }

    #[test]
    fn test_manifest_with_optional_deps() {
        let dir = tempdir().unwrap();
        let mod_dir = dir.path().join("test_mod2");
        fs::create_dir_all(&mod_dir).unwrap();

        let manifest_content = r#"
id = "test:mod_with_opt"
name = "Mod With Optional Deps"
version = "2.0.0"
description = "Test"
authors = ["Author"]
license = "MIT"
mclre_version = ">=1.0.0"
dependencies = [
    { id = "minecraft:required", version = "1.0.0", optional = false },
    { id = "minecraft:optional", version = "1.0.0", optional = true }
]
permissions = ["world_read"]
"#;
        fs::write(mod_dir.join("mod.toml"), manifest_content).unwrap();
        fs::write(mod_dir.join("mod.wasm"), vec![0, 97, 115, 109]).unwrap();

        let manifest = ModManifest::load_from_dir(&mod_dir).unwrap();
        assert_eq!(manifest.dependencies.len(), 2);
        assert!(!manifest.dependencies[0].optional);
        assert!(manifest.dependencies[1].optional);
    }
}
