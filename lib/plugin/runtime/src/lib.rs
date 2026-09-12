#![forbid(unsafe_code)]

#[cfg(test)]
mod cancellation_tests;
mod cryptography;
mod database;
mod database_parameters;
mod database_query;
mod engine;
mod imports;
mod metadata;
mod persistence;
mod provision;
mod registry;
mod release;
mod services;
mod state;
mod storage;
#[cfg(test)]
mod transaction_tests;

pub use cryptography::Keyring;
pub use database::ScopedDatabase;
pub use engine::{CompiledComponent, ComponentEngine, ComponentInstance};
pub use provision::DatabaseProvisioner;
pub use registry::{PersistentComponentSlot, StoredRelease};
pub use release::{ComponentSlot, ReleaseSnapshot};
pub use services::HostServices;
pub use state::InvocationResources;
pub use storage::ObjectStore;

pub const FRONTEND_HOST: &str = include_str!("../../../../sdk/web/host.mjs");

pub mod bindings {
    wasmtime::component::bindgen!({
        path: "../contract/wit",
        world: "plugin",
        imports: { default: async | trappable },
        exports: { default: async },
    });
}
