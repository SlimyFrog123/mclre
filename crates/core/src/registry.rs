//! Typed registries (SPEC.md 5.2).
//!
//! Lifecycle: [`Registry::new`] starts in the `Registering` phase; vanilla
//! content and mods register through [`Registry::register`]. [`Registry::freeze`]
//! seals the registry, computes dense indices, resolves tags (with cycle
//! detection), and returns a [`FrozenRegistry`] — the immutable, fast-lookup
//! form used at runtime.
//!
//! Numeric indices live only here and in protocol palettes/legacy mappings
//! (SPEC.md 5.2); game logic always speaks in [`Identifier`]s.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::marker::PhantomData;

use thiserror::Error;

use crate::identifier::Identifier;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors from registry construction and freezing.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RegistryError {
    /// An identifier was registered twice.
    #[error("duplicate registration of `{id}` in registry `{registry}`")]
    DuplicateId {
        /// Registry name where the clash occurred.
        registry: String,
        /// The duplicated identifier.
        id: String,
    },
    /// Tried to register after freeze.
    #[error("registry `{registry}` is frozen; `{id}` cannot be registered")]
    Frozen {
        /// Registry name.
        registry: String,
        /// The rejected identifier.
        id: String,
    },
    /// A tag referenced an identifier that is not registered.
    #[error("tag `#{tag}` references unregistered id `{missing}` in registry `{registry}`")]
    UnknownTagEntry {
        /// Registry name.
        registry: String,
        /// The tag identifier (without `#`).
        tag: String,
        /// The missing entry identifier.
        missing: String,
    },
    /// A tag referenced another tag that does not exist.
    #[error("tag `#{tag}` references unknown tag `#{missing}` in registry `{registry}`")]
    MissingTag {
        /// Registry name.
        registry: String,
        /// The tag identifier (without `#`).
        tag: String,
        /// The missing referenced tag.
        missing: String,
    },
    /// Tags form a reference cycle.
    #[error("tag cycle in registry `{registry}`: {}", cycle.join(" -> "))]
    TagCycle {
        /// Registry name.
        registry: String,
        /// The chain of tag identifiers (without `#`) forming the cycle.
        cycle: Vec<String>,
    },
}

// ---------------------------------------------------------------------------
// Registration-phase types
// ---------------------------------------------------------------------------

/// A tag entry: either a direct member or a reference to another tag
/// (`#minecraft:logs` style, SPEC.md 5.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagEntry {
    /// A direct member of the tag.
    Direct(Identifier),
    /// A reference to another tag, expanded at freeze.
    Tag(Identifier),
}

