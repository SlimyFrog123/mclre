//! Namespaced identifiers (SPEC.md 5.1).
//!
//! Every block/item/entity/biome/etc. id in game logic, saves, and the protocol
//! is a [`Identifier`] (`namespace:path`). Numeric ids exist only inside
//! per-session protocol palettes, in-memory registry indices, and legacy
//! mapping tables — never here.
//!
//! Strings are interned in a process-wide pool; the [`Identifier`] itself is a
//! small `Copy` handle so registries can store and compare ids cheaply.

use std::fmt;
use std::sync::OnceLock;

use parking_lot::Mutex;

/// The default namespace applied to user-facing input that omits one
/// (e.g. `"stone"` typed by a player). Storage always carries an explicit
/// namespace (SPEC.md 5.1).
pub const DEFAULT_NAMESPACE: &str = "minecraft";

// ---------------------------------------------------------------------------
// Interner
// ---------------------------------------------------------------------------

/// A handle to a string interned in the process-wide pool. `Copy` and cheap to
/// compare/hash (SPEC.md 5.1: interned, `Copy`-able handle form).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InternedStr(u32);

struct StringPool {
    map: std::collections::HashMap<Box<str>, u32>,
    strings: Vec<Box<str>>,
}

impl StringPool {
    fn new() -> Self {
        Self {
            map: std::collections::HashMap::new(),
            strings: Vec::new(),
        }
    }

    fn intern(&mut self, s: &str) -> InternedStr {
        if let Some(&idx) = self.map.get(s) {
            return InternedStr(idx);
        }
        // A 32-bit index would only truncate past ~4 billion distinct strings;
        // unreachable for any real MCLRE session.
        #[allow(clippy::cast_possible_truncation)]
        let idx = self.strings.len() as u32;
        let owned: Box<str> = s.into();
        self.strings.push(owned.clone());
        self.map.insert(owned, idx);
        InternedStr(idx)
    }

    fn resolve(&self, s: InternedStr) -> String {
        self.strings.get(s.0 as usize).map_or_else(
            || format!("<invalid interner handle {}>", s.0),
            ToString::to_string,
        )
    }
}

fn pool() -> &'static Mutex<StringPool> {
    static POOL: OnceLock<Mutex<StringPool>> = OnceLock::new();
    POOL.get_or_init(|| Mutex::new(StringPool::new()))
}

impl InternedStr {
    /// Interns a string, returning a stable handle.
    pub fn intern(s: &str) -> Self {
        pool().lock().intern(s)
    }

    /// Resolves the handle back to an owned string.
    ///
    /// (The pool is `#![forbid(unsafe_code)]`-clean, so this clones instead of
    /// handing out `&'static str`.)
    pub fn resolve(self) -> String {
        pool().lock().resolve(self)
    }
}

// ---------------------------------------------------------------------------
// Identifier
// ---------------------------------------------------------------------------

/// A namespaced identifier: `namespace:path`.
///
/// - `namespace`: `[a-z0-9_.-]+`
/// - `path`: `[a-z0-9_.-/]+`
///
/// Compare/hash cost is O(1) (interned handles); ordering is by *content*
/// (namespace, then path), so iteration over sorted ids is deterministic
/// across platforms regardless of interning order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Identifier {
    namespace: InternedStr,
    path: InternedStr,
}

/// Errors produced when parsing or validating [`Identifier`]s.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    /// Input contained no `:` separator where one was required.
    #[error("missing `:` separator in identifier: {input:?}")]
    MissingSeparator {
        /// The offending input string.
        input: String,
    },
    /// The namespace part failed validation.
    #[error("invalid namespace {value:?} in identifier: {reason}")]
    InvalidNamespace {
        /// The offending namespace text.
        value: String,
        /// Why it failed.
        reason: &'static str,
    },
    /// The path part failed validation.
    #[error("invalid path {value:?} in identifier: {reason}")]
    InvalidPath {
        /// The offending path text.
        value: String,
        /// Why it failed.
        reason: &'static str,
    },
}

/// Characters allowed in a namespace.
fn is_valid_namespace_char(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '.' | '-')
}

/// Characters allowed in a path.
fn is_valid_path_char(c: char) -> bool {
    is_valid_namespace_char(c) || c == '/'
}

fn validate_namespace(ns: &str) -> Result<(), ParseError> {
    if ns.is_empty() {
        return Err(ParseError::InvalidNamespace {
            value: ns.to_string(),
            reason: "empty",
        });
    }
    if !ns.chars().all(is_valid_namespace_char) {
        return Err(ParseError::InvalidNamespace {
            value: ns.to_string(),
            reason: "must only contain [a-z0-9_.-]",
        });
    }
    Ok(())
}

fn validate_path(path: &str) -> Result<(), ParseError> {
    if path.is_empty() {
        return Err(ParseError::InvalidPath {
            value: path.to_string(),
            reason: "empty",
        });
    }
    if !path.chars().all(is_valid_path_char) {
        return Err(ParseError::InvalidPath {
            value: path.to_string(),
            reason: "must only contain [a-z0-9_.-/]",
        });
    }
    Ok(())
}

