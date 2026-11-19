//! The [`Profile`] descriptor and the baseline profile.

use std::sync::LazyLock;

use thiserror::Error;

use crate::content::{ContentBounds, ProfileOrder};
use crate::feature::{FeatureFlag, FeatureFlagSet};
use crate::id::ProfileId;
use crate::{
    BASELINE_BUILD_HEIGHT, BASELINE_DATA_VERSION, BASELINE_PROTOCOL_VERSION, BASELINE_SEA_LEVEL,
    BASELINE_TERRAIN_HEIGHT, BASELINE_WORLDGEN_MODULE,
};

/// Fields used to construct a [`Profile`]; see [`Profile::new`] for validation.
#[derive(Debug, Clone)]
pub struct ProfileConfig {
    /// Identifies the profile (`lce:141217`, `java:1.8.9`, ...).
    pub id: ProfileId,
    /// Position in the version timeline (see [`ProfileOrder`]).
    pub order: ProfileOrder,
    /// Save-format schema version written by this profile.
    pub data_version: u32,
    /// Wire-protocol version this profile speaks.
    pub protocol_version: u32,
    /// Total build height in blocks (valid Y range is `0..build_height`).
    pub build_height: u32,
    /// Terrain generation depth in blocks (worldgen only fills below this).
    pub terrain_height: u32,
    /// Sea level in blocks.
    pub sea_level: u32,
    /// Name of the frozen worldgen module this profile pins.
    pub worldgen_module: &'static str,
    /// Optional mechanics enabled for this profile.
    pub flags: FeatureFlagSet,
}

/// Invalid profile configuration.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProfileError {
    /// Terrain generation depth exceeded the build height.
    #[error("profile `{id}`: terrain height {terrain_height} exceeds build height {build_height}")]
    TerrainAboveBuild {
        /// Profile id.
        id: String,
        /// Requested terrain height.
        terrain_height: u32,
        /// Requested build height.
        build_height: u32,
    },
    /// Sea level was not strictly inside the terrain depth.
    #[error("profile `{id}`: sea level {sea_level} must be below terrain height {terrain_height}")]
    InvalidSeaLevel {
        /// Profile id.
        id: String,
        /// Requested sea level.
        sea_level: u32,
        /// Requested terrain height.
        terrain_height: u32,
    },
    /// A dimension was zero.
    #[error("profile `{id}`: {field} must be greater than zero")]
    ZeroDimension {
        /// Profile id.
        id: String,
        /// Offending field name.
        field: &'static str,
    },
    /// A version number was zero.
    #[error("profile `{id}`: {field} must be at least 1")]
    ZeroVersion {
        /// Profile id.
        id: String,
        /// Offending field name.
        field: &'static str,
    },
}

/// An immutable game-version descriptor (SPEC.md 6.1).
///
/// Profiles are data, not code paths: mechanics read [`Profile::has`], content
/// tables read [`ContentBounds`], and worldgen is selected by
/// [`Profile::worldgen_module`]. Fields are public because a profile is a plain
/// value with no invariants beyond those checked in [`Profile::new`].
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    /// Identifies the profile.
    pub id: ProfileId,
    /// Position in the version timeline.
    pub order: ProfileOrder,
    /// Save-format schema version.
    pub data_version: u32,
    /// Wire-protocol version.
    pub protocol_version: u32,
    /// Build height in blocks (valid Y range is `0..build_height`).
    pub build_height: u32,
    /// Terrain generation depth in blocks.
    pub terrain_height: u32,
    /// Sea level in blocks.
    pub sea_level: u32,
    /// Frozen worldgen module name (resolved by `mclre-worldgen` at M6).
    pub worldgen_module: &'static str,
    /// Enabled optional mechanics.
    pub flags: FeatureFlagSet,
}

impl Profile {
    /// Builds and validates a profile.
    pub fn new(config: ProfileConfig) -> Result<Self, ProfileError> {
        let id = config.id.to_string();
        if config.build_height == 0 {
            return Err(ProfileError::ZeroDimension {
                id,
                field: "build_height",
            });
        }
        if config.terrain_height == 0 {
            return Err(ProfileError::ZeroDimension {
                id,
                field: "terrain_height",
            });
        }
        if config.terrain_height > config.build_height {
            return Err(ProfileError::TerrainAboveBuild {
                id,
                terrain_height: config.terrain_height,
                build_height: config.build_height,
            });
        }
        if config.sea_level >= config.terrain_height {
            return Err(ProfileError::InvalidSeaLevel {
                id,
                sea_level: config.sea_level,
                terrain_height: config.terrain_height,
            });
        }
        if config.data_version == 0 {
            return Err(ProfileError::ZeroVersion {
                id,
                field: "data_version",
            });
        }
        if config.protocol_version == 0 {
            return Err(ProfileError::ZeroVersion {
                id,
                field: "protocol_version",
            });
        }
        Ok(Self {
            id: config.id,
            order: config.order,
            data_version: config.data_version,
            protocol_version: config.protocol_version,
            build_height: config.build_height,
            terrain_height: config.terrain_height,
            sea_level: config.sea_level,
            worldgen_module: config.worldgen_module,
            flags: config.flags,
        })
    }

