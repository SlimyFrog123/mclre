//! Permission management for mods.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use mclre_core::Identifier;
use mclre_mod_api::Permission;
use thiserror::Error;

/// Errors related to permissions
#[derive(Debug, Error)]
pub enum PermissionError {
    #[error("Permission denied: mod {mod_id} lacks {permission:?}")]
    Denied {
        mod_id: String,
        permission: Permission,
    },
    #[error("Mod not found: {0}")]
    ModNotFound(String),
    #[error("Permission not granted: {0:?}")]
    NotGranted(Permission),
}

/// Permission manager for tracking mod permissions
#[derive(Debug)]
pub struct PermissionManager {
    /// Mod ID -> granted permissions
    permissions: RwLock<HashMap<String, std::collections::HashSet<Permission>>>,
}

impl PermissionManager {
    /// Create a new permission manager
    pub fn new() -> Self {
        Self {
            permissions: RwLock::new(HashMap::new()),
        }
    }

    /// Grant permissions to a mod
    pub fn grant_permissions(&self, mod_id: &Identifier, permissions: &[Permission]) {
        let mut perms = self
            .permissions
            .write()
            .expect("Permission RwLock poisoned");
        let entry = perms.entry(mod_id.to_string()).or_default();
        for perm in permissions {
            entry.insert(*perm);
        }
    }

    /// Revoke permissions from a mod
    pub fn revoke_permissions(&self, mod_id: &Identifier, permissions: &[Permission]) {
        let mut perms = self
            .permissions
            .write()
            .expect("Permission RwLock poisoned");
        if let Some(entry) = perms.get_mut(&mod_id.to_string()) {
            for perm in permissions {
                entry.remove(perm);
            }
        }
    }

    /// Check if a mod has a permission
    pub fn has_permission(&self, mod_id: &Identifier, permission: Permission) -> bool {
        self.permissions
            .read()
            .expect("Permission RwLock poisoned")
            .get(&mod_id.to_string())
            .map(|perms| perms.contains(&permission))
            .unwrap_or(false)
    }

