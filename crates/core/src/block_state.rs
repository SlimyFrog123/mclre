//! Block model: `(block, typed properties) → dense BlockStateId` (SPEC.md 5.3).
//!
//! Blocks register with property declarations (bool / int-range / enum). At
//! registry freeze the full state table is enumerated in a deterministic order
//! — blocks in registration order, properties in declaration order, values in
//! canonical order (false<true; min..max ascending; enum order as declared) —
//! producing a globally dense, reproducible `BlockStateId` space.
//!
//! Legacy `(id, meta)` mapping lives only in generated `data/legacy/*.toml`
//! (importer/parity), never here.

// All casts here operate on indices bounded by declared content sizes
// (properties per block, states per block); no untrusted input reaches them.
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_lossless)]

use thiserror::Error;

use crate::registry::{Registry, RegistryError};

// ---------------------------------------------------------------------------
// Property definitions
// ---------------------------------------------------------------------------

/// A declared block property (bool / int-range / enum, SPEC.md 5.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyDefinition {
    /// A boolean property.
    Bool {
        /// Property name (validated like a path segment).
        name: Box<str>,
    },
    /// An integer property over an inclusive range.
    Int {
        /// Property name.
        name: Box<str>,
        /// Minimum value (inclusive).
        min: i32,
        /// Maximum value (inclusive).
        max: i32,
    },
    /// An enum property with a fixed ordered value list.
    Enum {
        /// Property name.
        name: Box<str>,
        /// Values in canonical order (each a lowercase identifier-like token).
        values: Vec<Box<str>>,
    },
}

/// A concrete property value supplied at state lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyValue {
    /// Boolean value.
    Bool(bool),
    /// Integer value.
    Int(i32),
    /// Enum value (string token).
    Enum(Box<str>),
}

/// Errors from property/state computation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StateError {
    /// An integer property was declared with min > max.
    #[error("property `{name}` has invalid int range {min}..={max}")]
    InvalidIntRange {
        /// Property name.
        name: String,
        /// Declared minimum.
        min: i32,
        /// Declared maximum.
        max: i32,
    },
    /// An enum property was declared with no values.
    #[error("enum property `{name}` has no values")]
    EmptyEnum {
        /// Property name.
        name: String,
    },
    /// A property name was reused within one block.
    #[error("duplicate property name `{name}` in block")]
    DuplicatePropertyName {
        /// Property name.
        name: String,
    },
    /// The supplied value's kind does not match the property kind.
    #[error("property `{name}` received a value of the wrong kind")]
    KindMismatch {
        /// Property name.
        name: String,
    },
    /// The supplied value is outside the property's domain.
    #[error("property `{name}` received out-of-range value {value}")]
    ValueOutOfRange {
        /// Property name.
        name: String,
        /// The offending value rendered as a string.
        value: String,
    },
    /// The value list length did not match the property count.
    #[error("expected {expected} property values, got {got}")]
    WrongValueCount {
        /// Expected count.
        expected: usize,
        /// Supplied count.
        got: usize,
    },
    /// Freezing produced an error from the underlying registry.
    #[error(transparent)]
    Registry(#[from] RegistryError),
}

impl PropertyDefinition {
    /// A boolean property (`false` < `true`).
    pub fn bool(name: impl Into<Box<str>>) -> Self {
        Self::Bool { name: name.into() }
    }

    /// An inclusive integer property (`min..=max`).
    pub fn int(name: impl Into<Box<str>>, min: i32, max: i32) -> Self {
        Self::Int {
            name: name.into(),
            min,
            max,
        }
    }

    /// An enum property with values in canonical order (first = default).
    ///
    /// Values are matched exactly (lowercase tokens by convention); the order
    /// given here defines the state-index order and must never be reordered
    /// once saves exist.
    pub fn enumeration(
        name: impl Into<Box<str>>,
        values: impl IntoIterator<Item = impl Into<Box<str>>>,
    ) -> Self {
        Self::Enum {
            name: name.into(),
            values: values.into_iter().map(Into::into).collect(),
        }
    }