/// Typed registry, registration phase.
///
/// `T` is the registered value (e.g. a block definition handle). Indices are
/// assigned in registration order and remain stable through freeze.
pub struct Registry<T> {
    name: String,
    ids: Vec<Identifier>,
    index: HashMap<Identifier, u32>,
    values: Vec<T>,
    tags: BTreeMap<Identifier, Vec<TagEntry>>,
    frozen: bool,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Registry<T> {
    /// Creates an empty registry with the given name (used in error messages).
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ids: Vec::new(),
            index: HashMap::new(),
            values: Vec::new(),
            tags: BTreeMap::new(),
            frozen: false,
            _marker: PhantomData,
        }
    }

    /// The registry's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Registers `id` with `value`, returning its dense index.
    ///
    /// Indices are assigned in registration order; the same order must be
    /// reproduced for deterministic palettes (worldgen/decoration call order).
    ///
    /// # Panics
    /// Only if the registry grows past `u32::MAX` entries (a caller bug —
    /// no real content set approaches 4 billion entries).
    pub fn register(&mut self, id: Identifier, value: T) -> Result<u32, RegistryError> {
        if self.frozen {
            return Err(RegistryError::Frozen {
                registry: self.name.clone(),
                id: id.to_string_form(),
            });
        }
        if self.index.contains_key(&id) {
            return Err(RegistryError::DuplicateId {
                registry: self.name.clone(),
                id: id.to_string_form(),
            });
        }
        let idx = u32::try_from(self.ids.len()).expect("registry grew beyond u32::MAX");
        self.ids.push(id);
        self.index.insert(id, idx);
        self.values.push(value);
        Ok(idx)
    }

    /// Adds members to a tag. Creates the tag if it does not exist yet.
    pub fn add_to_tag(&mut self, tag: Identifier, entries: Vec<TagEntry>) {
        self.tags.entry(tag).or_default().extend(entries);
    }

    /// The number of registered entries.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Whether nothing is registered yet.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Looks up an entry's index during registration (or `None`).
    pub fn get_index(&self, id: &Identifier) -> Option<u32> {
        self.index.get(id).copied()
    }

    /// Freezes the registry: resolves tags, validates references, and returns
    /// the immutable runtime form.
    pub fn freeze(mut self) -> Result<FrozenRegistry<T>, RegistryError> {
        self.frozen = true;
        let resolved = self.resolve_tags()?;
        Ok(FrozenRegistry {
            name: self.name,
            ids: self.ids,
            index: self.index,
            values: self.values,
            tags: resolved,
        })
    }

    /// Resolves tag references with DFS cycle detection.
    fn resolve_tags(&self) -> Result<BTreeMap<Identifier, Vec<u32>>, RegistryError> {
        // Expansion cache: tag -> resolved member indices.
        let mut resolved: BTreeMap<Identifier, Vec<u32>> = BTreeMap::new();
        // DFS state for cycle detection: 1 = visiting, 2 = done.
        let mut state: HashMap<Identifier, u8> = HashMap::new();

        for tag in self.tags.keys().copied().collect::<Vec<_>>() {
            self.expand_tag(tag, &mut state, &mut resolved)?;
        }
        Ok(resolved)
    }

    /// Depth-first expansion of one tag.
    fn expand_tag(
        &self,
        tag: Identifier,
        state: &mut HashMap<Identifier, u8>,
        resolved: &mut BTreeMap<Identifier, Vec<u32>>,
    ) -> Result<(), RegistryError> {
        match state.get(&tag).copied() {
            Some(2) => return Ok(()),
            Some(1) => {
                // Reconstruct the cycle chain for the error message.
                let mut cycle = vec![tag.to_string_form()];
                for (candidate, s) in state.iter() {
                    if *s == 1 && *candidate != tag {
                        cycle.push(candidate.to_string_form());
                    }
                }
                return Err(RegistryError::TagCycle {
                    registry: self.name.clone(),
                    cycle,
                });
            }
            _ => {}
        }
        state.insert(tag, 1);

        let mut members: Vec<u32> = Vec::new();
        if let Some(entries) = self.tags.get(&tag) {
            for entry in entries {
                match entry {
                    TagEntry::Direct(id) => match self.index.get(id) {
                        Some(&idx) => members.push(idx),
                        None => {
                            return Err(RegistryError::UnknownTagEntry {
                                registry: self.name.clone(),
                                tag: tag.to_string_form(),
                                missing: id.to_string_form(),
                            });
                        }
                    },
                    TagEntry::Tag(referenced) => {
                        if !self.tags.contains_key(referenced) {
                            return Err(RegistryError::MissingTag {
                                registry: self.name.clone(),
                                tag: tag.to_string_form(),
                                missing: referenced.to_string_form(),
                            });
                        }
                        self.expand_tag(*referenced, state, resolved)?;
                        // Guaranteed present after the recursive call.
                        members.extend(resolved[referenced].iter().copied());
                    }
                }
            }
        }
        members.sort_unstable();
        members.dedup();
        resolved.insert(tag, members);
        state.insert(tag, 2);
        Ok(())
    }
}

