//! `level.dat` NBT structure and (de)serialization.
//!
//! The `level.dat` file stores world metadata: data version, profile, world
//! size preset, seed, dimension data, and game rules.

use mclre_core::nbt::{Compound, Value};

use crate::preset::WorldSizePreset;

/// The current MCLRE data version written to saves.
pub const DATA_VERSION: i32 = 1;

/// The `level.dat` NBT structure.
#[derive(Debug, Clone, PartialEq)]
pub struct LevelDat {
    /// The data version of this save.
    pub data_version: i32,
    /// The game profile ID used to generate this world.
    pub profile_id: String,
    /// The world size preset.
    pub world_size_preset: WorldSizePreset,
    /// The world seed.
    pub seed: i64,
    /// Dimension data (Overworld, Nether, End, etc.).
    pub dimensions: Vec<DimensionData>,
    /// Game rules.
    pub game_rules: Compound,
}

impl LevelDat {
    /// Creates a new `LevelDat` for a fresh world.
    pub fn new(profile_id: impl Into<String>, preset: WorldSizePreset, seed: i64) -> Self {
        Self {
            data_version: DATA_VERSION,
            profile_id: profile_id.into(),
            world_size_preset: preset,
            seed,
            dimensions: vec![
                DimensionData::overworld(),
                DimensionData::nether(),
                DimensionData::the_end(),
            ],
            game_rules: Compound::new(),
        }
    }

    /// Encodes to the NBT compound expected in `level.dat`.
    pub fn to_compound(&self) -> Compound {
        let mut root = Compound::new();
        root.insert("DataVersion", Value::Int(self.data_version));
        root.insert("profile_id", Value::String(self.profile_id.clone()));
        root.insert(
            "world_size_preset",
            Value::String(self.world_size_preset.to_string()),
        );
        root.insert("seed", Value::Long(self.seed));
        root.insert(
            "dimensions",
            Value::List(
                self.dimensions
                    .iter()
                    .map(|d| Value::Compound(d.to_compound()))
                    .collect(),
            ),
        );
        root.insert("game_rules", Value::Compound(self.game_rules.clone()));
        root
    }

    /// Decodes from the NBT compound in `level.dat`.
    pub fn from_compound(compound: &Compound) -> Result<Self, LevelDatError> {
        let data_version = compound
            .get_int("DataVersion")
            .ok_or(LevelDatError::Missing("DataVersion"))? as i32;

        if data_version > DATA_VERSION {
            return Err(LevelDatError::UnsupportedDataVersion(data_version));
        }

        let profile_id = compound
            .get_str("profile_id")
            .ok_or(LevelDatError::Missing("profile_id"))?
            .to_string();

        let world_size_preset = compound
            .get_str("world_size_preset")
            .ok_or(LevelDatError::Missing("world_size_preset"))?
            .parse()
            .map_err(|e| LevelDatError::InvalidValue(format!("world_size_preset: {e}")))?;

        let seed = compound
            .get_int("seed")
            .ok_or(LevelDatError::Missing("seed"))?;

        let dimensions = compound
            .get_list("dimensions")
            .ok_or(LevelDatError::Missing("dimensions"))?
            .iter()
            .filter_map(|v| match v {
                mclre_core::nbt::Value::Compound(c) => Some(c),
                _ => None,
            })
            .map(DimensionData::from_compound)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| LevelDatError::InvalidValue(format!("dimension: {e}")))?;

        let game_rules = compound
            .get_compound("game_rules")
            .cloned()
            .unwrap_or_default();

        Ok(Self {
            data_version,
            profile_id,
            world_size_preset,
            seed,
            dimensions,
            game_rules,
        })
    }
}

/// Errors from `level.dat` encoding/decoding.
#[derive(Debug, thiserror::Error)]
pub enum LevelDatError {
    #[error("missing field: {0}")]
    Missing(&'static str),
    #[error("unsupported data version {0}")]
    UnsupportedDataVersion(i32),
    #[error("invalid value: {0}")]
    InvalidValue(String),
}

/// Per-dimension data stored in `level.dat`.
#[derive(Debug, Clone, PartialEq)]
pub struct DimensionData {
    /// The dimension namespace (e.g., "minecraft:overworld").
    pub namespace: String,
    /// The dimension type ID.
    pub dimension_type: String,
    /// The generator settings (serialized).
    pub generator: Compound,
}

impl DimensionData {
    /// Overworld dimension.
    pub fn overworld() -> Self {
        Self {
            namespace: "minecraft:overworld".into(),
            dimension_type: "minecraft:overworld".into(),
            generator: Compound::new(),
        }
    }

    /// Nether dimension.
    pub fn nether() -> Self {
        Self {
            namespace: "minecraft:the_nether".into(),
            dimension_type: "minecraft:the_nether".into(),
            generator: Compound::new(),
        }
    }

    /// The End dimension.
    pub fn the_end() -> Self {
        Self {
            namespace: "minecraft:the_end".into(),
            dimension_type: "minecraft:the_end".into(),
            generator: Compound::new(),
        }
    }

    fn to_compound(&self) -> Compound {
        let mut c = Compound::new();
        c.insert("namespace", Value::String(self.namespace.clone()));
        c.insert("dimension_type", Value::String(self.dimension_type.clone()));
        c.insert("generator", Value::Compound(self.generator.clone()));
        c
    }

    fn from_compound(compound: &Compound) -> std::result::Result<Self, String> {
        let namespace = compound
            .get_str("namespace")
            .ok_or("missing namespace")?
            .to_string();
        let dimension_type = compound
            .get_str("dimension_type")
            .ok_or("missing dimension_type")?
            .to_string();
        let generator = compound
            .get_compound("generator")
            .cloned()
            .unwrap_or_default();
        Ok(Self {
            namespace,
            dimension_type,
            generator,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_dat_round_trip() {
        let original = LevelDat::new("test_profile", WorldSizePreset::Classic, 12345);
        let compound = original.to_compound();
        let decoded = LevelDat::from_compound(&compound).expect("decode");
        assert_eq!(decoded, original);
    }

    #[test]
    fn level_dat_rejects_future_versions() {
        let mut c = LevelDat::new("p", WorldSizePreset::Infinite, 0).to_compound();
        c.insert("DataVersion", Value::Int(DATA_VERSION + 1));
        assert!(matches!(
            LevelDat::from_compound(&c),
            Err(LevelDatError::UnsupportedDataVersion(_))
        ));
    }
}