    /// The property's name.
    pub fn name(&self) -> &str {
        match self {
            Self::Bool { name } | Self::Int { name, .. } | Self::Enum { name, .. } => name,
        }
    }

    /// Number of distinct values the property can take.
    ///
    /// # Panics
    /// Only if the int span overflows `usize` (a caller bug — an `i32` range
    /// always fits in 64-bit `usize`).
    pub fn value_count(&self) -> usize {
        match self {
            Self::Bool { .. } => 2,
            Self::Int { min, max, .. } => usize::try_from(i64::from(*max) - i64::from(*min) + 1)
                .expect("int property span exceeds usize"),
            Self::Enum { values, .. } => values.len(),
        }
    }

    /// Maps a [`PropertyValue`] to its canonical index, or an error.
    pub fn index_of(&self, value: &PropertyValue) -> Result<usize, StateError> {
        let name = self.name();
        let mismatch = || StateError::KindMismatch {
            name: name.to_string(),
        };
        let out_of_range = |v: String| StateError::ValueOutOfRange {
            name: name.to_string(),
            value: v,
        };
        match (self, value) {
            (Self::Bool { .. }, PropertyValue::Bool(b)) => Ok(usize::from(*b)),
            (Self::Int { min, max, .. }, PropertyValue::Int(i)) => {
                if *i >= *min && *i <= *max {
                    Ok((*i - *min) as usize)
                } else {
                    Err(out_of_range(i.to_string()))
                }
            }
            (Self::Enum { values, .. }, PropertyValue::Enum(v)) => values
                .iter()
                .position(|candidate| candidate == v)
                .ok_or_else(|| out_of_range(v.to_string())),
            _ => Err(mismatch()),
        }
    }

    /// The canonical value at `index`.
    ///
    /// # Panics
    /// If `index >= value_count()` (a caller bug; state enumeration never
    /// exceeds the declared domain).
    pub fn value_at(&self, index: usize) -> PropertyValue {
        match self {
            Self::Bool { .. } => PropertyValue::Bool(index == 1),
            Self::Int { min, .. } => PropertyValue::Int(*min + index as i32),
            Self::Enum { values, .. } => PropertyValue::Enum(values[index].clone()),
        }
    }
}

// ---------------------------------------------------------------------------
// Block properties
// ---------------------------------------------------------------------------

/// The property set of one block, declared at registration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlockProperties {
    /// Declared properties, in declaration order (canonical enumeration order).
    pub properties: Vec<PropertyDefinition>,
}

impl BlockProperties {
    /// Creates a property-less block (e.g. stone-like).
    pub fn none() -> Self {
        Self::default()
    }

    /// Creates a block from properties, validating names and domains.
    pub fn new(properties: Vec<PropertyDefinition>) -> Result<Self, StateError> {
        for (i, p) in properties.iter().enumerate() {
            match p {
                PropertyDefinition::Int { name, min, max } if min > max => {
                    return Err(StateError::InvalidIntRange {
                        name: name.to_string(),
                        min: *min,
                        max: *max,
                    });
                }
                PropertyDefinition::Enum { name, values } if values.is_empty() => {
                    return Err(StateError::EmptyEnum {
                        name: name.to_string(),
                    });
                }
                _ => {}
            }
            if properties[i + 1..].iter().any(|q| q.name() == p.name()) {
                return Err(StateError::DuplicatePropertyName {
                    name: p.name().to_string(),
                });
            }
        }
        Ok(Self { properties })
    }

    /// Total number of states this block has (product of value counts).
    pub fn state_count(&self) -> usize {
        self.properties
            .iter()
            .map(PropertyDefinition::value_count)
            .product()
    }
}

// ---------------------------------------------------------------------------
// Frozen state table
// ---------------------------------------------------------------------------

/// A globally dense block-state index (newtype; numeric only inside this
/// module, chunk palettes, and protocol palettes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockStateId(pub u32);

/// A block's registry index (dense; SPEC.md 5.3 `BlockId`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockId(pub u32);

