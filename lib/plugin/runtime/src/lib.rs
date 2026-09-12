#![forbid(unsafe_code)]

#[cfg(test)]
mod cancellation_tests;
mod database;
mod database_parameters;
mod database_query;
mod engine;
mod imports;
mod metadata;
mod provision;
mod release;
mod services;
mod state;
mod storage;
#[cfg(test)]
mod transaction_tests;

pub use database::ScopedDatabase;
pub use engine::{CompiledComponent, ComponentEngine, ComponentInstance};
pub use provision::DatabaseProvisioner;
pub use release::{ComponentSlot, ReleaseSnapshot};
pub use services::HostServices;
pub use state::InvocationResources;
pub use storage::ObjectStore;

pub mod bindings {
    wasmtime::component::bindgen!({
        path: "../contract/wit",
        world: "plugin",
        imports: { default: async | trappable },
        exports: { default: async },
    });
}
