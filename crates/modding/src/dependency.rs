//! Dependency resolution for mods.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use mclre_core::Identifier;
use mclre_mod_api::{ModDependency, ModMetadata};
use semver::{Version, VersionReq};
use thiserror::Error;

/// Errors that can occur during dependency resolution
#[derive(Debug, Error)]
pub enum DependencyError {
    #[error("Missing dependency: {0} (required by {1})")]
    MissingDependency(String, String),
    #[error("Version conflict for {0}: {1}")]
    VersionConflict(String, String),
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),
    #[error("Invalid version requirement: {0}")]
    InvalidVersionReq(String),
    #[error("Mod not found: {0}")]
    ModNotFound(String),
    #[error("Incompatible mod: {0} requires MCLRE {1}")]
    IncompatibleMod(String, String),
}

/// Result of dependency resolution
#[derive(Debug, Clone)]
pub struct ResolvedDependencies {
    /// Load order (topologically sorted)
    pub load_order: Vec<Identifier>,
    /// Map of mod ID to its resolved version
    pub versions: HashMap<Identifier, String>,
    /// Dependencies for each mod
    pub dependencies: HashMap<Identifier, Vec<ModDependency>>,
}

/// Dependency resolver
pub struct DependencyResolver {
    /// Available mods (ID -> version)
    available_mods: HashMap<Identifier, Version>,
    /// Mod metadata (for version requirements)
    mod_metadata: HashMap<Identifier, ModMetadata>,
}

impl DependencyResolver {
    /// Create a new dependency resolver
    pub fn new() -> Self {
        Self {
            available_mods: HashMap::new(),
            mod_metadata: HashMap::new(),
        }
    }

    /// Add a mod to the resolver
    pub fn add_mod(&mut self, id: Identifier, version: Version, metadata: ModMetadata) {
        self.available_mods.insert(id.clone(), version);
        self.mod_metadata.insert(id, metadata);
    }

    /// Remove a mod from the resolver
    pub fn remove_mod(&mut self, id: &Identifier) {
        self.available_mods.remove(id);
        self.mod_metadata.remove(id);
    }

    /// Resolve dependencies for a set of mods to load
    pub fn resolve(&self, to_load: &[Identifier]) -> Result<ResolvedDependencies, DependencyError> {
        let mut graph: BTreeMap<Identifier, BTreeSet<Identifier>> = BTreeMap::new();
        let mut in_degree: BTreeMap<Identifier, usize> = BTreeMap::new();
        let mut all_mods = BTreeSet::new();

        // Build dependency graph (edge: dependency -> dependent).
        for mod_id in to_load {
            self.collect_dependencies(mod_id, &mut graph, &mut in_degree, &mut all_mods)?;
        }

        // Topological sort (Kahn's algorithm). Ordered containers keep the
        // result deterministic across runs and platforms.
        let mut queue: VecDeque<Identifier> = all_mods
            .iter()
            .filter(|id| in_degree.get(*id).copied().unwrap_or(0) == 0)
            .copied()
            .collect();

        let mut load_order = Vec::new();
        let mut processed = BTreeSet::new();

        while let Some(mod_id) = queue.pop_front() {
            if !processed.insert(mod_id) {
                continue;
            }
            load_order.push(mod_id);

            if let Some(dependents) = graph.get(&mod_id) {
                for dependent in dependents {
                    if let Some(deg) = in_degree.get_mut(dependent) {
                        *deg = deg.saturating_sub(1);
                        if *deg == 0 {
                            queue.push_back(*dependent);
                        }
                    }
                }
            }
        }

        // Check for circular dependencies
        if processed.len() != all_mods.len() {
            let remaining: Vec<_> = all_mods.difference(&processed).collect();
            return Err(DependencyError::CircularDependency(format!(
                "Circular dependency involving: {:?}",
                remaining
            )));
        }

        // Build resolved dependencies
        let mut dependencies = HashMap::new();
        let mut versions = HashMap::new();

        for mod_id in &load_order {
            if let Some(meta) = self.mod_metadata.get(mod_id) {
                dependencies.insert(mod_id.clone(), meta.dependencies.clone());
                versions.insert(mod_id.clone(), meta.version.clone());
            }
        }

        Ok(ResolvedDependencies {
            load_order,
            versions,
            dependencies,
        })
    }

