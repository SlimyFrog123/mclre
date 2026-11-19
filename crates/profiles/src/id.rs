//! Profile identifiers (`namespace:path`, e.g. `lce:141217`).

use std::fmt;
use std::str::FromStr;

use mclre_core::{Identifier, ParseError};

/// Identifier of a game profile.
///
/// A profile id is a namespaced [`Identifier`] (`lce:` for Legacy Console
/// Edition profiles, `java:` for future Java-parity profiles), so it obeys the
/// same character rules and is stored in saves verbatim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProfileId(Identifier);

impl ProfileId {
    /// Parses a full `namespace:path` profile id.
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        Ok(Self(Identifier::parse(input)?))
    }

    /// Wraps an already-validated identifier.
    pub fn from_identifier(id: Identifier) -> Self {
        Self(id)
    }

    /// The underlying identifier.
    pub fn as_identifier(&self) -> Identifier {
        self.0
    }

    /// Canonical `namespace:path` form.
    pub fn as_str(&self) -> String {
        self.0.to_string_form()
    }
}

impl fmt::Display for ProfileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.to_string_form())
    }
}

impl FromStr for ProfileId {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unqualified_and_uppercase_ids() {
        assert!(ProfileId::parse("141217").is_err());
        assert!(ProfileId::parse("lce:141217:").is_err());
        assert!(ProfileId::parse("Lce:141217").is_err());
    }

    #[test]
    fn accepts_namespaces_and_paths() {
        assert_eq!(
            ProfileId::parse("java:1.8.9").expect("valid").as_str(),
            "java:1.8.9"
        );
        assert_eq!(
            ProfileId::parse("lce:tu19").expect("valid").as_str(),
            "lce:tu19"
        );
    }
}
