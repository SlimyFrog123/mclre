//! MCLRE core: identifiers, registries, tags, math types, reference-exact RNG,
//! NBT, errors, tick types (SPEC.md 5; PLAN.md M1).
//!
//! Implemented so far: [`identifier`], [`random`], [`mth`] (including [`Vec3`]
//! and [`AABB`]), [`registry`], [`block_state`], and [`nbt`].

pub mod block_state;
pub mod identifier;
pub mod mth;
pub mod nbt;
pub mod random;
pub mod registry;

pub use block_state::{
    BlockId, BlockProperties, BlockStateId, BlockStateModel, PropertyDefinition, PropertyValue,
    StateError,
};
pub use identifier::{DEFAULT_NAMESPACE, Identifier, InternedStr, ParseError};
pub use mth::{AABB, Mth, Vec3};
pub use random::{CosmeticRandom, GameRandom};
pub use registry::{FrozenRegistry, Registry, RegistryError, TagEntry};
