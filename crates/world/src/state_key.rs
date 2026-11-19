//! Namespaced block-state keys for save palettes (SPEC.md 5.2–5.3).
//!
//! Saves never contain numeric state ids: chunk palettes store
//! `{ Name: "minecraft:oak_log", Properties: { axis: "y" } }`, exactly like
//! Java Edition Anvil files. [`StateKey`] is that representation;
//! [`StateCodec`] converts between it and [`BlockStateId`], and is implemented
//! for [`BlockStateModel`] so any content set (vanilla plus mods) works.
//!
//! Properties are kept in declaration order (the canonical order of the block
//! state model), which is deterministic and matches how the state table was
//! enumerated. Lookup is by name, so saves written by other tools with a
//! different property order still load.

use std::collections::BTreeMap;
use std::fmt;

use mclre_core::{BlockStateId, BlockStateModel, PropertyDefinition, PropertyValue};

/// One palette entry: a namespaced block name plus its property values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateKey {
    /// Namespaced block identifier (`minecraft:stone`).
    pub name: String,
    /// Properties in the block's canonical declaration order.
    pub properties: Vec<(String, String)>,
}

impl StateKey {
    /// A key with no properties.
    pub fn stateless(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            properties: Vec::new(),
        }
    }

    /// A key with properties (callers pass canonical order).
    pub fn new(
        name: impl Into<String>,
        properties: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        Self {
            name: name.into(),
            properties: properties
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        }
    }

    /// The value of a property, if present.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

impl fmt::Display for StateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)?;
        if !self.properties.is_empty() {
            f.write_str("[")?;
            for (i, (key, value)) in self.properties.iter().enumerate() {
                if i > 0 {
                    f.write_str(",")?;
                }
                write!(f, "{key}={value}")?;
            }
            f.write_str("]")?;
        }
        Ok(())
    }
}

/// Conversion between block-state ids and namespaced palette keys.
///
/// Implemented for [`BlockStateModel`]. Save code takes `&dyn StateCodec`, so
/// tests can use tiny fake tables and mods are covered automatically.
pub trait StateCodec {
    /// The palette key for a state, or `None` if the state is unknown.
    fn key_of(&self, state: BlockStateId) -> Option<StateKey>;

    /// The state for a palette key, or `None` if it is not a known state.
    fn state_of(&self, key: &StateKey) -> Option<BlockStateId>;

    /// Number of block states in this model.
    ///
    /// Save formats need it to derive the implicit bit width of direct (global
    /// palette) sections; it must be stable for a given content set.
    fn state_count(&self) -> u32;
}

/// Renders a property value the way saves express it (`true`, `7`, `top`).
pub fn value_to_string(value: &PropertyValue) -> String {
    match value {
        PropertyValue::Bool(value) => value.to_string(),
        PropertyValue::Int(value) => value.to_string(),
        PropertyValue::Enum(value) => value.to_string(),
    }
}

/// Parses a save property string into a typed value.
fn value_from_string(definition: &PropertyDefinition, raw: &str) -> Option<PropertyValue> {
    match definition {
        PropertyDefinition::Bool { .. } => match raw {
            "true" => Some(PropertyValue::Bool(true)),
            "false" => Some(PropertyValue::Bool(false)),
            _ => None,
        },
        PropertyDefinition::Int { min, max, .. } => {
            let value: i32 = raw.parse().ok()?;
            (*min..=*max)
                .contains(&value)
                .then_some(PropertyValue::Int(value))
        }
        PropertyDefinition::Enum { values, .. } => values
            .iter()
            .find(|value| value.as_ref() == raw)
            .map(|_| PropertyValue::Enum(raw.into())),
    }
}

impl StateCodec for BlockStateModel {
    fn key_of(&self, state: BlockStateId) -> Option<StateKey> {
        let block = self.block_of(state)?;
        let name = self.registry().id_by_index(block.0)?.to_string();
        let properties = self
            .registry()
            .by_index(block.0)?
            .properties
            .iter()
            .map(PropertyDefinition::name)
            .zip(self.values_of(state)?.iter())
            .map(|(name, value)| (name.to_string(), value_to_string(value)))
            .collect();
        Some(StateKey { name, properties })
    }