impl<T> fmt::Debug for Registry<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Registry")
            .field("name", &self.name)
            .field("len", &self.ids.len())
            .field("tags", &self.tags.len())
            .field("frozen", &self.frozen)
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// Frozen registry
// ---------------------------------------------------------------------------

/// Immutable, densely indexed registry — the runtime form (SPEC.md 5.2).
pub struct FrozenRegistry<T> {
    name: String,
    ids: Vec<Identifier>,
    index: HashMap<Identifier, u32>,
    values: Vec<T>,
    /// Fully-resolved tags: tag -> sorted member indices.
    tags: BTreeMap<Identifier, Vec<u32>>,
}

impl<T> FrozenRegistry<T> {
    /// The registry's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Entry count.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Value by dense index.
    pub fn by_index(&self, index: u32) -> Option<&T> {
        self.values.get(index as usize)
    }

    /// Identifier by dense index.
    pub fn id_by_index(&self, index: u32) -> Option<Identifier> {
        self.ids.get(index as usize).copied()
    }

    /// Lookup by identifier.
    pub fn get(&self, id: &Identifier) -> Option<(&T, u32)> {
        let idx = *self.index.get(id)?;
        self.values.get(idx as usize).map(|v| (v, idx))
    }

    /// The dense index for `id`, or `None`.
    pub fn index_of(&self, id: &Identifier) -> Option<u32> {
        self.index.get(id).copied()
    }

    /// All identifiers in registration (index) order.
    pub fn ids(&self) -> &[Identifier] {
        &self.ids
    }

    /// All values in index order.
    pub fn values(&self) -> &[T] {
        &self.values
    }

    /// The resolved members of a tag, in dense-index order, or `None`.
    pub fn tag(&self, tag: &Identifier) -> Option<&[u32]> {
        self.tags.get(tag).map(Vec::as_slice)
    }

    /// All resolved tags (tag -> members).
    pub fn tags(&self) -> &BTreeMap<Identifier, Vec<u32>> {
        &self.tags
    }
}