    /// Whether this profile enables `flag`.
    pub fn has(&self, flag: FeatureFlag) -> bool {
        self.flags.contains(flag)
    }

    /// Whether content with `bounds` exists in this profile.
    pub fn provides(&self, bounds: &ContentBounds) -> bool {
        bounds.contains(self.order)
    }

    /// Whether this profile is at least as new as `other`.
    pub fn is_at_least(&self, other: &Self) -> bool {
        self.order >= other.order
    }
}

/// The baseline profile: Legacy Console Edition as of 2014-12-17
/// (`docs/reference/baseline.md`).
///
/// Access it through [`baseline`], which initializes it once.
static BASELINE: LazyLock<Profile> = LazyLock::new(|| {
    Profile::new(ProfileConfig {
        id: ProfileId::parse("lce:141217").expect("baseline profile id is a valid identifier"),
        order: ProfileOrder::FIRST,
        data_version: BASELINE_DATA_VERSION,
        protocol_version: BASELINE_PROTOCOL_VERSION,
        build_height: BASELINE_BUILD_HEIGHT,
        terrain_height: BASELINE_TERRAIN_HEIGHT,
        sea_level: BASELINE_SEA_LEVEL,
        worldgen_module: BASELINE_WORLDGEN_MODULE,
        flags: FeatureFlagSet::from(
            &[FeatureFlag::LceCraftingMenu, FeatureFlag::LceHostPrivileges][..],
        ),
    })
    .expect("baseline profile constants are valid")
});

/// The baseline profile (initialized on first use).
pub fn baseline() -> &'static Profile {
    &BASELINE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> ProfileConfig {
        ProfileConfig {
            id: ProfileId::parse("test:profile").expect("valid"),
            order: ProfileOrder::new(7),
            data_version: 3,
            protocol_version: 2,
            build_height: 256,
            terrain_height: 128,
            sea_level: 63,
            worldgen_module: "test_gen",
            flags: FeatureFlagSet::new(),
        }
    }

    #[test]
    fn valid_config_builds_profile() {
        let profile = Profile::new(config()).expect("valid");
        assert_eq!(profile.id.as_str(), "test:profile");
        assert_eq!(profile.order, ProfileOrder::new(7));
        assert!(profile.flags.is_empty());
    }

    #[test]
    fn rejects_invalid_dimensions_and_versions() {
        let mut c = config();
        c.terrain_height = 300;
        assert!(matches!(
            Profile::new(c),
            Err(ProfileError::TerrainAboveBuild { .. })
        ));

        let mut c = config();
        c.sea_level = 128;
        assert!(matches!(
            Profile::new(c),
            Err(ProfileError::InvalidSeaLevel { .. })
        ));

        let mut c = config();
        c.build_height = 0;
        assert!(matches!(
            Profile::new(c),
            Err(ProfileError::ZeroDimension { .. })
        ));

        let mut c = config();
        c.data_version = 0;
        assert!(matches!(
            Profile::new(c),
            Err(ProfileError::ZeroVersion { .. })
        ));

        let mut c = config();
        c.protocol_version = 0;
        assert!(matches!(
            Profile::new(c),
            Err(ProfileError::ZeroVersion { .. })
        ));
    }

    #[test]
    fn baseline_enables_lce_only_mechanics() {
        let profile = baseline();
        assert!(profile.has(FeatureFlag::LceCraftingMenu));
        assert!(profile.has(FeatureFlag::LceHostPrivileges));
        assert_eq!(profile.order, ProfileOrder::FIRST);
        assert!(!profile.flags.is_empty());
    }

    #[test]
    fn content_bounds_are_evaluated_against_profile_order() {
        let profile = baseline();
        assert!(profile.provides(&ContentBounds::always()));
        assert!(profile.provides(&ContentBounds::until(ProfileOrder::FIRST)));
        assert!(!profile.provides(&ContentBounds::since(ProfileOrder::new(1))));
    }

    #[test]
    fn ordering_compares_timeline_position_not_id_text() {
        let mut later = config();
        later.id = ProfileId::parse("java:1.8.9").expect("valid");
        later.order = ProfileOrder::new(1);
        let later = Profile::new(later).expect("valid");
        let earliest = baseline(); // order 0
        assert!(later.is_at_least(earliest));
        assert!(!earliest.is_at_least(&later));
        assert!(earliest.is_at_least(earliest));
    }
}
