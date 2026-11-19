//! Game profiles: version-specific behavior as data (SPEC.md 6).
//!
//! Everything that differs between game versions is selected through a
//! [`Profile`]: typed [`FeatureFlag`]s, per-entry content availability
//! ([`ContentBounds`]), a pinned worldgen module name, protocol/data versions,
//! and world dimensions. Mechanics ask `profile.has(FeatureFlag::X)` at the
//! decision point; they never compare version numbers (CLAUDE.md rule 4).
//!
//! The baseline profile (`lce:141217`, see `docs/reference/baseline.md`) is
//! available from [`baseline`]. Backing fields are public data: profiles are
//! immutable descriptors, constructed in [`BASELINE_PROFILE`] or in tests.

use mclre_core::{Identifier, ParseError};

pub mod content;
pub mod feature;
pub mod id;
pub mod profile;

pub use content::{ContentBounds, ProfileOrder};
pub use feature::{FeatureFlag, FeatureFlagSet};
pub use id::ProfileId;
pub use profile::{Profile, ProfileConfig, ProfileError, baseline};

/// Worldgen module pinned by the baseline profile (implemented in
/// `mclre-worldgen` at M6).
pub const BASELINE_WORLDGEN_MODULE: &str = "lce141217";

/// Build height of the baseline world: blocks `0..=255` (reference
/// `Level.h:69-70`, `minBuildHeight = 0`, `maxBuildHeight = 256`).
pub const BASELINE_BUILD_HEIGHT: u32 = 256;

/// Terrain generation depth of the baseline world (reference `Level.h:71`
/// `genDepthBits = 7` → `genDepth = 128`).
pub const BASELINE_TERRAIN_HEIGHT: u32 = 128;

/// Sea level of the baseline world (reference `Level.h:74`
/// `constSeaLevel = genDepth / 2 - 1` = 63).
pub const BASELINE_SEA_LEVEL: u32 = 63;

/// Data version of the baseline save format (our schema, starting at 1; bumped
/// whenever `world`'s on-disk layout changes — SPEC.md 5.6).
pub const BASELINE_DATA_VERSION: u32 = 1;

/// Protocol version of the v1 wire protocol (SPEC.md 5.7); bumped on any
/// breaking packet change.
pub const BASELINE_PROTOCOL_VERSION: u32 = 1;

/// Convenience re-export so dependents can name [`ParseError`] without also
/// depending on `mclre-core`.
pub use mclre_core::ParseError as IdParseError;

/// Parses `input` as a [`ProfileId`] (thin alias for [`ProfileId::parse`]).
pub fn parse_profile_id(input: &str) -> Result<ProfileId, ParseError> {
    ProfileId::parse(input)
}

/// Whether `id` is a valid profile identifier.
pub fn is_valid_profile_id(id: &Identifier) -> bool {
    // Only namespace/path shape is enforced by `Identifier`; profile-specific
    // naming rules (e.g. `lce:tuNN`) are validated when those profiles exist.
    !id.namespace().is_empty() && !id.path().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_constants_are_self_consistent() {
        let profile = baseline();
        assert!(profile.sea_level < profile.terrain_height);
        assert!(profile.terrain_height < profile.build_height);
        assert_eq!(profile.sea_level, profile.terrain_height / 2 - 1);
    }

    #[test]
    fn baseline_profile_matches_constants() {
        let profile = baseline();
        assert_eq!(profile.id.as_str(), "lce:141217");
        assert_eq!(profile.build_height, BASELINE_BUILD_HEIGHT);
        assert_eq!(profile.terrain_height, BASELINE_TERRAIN_HEIGHT);
        assert_eq!(profile.sea_level, BASELINE_SEA_LEVEL);
        assert_eq!(profile.data_version, BASELINE_DATA_VERSION);
        assert_eq!(profile.protocol_version, BASELINE_PROTOCOL_VERSION);
        assert_eq!(profile.worldgen_module, BASELINE_WORLDGEN_MODULE);
    }

    #[test]
    fn profile_id_parse_and_display_round_trip() {
        let id = parse_profile_id("lce:tu19").expect("valid");
        assert_eq!(id.to_string(), "lce:tu19");
        assert_eq!(id.as_str(), "lce:tu19");
        assert!(is_valid_profile_id(&id.as_identifier()));
        assert!(parse_profile_id("tu19").is_err());
        assert!(parse_profile_id("LCE:TU19").is_err());
    }
}
