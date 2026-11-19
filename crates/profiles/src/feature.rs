//! Typed feature flags (SPEC.md 6.1).
//!
//! A flag names a mechanic whose behavior differs between game versions or
//! editions. Mechanics query `profile.has(FeatureFlag::X)`; they never test
//! version numbers. Every flag must also have a row with reference evidence in
//! `docs/reference/feature-flags.md` (CLAUDE.md rule 3 / SPEC.md 3.5).
//!
//! # Adding a flag
//!
//! 1. Confirm the mechanic actually differs (reference comments, owner
//!    captures, SPEC).
//! 2. Add the variant here **and** a row in `docs/reference/feature-flags.md`
//!    citing the reference file/function.
//! 3. Set it in the profiles that have it; read it at the single decision
//!    point; cover it with a parity test.

use std::collections::BTreeSet;

/// A typed, profile-gated gameplay/UX mechanic.
///
/// The enum is `#[non_exhaustive]`: variants are added as features are ported,
/// so downstream matches must include a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FeatureFlag {
    /// Console-style crafting menu: tabbed recipe groups/categories with
    /// "craftable now" filtering, instead of the Java 3×3 grid UI.
    ///
    /// Applies to `lce:` profiles; Java-parity profiles will disable it.
    /// Evidence: SPEC.md 3.6 and PLAN.md M5 (LCE crafting UX); exact reference
    /// files/functions to be cited at M5 in `docs/reference/feature-flags.md`.
    LceCraftingMenu,
    /// LCE host-privilege permission model (per-player build/mine/use/attack
    /// toggles) instead of Java slash commands.
    ///
    /// Evidence: SPEC.md 5.8; reference `HostOptions`/privilege code to be
    /// cited at M9.
    LceHostPrivileges,
}

impl FeatureFlag {
    /// Every flag known to this build, in a stable order (used for iteration,
    /// UI/debug dumps, and save compatibility checks).
    pub const ALL: &'static [FeatureFlag] =
        &[FeatureFlag::LceCraftingMenu, FeatureFlag::LceHostPrivileges];

    /// Stable machine-readable key (used in save files and diagnostics).
    pub fn key(self) -> &'static str {
        match self {
            FeatureFlag::LceCraftingMenu => "lce_crafting_menu",
            FeatureFlag::LceHostPrivileges => "lce_host_privileges",
        }
    }
}

/// A set of enabled [`FeatureFlag`]s.
///
/// Backed by a `BTreeSet`, so iteration order is deterministic (the enum's
/// declaration order) on every platform.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeatureFlagSet {
    flags: BTreeSet<FeatureFlag>,
}

impl FeatureFlagSet {
    /// An empty set (no optional mechanics enabled).
    pub fn new() -> Self {
        Self::default()
    }

    /// Enables `flag`; returns whether it was newly inserted.
    pub fn insert(&mut self, flag: FeatureFlag) -> bool {
        self.flags.insert(flag)
    }

    /// Enables every flag in `flags`.
    pub fn extend<I: IntoIterator<Item = FeatureFlag>>(&mut self, flags: I) {
        self.flags.extend(flags);
    }

    /// Disables `flag`; returns whether it was present.
    pub fn remove(&mut self, flag: FeatureFlag) -> bool {
        self.flags.remove(&flag)
    }

    /// Whether `flag` is enabled.
    pub fn contains(&self, flag: FeatureFlag) -> bool {
        self.flags.contains(&flag)
    }

    /// Number of enabled flags.
    pub fn len(&self) -> usize {
        self.flags.len()
    }

    /// Whether no flag is enabled.
    pub fn is_empty(&self) -> bool {
        self.flags.is_empty()
    }

    /// Iterates enabled flags in declaration order.
    pub fn iter(&self) -> impl Iterator<Item = FeatureFlag> + '_ {
        self.flags.iter().copied()
    }

    /// Stable keys of the enabled flags, in declaration order.
    pub fn keys(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.flags.iter().map(|flag| flag.key())
    }
}

impl FromIterator<FeatureFlag> for FeatureFlagSet {
    fn from_iter<I: IntoIterator<Item = FeatureFlag>>(iter: I) -> Self {
        Self {
            flags: iter.into_iter().collect(),
        }
    }
}

impl From<&[FeatureFlag]> for FeatureFlagSet {
    fn from(flags: &[FeatureFlag]) -> Self {
        flags.iter().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_operations_are_membership_based() {
        let mut set = FeatureFlagSet::new();
        assert!(set.is_empty());
        assert!(set.insert(FeatureFlag::LceHostPrivileges));
        assert!(!set.insert(FeatureFlag::LceHostPrivileges));
        assert!(set.contains(FeatureFlag::LceHostPrivileges));
        assert_eq!(set.len(), 1);
        assert!(set.remove(FeatureFlag::LceHostPrivileges));
        assert!(!set.remove(FeatureFlag::LceHostPrivileges));
        assert!(set.is_empty());
    }

    #[test]
    fn iteration_is_in_declaration_order() {
        let set = FeatureFlagSet::from(
            &[FeatureFlag::LceHostPrivileges, FeatureFlag::LceCraftingMenu][..],
        );
        let keys: Vec<&str> = set.keys().collect();
        assert_eq!(keys, vec!["lce_crafting_menu", "lce_host_privileges"]);
    }

    #[test]
    fn every_variant_is_listed_in_all_with_a_unique_key() {
        // Guards against adding a variant without updating `ALL`/`key`.
        assert_eq!(FeatureFlag::ALL.len(), 2);
        let mut keys: Vec<&str> = FeatureFlag::ALL.iter().map(|f| f.key()).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), FeatureFlag::ALL.len());
    }
}