    /// Collect all transitive dependencies for a mod
    fn collect_dependencies(
        &self,
        mod_id: &Identifier,
        graph: &mut BTreeMap<Identifier, BTreeSet<Identifier>>,
        in_degree: &mut BTreeMap<Identifier, usize>,
        all_mods: &mut BTreeSet<Identifier>,
    ) -> Result<(), DependencyError> {
        if all_mods.contains(mod_id) {
            return Ok(());
        }
        all_mods.insert(mod_id.clone());

        let meta = self
            .mod_metadata
            .get(mod_id)
            .ok_or_else(|| DependencyError::ModNotFound(mod_id.to_string()))?;

        for dep in &meta.dependencies {
            // Check if dependency is available
            if !self.available_mods.contains_key(&dep.id) {
                if !dep.optional {
                    return Err(DependencyError::MissingDependency(
                        dep.id.to_string(),
                        mod_id.to_string(),
                    ));
                }
                continue;
            }

            // Check version compatibility
            let dep_version = self.available_mods.get(&dep.id).unwrap();
            let req = VersionReq::parse(&dep.version)
                .map_err(|e| DependencyError::InvalidVersionReq(e.to_string()))?;

            if !req.matches(dep_version) {
                return Err(DependencyError::VersionConflict(
                    mod_id.to_string(),
                    format!("requires {} but {} is available", dep.version, dep_version),
                ));
            }

            // Add edge: dependency -> mod_id (dependencies load first).
            graph
                .entry(dep.id)
                .or_default()
                .insert(mod_id.clone());
            *in_degree.entry(mod_id.clone()).or_insert(0) += 1;

            // Recursively collect transitive dependencies
            self.collect_dependencies(&dep.id, graph, in_degree, all_mods)?;
        }

        Ok(())
    }

    /// Check if a set of mods is compatible with the current MCLRE version
    pub fn check_compatibility(&self, mclre_version: &semver::Version) -> Vec<String> {
        let mut incompatible = Vec::new();
        for (id, meta) in &self.mod_metadata {
            if let Ok(req) = semver::VersionReq::parse(&meta.mclre_version) {
                if !req.matches(mclre_version) {
                    incompatible.push(format!(
                        "{} requires MCLRE {} but {} is running",
                        id, meta.mclre_version, mclre_version
                    ));
                }
            }
        }
        incompatible
    }
}