impl Identifier {
    /// Constructs an identifier from explicit parts, validating both.
    pub fn new(namespace: &str, path: &str) -> Result<Self, ParseError> {
        validate_namespace(namespace)?;
        validate_path(path)?;
        Ok(Self {
            namespace: InternedStr::intern(namespace),
            path: InternedStr::intern(path),
        })
    }

    /// Parses a fully-qualified `"namespace:path"` string.
    ///
    /// Unlike [`Identifier::parse_with_default_namespace`], this requires an
    /// explicit namespace (storage, protocol, and save data always carry one).
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        let (namespace, path) =
            input
                .split_once(':')
                .ok_or_else(|| ParseError::MissingSeparator {
                    input: input.to_string(),
                })?;
        Self::new(namespace, path)
    }

    /// Parses user-facing input, applying [`DEFAULT_NAMESPACE`] when the input
    /// omits a namespace. Storage never uses this — see [`Identifier::parse`].
    pub fn parse_with_default_namespace(input: &str) -> Result<Self, ParseError> {
        match input.split_once(':') {
            Some((namespace, path)) => Self::new(namespace, path),
            None => Self::new(DEFAULT_NAMESPACE, input),
        }
    }

    /// The namespace part.
    pub fn namespace(&self) -> String {
        self.namespace.resolve()
    }

    /// The path part.
    pub fn path(&self) -> String {
        self.path.resolve()
    }

    /// The fully-qualified string form (`namespace:path`).
    pub fn to_string_form(&self) -> String {
        format!("{}:{}", self.namespace.resolve(), self.path.resolve())
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_form())
    }
}

/// Content ordering: by namespace, then path — deterministic regardless of
/// interning order.
impl Ord for Identifier {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_string_form().cmp(&other.to_string_form())
    }
}

impl PartialOrd for Identifier {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_explicit_namespace() {
        let id = Identifier::parse("minecraft:stone").expect("valid");
        assert_eq!(id.namespace(), "minecraft");
        assert_eq!(id.path(), "stone");
        assert_eq!(id.to_string_form(), "minecraft:stone");
    }

    #[test]
    fn parses_multiple_separators_in_path_only() {
        let id = Identifier::parse("minecraft:blocks/stone_slab").expect("valid");
        assert_eq!(id.path(), "blocks/stone_slab");
    }

    #[test]
    fn default_namespace_applies_only_on_user_facing_parse() {
        assert!(
            Identifier::parse("stone").is_err(),
            "plain parse requires explicit namespace"
        );
        let id = Identifier::parse_with_default_namespace("stone").expect("valid");
        assert_eq!(id.to_string_form(), "minecraft:stone");
        let explicit = Identifier::parse_with_default_namespace("lce:mini_game").expect("valid");
        assert_eq!(explicit.namespace(), "lce");
    }

    #[test]
    fn empty_namespace_is_rejected() {
        assert_eq!(
            Identifier::parse(":stone"),
            Err(ParseError::InvalidNamespace {
                value: String::new(),
                reason: "empty"
            })
        );
    }

    #[test]
    fn uppercase_is_rejected() {
        assert!(Identifier::parse("minecraft:Stone").is_err());
        assert!(Identifier::parse("Minecraft:stone").is_err());
    }

    #[test]
    fn forbidden_characters_are_rejected() {
        assert!(Identifier::parse("minecraft:stone!").is_err());
        assert!(Identifier::parse("mine craft:stone").is_err());
        assert!(Identifier::parse("mine craft:stone").is_err());
    }

    #[test]
    fn handles_are_copy_and_comparable() {
        let a = Identifier::parse("minecraft:stone").expect("valid");
        let b = a; // Copy
        assert_eq!(a, b);
    }

    #[test]
    fn content_order_is_independent_of_interning_order() {
        // Interned in this order: stone first.
        let stone = Identifier::parse("minecraft:stone").expect("valid");
        let dirt = Identifier::parse("minecraft:dirt").expect("valid");
        // Ordering is by content, not handle: dirt < stone.
        assert!(dirt < stone);
        // A different interning order must not change the result.
        let stone2 = Identifier::parse("minecraft:stone").expect("valid");
        let dirt2 = Identifier::parse("minecraft:dirt").expect("valid");
        assert!(dirt2 < stone2);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    fn ns_strategy() -> impl Strategy<Value = String> {
        "[a-z0-9_.-]{1,32}"
    }

    fn path_strategy() -> impl Strategy<Value = String> {
        "[a-z0-9_.-/]{1,64}"
    }

    proptest! {
        #[test]
        fn round_trip_through_string_form(ns in ns_strategy(), path in path_strategy()) {
            let id = Identifier::new(&ns, &path).expect("generated parts are valid");
            let s = id.to_string_form();
            let reparsed = Identifier::parse(&s).expect("string form reparses");
            prop_assert_eq!(id, reparsed);
        }

        #[test]
        fn default_namespace_round_trip(path in path_strategy()) {
            let id = Identifier::parse_with_default_namespace(&path).expect("valid path only");
            prop_assert_eq!(id.namespace(), DEFAULT_NAMESPACE);
            prop_assert_eq!(id.path(), path);
        }
    }
}