    fn state_of(&self, key: &StateKey) -> Option<BlockStateId> {
        let identifier = mclre_core::Identifier::parse(&key.name).ok()?;
        let (properties, index) = self.registry().get(&identifier)?;
        let mut values = Vec::with_capacity(properties.properties.len());
        let mut seen = BTreeMap::new();
        for (name, value) in &key.properties {
            if seen.insert(name.as_str(), value.as_str()).is_some() {
                // Duplicate property in the palette entry: reject, never guess.
                return None;
            }
        }
        for definition in &properties.properties {
            let raw = seen.remove(definition.name())?;
            values.push(value_from_string(definition, raw)?);
        }
        if !seen.is_empty() {
            // Unknown properties would be silently lost; refuse instead.
            return None;
        }
        self.state_from_values(mclre_core::BlockId(index), &values)
            .ok()
    }

    fn state_count(&self) -> u32 {
        BlockStateModel::state_count(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mclre_core::{BlockProperties, PropertyDefinition, Registry};

    fn model() -> BlockStateModel {
        let mut registry = Registry::new("minecraft:block");
        registry
            .register(
                mclre_core::Identifier::parse("minecraft:air").expect("valid"),
                BlockProperties::none(),
            )
            .expect("register");
        registry
            .register(
                mclre_core::Identifier::parse("minecraft:oak_log").expect("valid"),
                BlockProperties::new(vec![PropertyDefinition::enumeration(
                    "axis",
                    ["x", "y", "z"],
                )])
                .expect("valid"),
            )
            .expect("register");
        registry
            .register(
                mclre_core::Identifier::parse("minecraft:furnace").expect("valid"),
                BlockProperties::new(vec![
                    PropertyDefinition::enumeration("facing", ["north", "south", "west", "east"]),
                    PropertyDefinition::bool("lit"),
                ])
                .expect("valid"),
            )
            .expect("register");
        BlockStateModel::freeze(registry).expect("freeze")
    }

    #[test]
    fn every_state_round_trips() {
        let model = model();
        for raw in 0..model.state_count() {
            let state = BlockStateId(raw);
            let key = model.key_of(state).expect("known state");
            let back = model.state_of(&key).expect("round trip");
            assert_eq!(back, state, "failed for {key}");
        }
    }

    #[test]
    fn keys_have_namespaced_names_and_canonical_property_order() {
        let model = model();
        let furnace = mclre_core::Identifier::parse("minecraft:furnace").expect("valid");
        let (_, block) = model.registry().get(&furnace).expect("registered");
        let state = model
            .default_state(mclre_core::BlockId(block))
            .expect("has states");
        let key = model.key_of(state).expect("known");
        assert_eq!(key.name, "minecraft:furnace");
        assert_eq!(
            key.properties,
            vec![
                ("facing".to_string(), "north".to_string()),
                ("lit".to_string(), "false".to_string())
            ]
        );
        assert_eq!(key.to_string(), "minecraft:furnace[facing=north,lit=false]");
    }

    #[test]
    fn lookup_accepts_any_property_order() {
        let model = model();
        let key = StateKey::new("minecraft:furnace", [("lit", "true"), ("facing", "east")]);
        let state = model.state_of(&key).expect("known state");
        let back = model.key_of(state).expect("known");
        assert_eq!(back.get("facing"), Some("east"));
        assert_eq!(back.get("lit"), Some("true"));
    }

    #[test]
    fn unknown_blocks_properties_and_values_are_rejected() {
        let model = model();
        assert!(
            model
                .state_of(&StateKey::stateless("minecraft:not_a_block"))
                .is_none()
        );
        assert!(
            model
                .state_of(&StateKey::new("minecraft:oak_log", [("axis", "w")]))
                .is_none()
        );
        assert!(
            model
                .state_of(&StateKey::new("minecraft:oak_log", [("nope", "1")]))
                .is_none()
        );
        // Extra unknown properties are refused rather than dropped.
        assert!(
            model
                .state_of(&StateKey::new(
                    "minecraft:oak_log",
                    [("axis", "y"), ("mystery", "1")]
                ))
                .is_none()
        );
        // Duplicate properties are refused.
        assert!(
            model
                .state_of(&StateKey::new(
                    "minecraft:oak_log",
                    [("axis", "y"), ("axis", "x")]
                ))
                .is_none()
        );
        // Missing properties are refused.
        assert!(
            model
                .state_of(&StateKey::new("minecraft:furnace", [("facing", "north")]))
                .is_none()
        );
        // Out-of-range integers are refused.
        assert!(
            model
                .state_of(&StateKey::new("minecraft:oak_log", [("axis", "4")]))
                .is_none()
        );
    }
}