/// The frozen block-state table: block registry + dense state enumeration.
pub struct BlockStateModel {
    blocks: crate::registry::FrozenRegistry<BlockProperties>,
    /// First `BlockStateId` of each block, indexed by block index.
    state_offsets: Vec<u32>,
    /// Block index of each state, indexed by `state - first_state`.
    state_block: Vec<u32>,
    state_count: u32,
}

impl BlockStateModel {
    /// Consumes a registration-phase block registry and enumerates all states
    /// deterministically (registration order × declaration order × canonical
    /// value order).
    ///
    /// # Panics
    /// Only if the total state space exceeds `u32::MAX` (a caller bug; no
    /// real content set approaches 4 billion states).
    pub fn freeze(registry: Registry<BlockProperties>) -> Result<Self, StateError> {
        let frozen = registry.freeze()?;
        let mut state_offsets = Vec::with_capacity(frozen.len());
        let mut state_block = Vec::new();
        let mut next = 0u32;
        for (block_index, props) in frozen.values().iter().enumerate() {
            state_offsets.push(next);
            let count = props.state_count();
            // Registration is bounded by content sets, not untrusted input.
            next = next
                .checked_add(u32::try_from(count).expect("state count overflow"))
                .expect("state space overflow");
            for _ in 0..count {
                state_block.push(block_index as u32);
            }
        }
        Ok(Self {
            blocks: frozen,
            state_offsets,
            state_block,
            state_count: next,
        })
    }

    /// Total number of block states.
    pub fn state_count(&self) -> u32 {
        self.state_count
    }

    /// Number of blocks.
    pub fn block_count(&self) -> u32 {
        self.blocks.len() as u32
    }

    /// Computes the state id for a block (by index) given canonical property
    /// value indices (one per declared property, in declaration order).
    ///
    /// # Panics
    /// Only if `value_indices.len()` mismatches the property count and debug
    /// assertions are on (a caller bug).
    pub fn state_from_indices(&self, block: BlockId, value_indices: &[usize]) -> BlockStateId {
        let props = self.blocks.by_index(block.0).expect("valid block index");
        debug_assert_eq!(value_indices.len(), props.properties.len());
        let base = self.state_offsets[block.0 as usize];
        let mut offset = 0usize;
        for (prop, &idx) in props.properties.iter().zip(value_indices) {
            offset = offset * prop.value_count() + idx;
        }
        BlockStateId(base + offset as u32)
    }

    /// Computes the state id for a block given concrete property values
    /// (length and kinds must match the declarations).
    pub fn state_from_values(
        &self,
        block: BlockId,
        values: &[PropertyValue],
    ) -> Result<BlockStateId, StateError> {
        let props = self
            .blocks
            .by_index(block.0)
            .ok_or(StateError::WrongValueCount {
                expected: 0,
                got: values.len(),
            })?;
        if values.len() != props.properties.len() {
            return Err(StateError::WrongValueCount {
                expected: props.properties.len(),
                got: values.len(),
            });
        }
        let base = self.state_offsets[block.0 as usize];
        let mut offset = 0usize;
        for (prop, value) in props.properties.iter().zip(values) {
            offset = offset * prop.value_count() + prop.index_of(value)?;
        }
        Ok(BlockStateId(base + offset as u32))
    }

    /// The block index a state belongs to.
    pub fn block_of(&self, state: BlockStateId) -> Option<BlockId> {
        if state.0 >= self.state_count {
            return None;
        }
        Some(BlockId(self.state_block[state.0 as usize]))
    }

    /// The first state id of a block (its default state).
    pub fn default_state(&self, block: BlockId) -> Option<BlockStateId> {
        self.state_offsets
            .get(block.0 as usize)
            .map(|&o| BlockStateId(o))
    }

    /// The concrete property values of a state, in declaration order.
    pub fn values_of(&self, state: BlockStateId) -> Option<Vec<PropertyValue>> {
        let block = self.block_of(state)?;
        let props = self.blocks.by_index(block.0)?;
        let mut offset = (state.0 - self.state_offsets[block.0 as usize]) as usize;
        // Mixed-radix decode: last property is the least significant digit.
        let mut indices = vec![0usize; props.properties.len()];
        for (slot, prop) in indices.iter_mut().zip(&props.properties).rev() {
            *slot = offset % prop.value_count();
            offset /= prop.value_count();
        }
        Some(
            props
                .properties
                .iter()
                .zip(indices)
                .map(|(prop, idx)| prop.value_at(idx))
                .collect(),
        )
    }