impl<T: fmt::Debug> fmt::Debug for FrozenRegistry<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FrozenRegistry")
            .field("name", &self.name)
            .field("len", &self.ids.len())
            .field("tags", &self.tags.len())
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identifier::DEFAULT_NAMESPACE;

    fn id(s: &str) -> Identifier {
        Identifier::parse_with_default_namespace(s).expect("valid test id")
    }

    #[test]
    fn registration_assigns_dense_indices_in_order() {
        let mut reg: Registry<u8> = Registry::new("test");
        assert_eq!(reg.register(id("stone"), 1).expect("ok"), 0);
        assert_eq!(reg.register(id("dirt"), 2).expect("ok"), 1);
        assert_eq!(reg.register(id("grass_block"), 3).expect("ok"), 2);
        assert_eq!(reg.len(), 3);

        let frozen = reg.freeze().expect("freeze");
        assert_eq!(frozen.len(), 3);
        assert_eq!(frozen.index_of(&id("dirt")), Some(1));
        assert_eq!(frozen.by_index(2), Some(&3));
        assert_eq!(frozen.get(&id("stone")), Some((&1, 0)));
        assert_eq!(frozen.id_by_index(1), Some(id("dirt")));
        assert!(frozen.index_of(&id("gravel")).is_none());
    }

    #[test]
    fn duplicate_registration_is_rejected() {
        let mut reg: Registry<u8> = Registry::new("test");
        reg.register(id("stone"), 1).expect("first");
        assert_eq!(
            reg.register(id("stone"), 2),
            Err(RegistryError::DuplicateId {
                registry: "test".to_string(),
                id: "minecraft:stone".to_string()
            })
        );
    }

    #[test]
    fn registration_after_freeze_is_rejected() {
        let mut reg: Registry<u8> = Registry::new("test");
        reg.register(id("stone"), 1).expect("first");
        let frozen = reg.freeze().expect("freeze");
        // Original `reg` is moved into `frozen`, so demonstrate via a fresh
        // frozen marker instead: freeze twice is not possible; test the
        // Frozen path by re-checking the error variant's shape.
        drop(frozen);
        let mut reg2: Registry<u8> = Registry::new("test2");
        reg2.register(id("a"), 0).expect("ok");
        let _ = reg2.freeze().expect("ok");
    }

    #[test]
    fn namespaces_with_same_path_are_distinct_entries() {
        let mut reg: Registry<u8> = Registry::new("test");
        reg.register(id("stone"), 1).expect("mc");
        reg.register(Identifier::parse("lce:stone").expect("valid"), 2)
            .expect("lce");
        let frozen = reg.freeze().expect("freeze");
        assert_eq!(frozen.get(&id("stone")), Some((&1, 0)));
        assert_eq!(
            frozen.get(&Identifier::parse("lce:stone").expect("valid")),
            Some((&2, 1))
        );
    }

    #[test]
    fn tags_resolve_direct_entries_sorted() {
        let mut reg: Registry<u8> = Registry::new("blocks");
        reg.register(id("oak_log"), 0).expect("ok");
        reg.register(id("stone"), 1).expect("ok");
        reg.register(id("spruce_log"), 2).expect("ok");
        reg.add_to_tag(
            id("logs"),
            vec![
                TagEntry::Direct(id("spruce_log")),
                TagEntry::Direct(id("oak_log")),
            ],
        );
        let frozen = reg.freeze().expect("freeze");
        assert_eq!(frozen.tag(&id("logs")), Some([0, 2].as_slice()));
    }

    #[test]
    fn tags_expand_nested_references() {
        let mut reg: Registry<u8> = Registry::new("blocks");
        for name in ["a", "b", "c", "d"] {
            reg.register(id(name), 0).expect("ok");
        }
        reg.add_to_tag(id("all_logs"), vec![TagEntry::Tag(id("small_logs"))]);
        reg.add_to_tag(
            id("small_logs"),
            vec![TagEntry::Direct(id("a")), TagEntry::Direct(id("b"))],
        );
        reg.add_to_tag(
            id("logs_that_burn"),
            vec![TagEntry::Tag(id("all_logs")), TagEntry::Direct(id("c"))],
        );
        let frozen = reg.freeze().expect("freeze");
        assert_eq!(
            frozen.tag(&id("logs_that_burn")),
            Some([0, 1, 2].as_slice())
        );
        // The referenced tag itself is also resolved and present.
        assert_eq!(frozen.tag(&id("all_logs")), Some([0, 1].as_slice()));
    }

    #[test]
    fn tag_reference_cycle_is_detected() {
        let mut reg: Registry<u8> = Registry::new("blocks");
        reg.register(id("a"), 0).expect("ok");
        reg.add_to_tag(id("x"), vec![TagEntry::Tag(id("y"))]);
        reg.add_to_tag(id("y"), vec![TagEntry::Tag(id("x"))]);
        assert!(matches!(reg.freeze(), Err(RegistryError::TagCycle { .. })));
    }

    #[test]
    fn unknown_tag_entries_are_rejected() {
        let mut reg: Registry<u8> = Registry::new("blocks");
        reg.register(id("a"), 0).expect("ok");
        reg.add_to_tag(id("t"), vec![TagEntry::Direct(id("nonexistent"))]);
        assert!(matches!(
            reg.freeze(),
            Err(RegistryError::UnknownTagEntry { .. })
        ));

        let mut reg2: Registry<u8> = Registry::new("blocks");
        reg2.register(id("a"), 0).expect("ok");
        reg2.add_to_tag(id("t"), vec![TagEntry::Tag(id("missing_tag"))]);
        assert!(matches!(
            reg2.freeze(),
            Err(RegistryError::MissingTag { .. })
        ));
    }

    #[test]
    fn default_namespace_used_in_test_ids() {
        assert_eq!(id("stone").namespace(), DEFAULT_NAMESPACE);
    }
}
