//! Baseline (vanilla) block content, registered through the public API.
//!
//! "Vanilla is a mod" (SPEC.md 2.5): these definitions use exactly the same
//! registry/property API that third-party content will use — there is no
//! privileged path. Identifiers follow the namespace policy (ADR 0003):
//! `minecraft:` where a Java Edition id exists, `lce:` only for LCE-exclusive
//! content.
//!
//! This is the M1 slice: a representative set covering every property shape
//! (stateless, bool, int range, enum, combinations). Full baseline content
//! (hardness, drops, models, item forms) lands at M5, driven by the tables
//! extracted from `reference/lce-141217/`.

use mclre_core::{
    BlockProperties, BlockStateModel, Identifier, ParseError, PropertyDefinition, Registry,
    RegistryError, StateError,
};
use thiserror::Error;

/// Errors while registering vanilla content.
#[derive(Debug, Error)]
pub enum ContentError {
    /// An identifier literal failed validation (a bug in this crate).
    #[error(transparent)]
    Id(#[from] ParseError),
    /// A block property declaration was invalid.
    #[error(transparent)]
    State(#[from] StateError),
    /// The target registry rejected an entry (duplicate or frozen).
    #[error(transparent)]
    Registry(#[from] RegistryError),
}

/// The property sets of the M1 block set, in registration order.
///
/// Order matters: block indices and block-state ids are derived from it, so
/// entries are only ever appended, never reordered (SPEC.md 5.3).
fn block_table() -> Result<Vec<(&'static str, BlockProperties)>, ContentError> {
    // Facings/axes use the modern flattened order; these lists define the
    // canonical state order and must be treated as frozen once saves exist.
    let axis = || PropertyDefinition::enumeration("axis", ["x", "y", "z"]);
    let facing = || PropertyDefinition::enumeration("facing", ["north", "south", "west", "east"]);

    let entries: Vec<(&'static str, Result<BlockProperties, StateError>)> = vec![
        // Air must be index 0 so "empty" is a stable, cheap sentinel.
        ("minecraft:air", Ok(BlockProperties::none())),
        ("minecraft:stone", Ok(BlockProperties::none())),
        ("minecraft:dirt", Ok(BlockProperties::none())),
        (
            "minecraft:grass_block",
            BlockProperties::new(vec![PropertyDefinition::bool("snowy")]),
        ),
        ("minecraft:bedrock", Ok(BlockProperties::none())),
        ("minecraft:sand", Ok(BlockProperties::none())),
        (
            "minecraft:water",
            BlockProperties::new(vec![PropertyDefinition::int("level", 0, 15)]),
        ),
        (
            "minecraft:lava",
            BlockProperties::new(vec![PropertyDefinition::int("level", 0, 15)]),
        ),
        ("minecraft:oak_log", BlockProperties::new(vec![axis()])),
        (
            "minecraft:oak_leaves",
            BlockProperties::new(vec![
                PropertyDefinition::int("distance", 1, 7),
                PropertyDefinition::bool("persistent"),
            ]),
        ),
        ("minecraft:oak_planks", Ok(BlockProperties::none())),
        ("minecraft:cobblestone", Ok(BlockProperties::none())),
        ("minecraft:coal_ore", Ok(BlockProperties::none())),
        ("minecraft:iron_ore", Ok(BlockProperties::none())),
        ("minecraft:glass", Ok(BlockProperties::none())),
        (
            "minecraft:oak_stairs",
            // 4 facings x 2 halves x 5 shapes = 40 states.
            BlockProperties::new(vec![
                facing(),
                PropertyDefinition::enumeration("half", ["bottom", "top"]),
                PropertyDefinition::enumeration(
                    "shape",
                    [
                        "straight",
                        "inner_left",
                        "inner_right",
                        "outer_left",
                        "outer_right",
                    ],
                ),
            ]),
        ),
        (
            "minecraft:stone_slab",
            // 3 states: bottom, top, double.
            BlockProperties::new(vec![PropertyDefinition::enumeration(
                "type",
                ["bottom", "top", "double"],
            )]),
        ),
        ("minecraft:crafting_table", Ok(BlockProperties::none())),
        (
            "minecraft:furnace",
            // 4 facings x lit(false/true) = 8 states.
            BlockProperties::new(vec![facing(), PropertyDefinition::bool("lit")]),
        ),
        (
            "minecraft:torch",
            // 5 attach directions, `up` first (default).
            BlockProperties::new(vec![PropertyDefinition::enumeration(
                "facing",
                ["up", "north", "south", "west", "east"],
            )]),
        ),
        ("minecraft:chest", BlockProperties::new(vec![facing()])),
        (
            "minecraft:wheat",
            BlockProperties::new(vec![PropertyDefinition::int("age", 0, 7)]),
        ),
    ];

    let mut out = Vec::with_capacity(entries.len());
    for (id, properties) in entries {
        out.push((id, properties?));
    }
    Ok(out)
}

/// Number of blocks in the M1 vanilla set.
pub const BLOCK_COUNT: usize = 22;

/// Registers every baseline block into `registry`, returning the number added.
///
/// Entries are registered in the frozen [`block_table`] order, so indices are
/// reproducible. Callers that add mod content afterwards get the next indices.
pub fn register(registry: &mut Registry<BlockProperties>) -> Result<u32, ContentError> {
    let table = block_table()?;
    debug_assert_eq!(table.len(), BLOCK_COUNT);
    let mut added = 0;
    for (id, properties) in table {
        registry.register(Identifier::parse(id)?, properties)?;
        added += 1;
    }
    Ok(added)
}

/// Builds a frozen [`BlockStateModel`] containing only vanilla blocks.
///
/// Convenience for tests, tools, and worldgen; real sessions start from a
/// shared registry that mods also populate.
pub fn block_states() -> Result<BlockStateModel, ContentError> {
    let mut registry = Registry::new("minecraft:block");
    register(&mut registry)?;
    Ok(BlockStateModel::freeze(registry)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mclre_core::{BlockId, BlockStateId, PropertyValue};

    fn states() -> BlockStateModel {
        block_states().expect("vanilla content freezes")
    }

    #[test]
    fn registers_the_full_block_set() {
        let model = states();
        assert_eq!(
            model.block_count(),
            u32::try_from(BLOCK_COUNT).expect("fits")
        );
        let ids = model.registry().ids();
        assert_eq!(ids[0].to_string_form(), "minecraft:air");
        assert_eq!(ids.len(), BLOCK_COUNT);
        // Registration order is the table order.
        assert_eq!(ids[1].to_string_form(), "minecraft:stone");
        assert_eq!(ids[BLOCK_COUNT - 1].to_string_form(), "minecraft:wheat");
    }

    #[test]
    fn per_block_state_counts_match_property_products() {
        let model = states();
        let registry = model.registry();
        let expected = [
            ("minecraft:air", 1),
            ("minecraft:grass_block", 2),
            ("minecraft:water", 16),
            ("minecraft:lava", 16),
            ("minecraft:oak_log", 3),
            ("minecraft:oak_leaves", 14), // 7 distances x 2 persistent
            ("minecraft:oak_stairs", 40),
            ("minecraft:stone_slab", 3),
            ("minecraft:furnace", 8),
            ("minecraft:torch", 5),
            ("minecraft:chest", 4),
            ("minecraft:wheat", 8),
        ];
        for (id, count) in expected {
            let identifier = Identifier::parse(id).expect("valid literal");
            let (properties, index) = registry.get(&identifier).expect("registered");
            assert_eq!(
                properties.state_count(),
                count,
                "wrong state count for {id}"
            );
            let first = model.default_state(BlockId(index)).expect("has states");
            let last = BlockStateId(first.0 + u32::try_from(count).expect("fits") - 1);
            assert_eq!(model.block_of(first), Some(BlockId(index)));
            assert_eq!(model.block_of(last), Some(BlockId(index)));
        }
    }

    #[test]
    fn state_ids_are_contiguous_and_cover_every_block() {
        let model = states();
        let total: u32 = model
            .registry()
            .values()
            .iter()
            .map(|p| u32::try_from(p.state_count()).expect("fits"))
            .sum();
        assert_eq!(model.state_count(), total);
        for state in 0..model.state_count() {
            assert!(
                model.block_of(BlockStateId(state)).is_some(),
                "state {state} has no block"
            );
        }
        assert_eq!(model.block_of(BlockStateId(total)), None);
    }

    #[test]
    fn every_state_round_trips_through_property_values() {
        let model = states();
        for state in 0..model.state_count() {
            let state = BlockStateId(state);
            let block = model.block_of(state).expect("state has a block");
            let values = model.values_of(state).expect("state has values");
            let recomputed = model
                .state_from_values(block, &values)
                .expect("values are valid for their block");
            assert_eq!(recomputed, state, "round trip failed for {state:?}");
        }
    }

    #[test]
    fn default_states_use_first_canonical_values() {
        let model = states();
        let registry = model.registry();
        let oak_log = Identifier::parse("minecraft:oak_log").expect("valid");
        let (_, index) = registry.get(&oak_log).expect("registered");
        let default = model.default_state(BlockId(index)).expect("has states");
        assert_eq!(
            model.values_of(default),
            Some(vec![PropertyValue::Enum("x".into())])
        );

        let furnace = Identifier::parse("minecraft:furnace").expect("valid");
        let (_, index) = registry.get(&furnace).expect("registered");
        let default = model.default_state(BlockId(index)).expect("has states");
        assert_eq!(
            model.values_of(default),
            Some(vec![
                PropertyValue::Enum("north".into()),
                PropertyValue::Bool(false)
            ])
        );
    }

    #[test]
    fn declaring_states_is_idempotent_across_builds() {
        // State ids must be reproducible run to run and platform to platform.
        let first = states();
        let second = states();
        assert_eq!(first.state_count(), second.state_count());
        for state in 0..first.state_count() {
            let state = BlockStateId(state);
            assert_eq!(first.block_of(state), second.block_of(state));
            assert_eq!(first.values_of(state), second.values_of(state));
        }
    }

    #[test]
    fn registering_into_an_existing_registry_appends_after_mod_content() {
        let mut registry = Registry::new("minecraft:block");
        registry
            .register(
                Identifier::parse("testmod:first").expect("valid"),
                BlockProperties::none(),
            )
            .expect("register mod block");
        let added = register(&mut registry).expect("vanilla registers");
        assert_eq!(added as usize, BLOCK_COUNT);
        let model = BlockStateModel::freeze(registry).expect("freeze");
        assert_eq!(model.registry().ids()[0].to_string_form(), "testmod:first");
        assert_eq!(model.registry().ids()[1].to_string_form(), "minecraft:air");
    }
}