    /// The block registry (identifiers + properties).
    pub fn registry(&self) -> &crate::registry::FrozenRegistry<BlockProperties> {
        &self.blocks
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identifier::Identifier;

    fn bool_prop(name: &str) -> PropertyDefinition {
        PropertyDefinition::Bool { name: name.into() }
    }

    fn enum_prop(name: &str, values: &[&str]) -> PropertyDefinition {
        PropertyDefinition::Enum {
            name: name.into(),
            values: values.iter().map(|v| (*v).into()).collect(),
        }
    }

    fn int_prop(name: &str, min: i32, max: i32) -> PropertyDefinition {
        PropertyDefinition::Int {
            name: name.into(),
            min,
            max,
        }
    }

    fn sample_model() -> BlockStateModel {
        let mut reg: Registry<BlockProperties> = Registry::new("blocks");
        // stone: no properties → exactly one state.
        reg.register(
            Identifier::parse_with_default_namespace("stone").expect("valid"),
            BlockProperties::none(),
        )
        .expect("ok");
        // oak_log: axis enum (3 states).
        reg.register(
            Identifier::parse_with_default_namespace("oak_log").expect("valid"),
            BlockProperties::new(vec![enum_prop("axis", &["x", "y", "z"])]).expect("valid"),
        )
        .expect("ok");
        // comparator: mixed bool + enum + int.
        reg.register(
            Identifier::parse_with_default_namespace("comparator").expect("valid"),
            BlockProperties::new(vec![
                bool_prop("powered"),
                enum_prop("facing", &["north", "south"]),
                int_prop("delay", 1, 4),
            ])
            .expect("valid"),
        )
        .expect("ok");
        BlockStateModel::freeze(reg).expect("freeze")
    }

    #[test]
    fn propertyless_block_has_exactly_one_state() {
        let model = sample_model();
        assert_eq!(model.block_count(), 3);
        // stone=0 (1 state), oak_log=1 (3), comparator=2 (2*2*4=16).
        assert_eq!(model.state_count(), 20);
        assert_eq!(model.default_state(BlockId(0)), Some(BlockStateId(0)));
        assert_eq!(model.block_of(BlockStateId(0)), Some(BlockId(0)));
    }

    #[test]
    fn dense_contiguous_enumeration_in_registration_order() {
        let model = sample_model();
        assert_eq!(model.default_state(BlockId(1)), Some(BlockStateId(1)));
        assert_eq!(model.default_state(BlockId(2)), Some(BlockStateId(4)));
        // Last state belongs to the last block.
        assert_eq!(model.block_of(BlockStateId(19)), Some(BlockId(2)));
        assert!(model.block_of(BlockStateId(20)).is_none());
    }

    #[test]
    fn state_round_trips_through_values() {
        let model = sample_model();
        // comparator: powered=false, facing=south, delay=3
        let state = model
            .state_from_values(
                BlockId(2),
                &[
                    PropertyValue::Bool(false),
                    PropertyValue::Enum("south".into()),
                    PropertyValue::Int(3),
                ],
            )
            .expect("valid values");
        assert_eq!(model.block_of(state), Some(BlockId(2)));
        assert_eq!(
            model.values_of(state),
            Some(vec![
                PropertyValue::Bool(false),
                PropertyValue::Enum("south".into()),
                PropertyValue::Int(3),
            ])
        );
    }

    #[test]
    fn value_ordering_is_canonical() {
        let model = sample_model();
        // In declaration order with all "first" values → default state.
        let first = model
            .state_from_values(
                BlockId(2),
                &[
                    PropertyValue::Bool(false),
                    PropertyValue::Enum("north".into()),
                    PropertyValue::Int(1),
                ],
            )
            .expect("valid");
        assert_eq!(Some(first), model.default_state(BlockId(2)));
        // Enum "south" (second value) with delay=4 (last) → last state of block.
        let last = model
            .state_from_values(
                BlockId(2),
                &[
                    PropertyValue::Bool(true),
                    PropertyValue::Enum("south".into()),
                    PropertyValue::Int(4),
                ],
            )
            .expect("valid");
        assert_eq!(last, BlockStateId(19));
    }

    #[test]
    fn out_of_range_and_mismatched_kinds_are_rejected() {
        let model = sample_model();
        // delay=5 is out of range.
        assert!(matches!(
            model.state_from_values(
                BlockId(2),
                &[
                    PropertyValue::Bool(false),
                    PropertyValue::Enum("north".into()),
                    PropertyValue::Int(5),
                ],
            ),
            Err(StateError::ValueOutOfRange { .. })
        ));
        // Wrong kind: bool where enum expected.
        assert!(matches!(
            model.state_from_values(BlockId(1), &[PropertyValue::Bool(true)],),
            Err(StateError::KindMismatch { .. })
        ));
        // Wrong length.
        assert!(matches!(
            model.state_from_values(BlockId(2), &[PropertyValue::Bool(false)]),
            Err(StateError::WrongValueCount { .. })
        ));
    }

    #[test]
    fn invalid_declarations_are_rejected() {
        assert!(BlockProperties::new(vec![int_prop("a", 4, 1)]).is_err());
        assert!(BlockProperties::new(vec![enum_prop("a", &[])]).is_err());
        assert!(BlockProperties::new(vec![bool_prop("a"), int_prop("a", 0, 1)]).is_err());
    }

    #[test]
    fn duplicate_block_registration_propagates() {
        let mut reg: Registry<BlockProperties> = Registry::new("blocks");
        let id = Identifier::parse_with_default_namespace("stone").expect("valid");
        reg.register(id, BlockProperties::none()).expect("first");
        reg.register(id, BlockProperties::none()).expect_err("dup");
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use crate::identifier::Identifier;
    use proptest::prelude::*;

    fn model_with_two_blocks() -> BlockStateModel {
        let mut reg: Registry<BlockProperties> = Registry::new("blocks");
        reg.register(
            Identifier::parse_with_default_namespace("a").expect("valid"),
            BlockProperties::new(vec![
                PropertyDefinition::Bool { name: "p".into() },
                PropertyDefinition::Int {
                    name: "q".into(),
                    min: -2,
                    max: 5,
                },
            ])
            .expect("valid"),
        )
        .expect("ok");
        reg.register(
            Identifier::parse_with_default_namespace("b").expect("valid"),
            BlockProperties::new(vec![PropertyDefinition::Enum {
                name: "r".into(),
                values: vec!["x".into(), "y".into(), "z".into()],
            }])
            .expect("valid"),
        )
        .expect("ok");
        BlockStateModel::freeze(reg).expect("freeze")
    }

    proptest! {
        /// Injectivity: each (block, values) pair maps to a unique state, and
        /// decoding recovers the values (round-trip) for the whole table.
        #[test]
        fn table_is_injective_and_round_trips(
            a_p in proptest::bool::ANY,
            a_q in -2i32..=5,
            b_r in 0usize..3,
        ) {
            let model = model_with_two_blocks();
            // Block A: encode from concrete values.
            let s_a = model
                .state_from_values(
                    BlockId(0),
                    &[PropertyValue::Bool(a_p), PropertyValue::Int(a_q)],
                )
                .expect("in range");
            assert_eq!(model.block_of(s_a), Some(BlockId(0)));
            prop_assert_eq!(
                model.values_of(s_a),
                Some(vec![PropertyValue::Bool(a_p), PropertyValue::Int(a_q)])
            );
            // Block B via canonical indices.
            let s_b = model.state_from_indices(BlockId(1), &[b_r]);
            assert_eq!(model.block_of(s_b), Some(BlockId(1)));
            prop_assert_eq!(
                model.values_of(s_b),
                Some(vec![PropertyValue::Enum(match b_r {
                    0 => "x".into(),
                    1 => "y".into(),
                    _ => "z".into(),
                })])
            );
            // Distinct states across blocks.
            prop_assert_ne!(s_a, s_b);
        }
    }
}