    /// Get all permissions for a mod
    pub fn get_permissions(&self, mod_id: &Identifier) -> Vec<Permission> {
        self.permissions
            .read()
            .expect("Permission RwLock poisoned")
            .get(&mod_id.to_string())
            .map(|perms| perms.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Grant all permissions to a mod (for trusted mods)
    pub fn grant_all(&self, mod_id: &Identifier) {
        let all_perms = [
            Permission::WorldRead,
            Permission::WorldWrite,
            Permission::PlayerInteract,
            Permission::EntityManipulate,
            Permission::WorldGen,
            Permission::Network,
            Permission::Config,
        ];
        self.grant_permissions(mod_id, &all_perms);
    }

    /// Revoke all permissions from a mod
    pub fn revoke_all(&self, mod_id: &Identifier) {
        self.permissions
            .write()
            .expect("Permission RwLock poisoned")
            .remove(&mod_id.to_string());
    }

    /// Check permission and return error if denied
    pub fn check_permission(
        &self,
        mod_id: &Identifier,
        permission: Permission,
    ) -> Result<(), crate::PermissionError> {
        if self.has_permission(mod_id, permission) {
            Ok(())
        } else {
            Err(crate::PermissionError::Denied {
                mod_id: mod_id.to_string(),
                permission,
            })
        }
    }

    /// Remove all permissions for a mod (on unload)
    pub fn remove_mod(&self, mod_id: &Identifier) {
        self.permissions
            .write()
            .expect("Permission RwLock poisoned")
            .remove(&mod_id.to_string());
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Permission set with convenient methods
#[derive(Debug, Clone)]
pub struct PermissionSet {
    permissions: std::collections::HashSet<Permission>,
}

impl PermissionSet {
    /// Create a new empty permission set
    pub fn new() -> Self {
        Self {
            permissions: std::collections::HashSet::new(),
        }
    }

    /// Create from a list of permissions
    pub fn from_permissions(perms: &[Permission]) -> Self {
        let mut set = Self::new();
        for perm in perms {
            set.permissions.insert(*perm);
        }
        set
    }

    /// Check if a permission is granted
    pub fn has(&self, perm: Permission) -> bool {
        self.permissions.contains(&perm)
    }

    /// Add a permission
    pub fn insert(&mut self, perm: Permission) {
        self.permissions.insert(perm);
    }

    /// Remove a permission
    pub fn remove(&mut self, perm: Permission) {
        self.permissions.remove(&perm);
    }

    /// Get all permissions
    pub fn all(&self) -> impl Iterator<Item = Permission> + '_ {
        self.permissions.iter().copied()
    }

    /// Union with another set
    pub fn union(&self, other: &Self) -> Self {
        let mut result = self.clone();
        result.permissions.extend(other.permissions.iter().copied());
        result
    }

    /// Intersection with another set
    pub fn intersection(&self, other: &Self) -> Self {
        let mut result = Self::new();
        for perm in self.permissions.intersection(&other.permissions) {
            result.permissions.insert(*perm);
        }
        result
    }

    /// Difference with another set
    pub fn difference(&self, other: &Self) -> Self {
        let mut result = Self::new();
        for perm in self.permissions.difference(&other.permissions) {
            result.permissions.insert(*perm);
        }
        result
    }
}

impl Default for PermissionSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mclre_core::Identifier;
    use mclre_mod_api::Permission;

    #[test]
    fn test_permission_manager() {
        let manager = PermissionManager::new();
        let mod_id = Identifier::parse("test:mod").unwrap();

        // Initially no permissions
        assert!(!manager.has_permission(&mod_id, Permission::WorldRead));

        // Grant permission
        manager.grant_permissions(&mod_id, &[Permission::WorldRead]);
        assert!(manager.has_permission(&mod_id, Permission::WorldRead));

        // Check another permission
        assert!(!manager.has_permission(&mod_id, Permission::WorldWrite));

        // Grant multiple
        manager.grant_permissions(
            &mod_id,
            &[Permission::WorldWrite, Permission::PlayerInteract],
        );
        assert!(manager.has_permission(&mod_id, Permission::WorldWrite));
        assert!(manager.has_permission(&mod_id, Permission::PlayerInteract));

        // Revoke
        manager.revoke_permissions(&mod_id, &[Permission::WorldRead]);
        assert!(!manager.has_permission(&mod_id, Permission::WorldRead));
        assert!(manager.has_permission(&mod_id, Permission::WorldWrite));
    }

    #[test]
    fn test_permission_set() {
        let mut set = crate::PermissionSet::new();
        assert!(!set.has(Permission::WorldRead));

        set.insert(Permission::WorldRead);
        assert!(set.has(Permission::WorldRead));

        set.insert(Permission::WorldWrite);
        assert!(set.has(Permission::WorldWrite));

        set.remove(Permission::WorldRead);
        assert!(!set.has(Permission::WorldRead));
        assert!(set.has(Permission::WorldWrite));

        let perms: Vec<_> = set.all().collect();
        assert_eq!(perms.len(), 1);
    }

    #[test]
    fn test_permission_set_operations() {
        let mut set1 = crate::PermissionSet::from_permissions(&[
            Permission::WorldRead,
            Permission::WorldWrite,
        ]);
        let set2 = crate::PermissionSet::from_permissions(&[
            Permission::WorldWrite,
            Permission::PlayerInteract,
        ]);

        let union = set1.union(&set2);
        assert!(union.has(Permission::WorldRead));
        assert!(union.has(Permission::WorldWrite));
        assert!(union.has(Permission::PlayerInteract));

        let intersection = set1.intersection(&set2);
        assert!(!intersection.has(Permission::WorldRead));
        assert!(intersection.has(Permission::WorldWrite));
        assert!(!intersection.has(Permission::PlayerInteract));

        let diff = set1.difference(&set2);
        assert!(diff.has(Permission::WorldRead));
        assert!(!diff.has(Permission::WorldWrite));
    }
}
