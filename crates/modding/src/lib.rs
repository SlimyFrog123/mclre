//! Host-side modding: mod manifest, dependency resolution, WASM sandbox host, event bus.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod dependency;
mod event_bus;
mod manifest;
mod mod_loader;
mod mod_registry;
mod permissions;
mod wasm_host;

pub use dependency::{DependencyError, DependencyResolver};
pub use event_bus::{EventBus, EventError, EventSubscription};
pub use manifest::{ModManifest, ModManifestError};
pub use mod_loader::{ModLoader, ModLoaderError};
pub use mod_registry::{ModRegistry, ModRegistryError};
pub use permissions::{PermissionError, PermissionSet};
pub use wasm_host::{WasmError, WasmHost, WasmHostConfig};

/// Re-exports for convenience
pub mod prelude {
    pub use crate::{
        DependencyError, DependencyResolver, EventBus, EventError, EventSubscription, ModLoader,
        ModLoaderError, ModManifest, ModManifestError, ModRegistry, ModRegistryError,
        PermissionError, PermissionSet, WasmError, WasmHost, WasmHostConfig,
    };
}

/// Version of the modding system
pub const MODDING_VERSION: &str = "1.0.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modding_version() {
        assert_eq!(MODDING_VERSION, "1.0.0");
    }
}