impl Default for DependencyResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::PermissionSet;
    use mclre_core::Identifier;
    use mclre_mod_api::{ModDependency, ModMetadata};
    use semver::Version;

    fn create_test_metadata(id: &str, version: &str, deps: Vec<ModDependency>) -> ModMetadata {
        let id = Identifier::parse(id).unwrap();
        ModMetadata {
            id: id.clone(),
            name: format!("Mod {}", id),
            version: version.to_string(),
            description: "Test".to_string(),
            authors: vec!["Test".to_string()],
            license: "MIT".to_string(),
            mclre_version: "^1.0.0".to_string(),
            dependencies: deps,
        }
    }

    #[test]
    fn test_simple_dependency() {
        let mut resolver = DependencyResolver::new();

        let mod_a = Identifier::parse("test:mod_a").unwrap();
        let mod_b = Identifier::parse("test:mod_b").unwrap();

        resolver.add_mod(
            mod_a.clone(),
            Version::parse("1.0.0").unwrap(),
            create_test_metadata("test:mod_a", "1.0.0", vec![]),
        );
        resolver.add_mod(
            mod_b.clone(),
            Version::parse("1.0.0").unwrap(),
            create_test_metadata(
                "test:mod_b",
                "1.0.0",
                vec![ModDependency {
                    id: mod_a.clone(),
                    version: "^1.0.0".to_string(),
                    optional: false,
                }],
            ),
        );

        let resolved = resolver.resolve(&[mod_b]).unwrap();
        let load_order = resolved.load_order;
        assert_eq!(load_order.len(), 2);
        // mod_a should come before mod_b
        assert_eq!(load_order[0], mod_a);
        assert_eq!(load_order[1], mod_b);
    }

    #[test]
    fn test_missing_dependency() {
        let mut resolver = DependencyResolver::new();

        let mod_a = Identifier::parse("test:mod_a").unwrap();
        let mod_b = Identifier::parse("test:mod_b").unwrap();

        resolver.add_mod(
            mod_b.clone(),
            Version::parse("1.0.0").unwrap(),
            create_test_metadata(
                "test:mod_b",
                "1.0.0",
                vec![ModDependency {
                    id: mod_a.clone(),
                    version: "^1.0.0".to_string(),
                    optional: false,
                }],
            ),
        );

        let result = resolver.resolve(&[mod_b]);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DependencyError::MissingDependency(_, _)
        ));
    }

    #[test]
    fn test_optional_dependency() {
        let mut resolver = DependencyResolver::new();

        let mod_a = Identifier::parse("test:mod_a").unwrap();
        let mod_b = Identifier::parse("test:mod_b").unwrap();

        resolver.add_mod(
            mod_b.clone(),
            Version::parse("1.0.0").unwrap(),
            create_test_metadata(
                "test:mod_b",
                "1.0.0",
                vec![ModDependency {
                    id: mod_a.clone(),
                    version: "^1.0.0".to_string(),
                    optional: true,
                }],
            ),
        );

        // Should succeed even though mod_a is not present (optional)
        let resolved = resolver.resolve(&[mod_b]).unwrap();
        let load_order = resolved.load_order;
        assert_eq!(load_order.len(), 1);
        assert_eq!(load_order[0], mod_b);
    }

    #[test]
    fn test_version_conflict() {
        let mut resolver = DependencyResolver::new();

        let mod_a = Identifier::parse("test:mod_a").unwrap();
        let mod_b = Identifier::parse("test:mod_b").unwrap();

        resolver.add_mod(
            mod_a.clone(),
            Version::parse("2.0.0").unwrap(),
            create_test_metadata("test:mod_a", "2.0.0", vec![]),
        );
        resolver.add_mod(
            mod_b.clone(),
            Version::parse("1.0.0").unwrap(),
            create_test_metadata(
                "test:mod_b",
                "1.0.0",
                vec![ModDependency {
                    id: mod_a.clone(),
                    version: "^1.0.0".to_string(),
                    optional: false,
                }],
            ),
        );

        let result = resolver.resolve(&[mod_b]);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DependencyError::VersionConflict(_, _)
        ));
    }

    #[test]
    fn test_circular_dependency() {
        let mut resolver = DependencyResolver::new();

        let mod_a = Identifier::parse("test:mod_a").unwrap();
        let mod_b = Identifier::parse("test:mod_b").unwrap();

        resolver.add_mod(
            mod_a.clone(),
            Version::parse("1.0.0").unwrap(),
            create_test_metadata(
                "test:mod_a",
                "1.0.0",
                vec![ModDependency {
                    id: mod_b.clone(),
                    version: "^1.0.0".to_string(),
                    optional: false,
                }],
            ),
        );
        resolver.add_mod(
            mod_b.clone(),
            Version::parse("1.0.0").unwrap(),
            create_test_metadata(
                "test:mod_b",
                "1.0.0",
                vec![ModDependency {
                    id: mod_a.clone(),
                    version: "^1.0.0".to_string(),
                    optional: false,
                }],
            ),
        );

        let result = resolver.resolve(&[mod_a, mod_b]);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DependencyError::CircularDependency(_)
        ));
    }
}
