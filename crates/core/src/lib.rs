//! MCLRE core: identifiers, registries, tags, math types, reference-exact RNG,
//! NBT, errors, tick types (SPEC.md 5; PLAN.md M1).
//!
//! Currently implemented: [`identifier`]. Registries, tags, block states,
//! `GameRandom`/`CosmeticRandom`, `Mth`, and NBT land in subsequent M1 slices.

pub mod identifier;

pub use identifier::{DEFAULT_NAMESPACE, Identifier, InternedStr, ParseError};
