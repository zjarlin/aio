#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

mod compiler;
mod component;
mod component_index;
mod components;
mod contract;
mod definition;
mod parser;

pub(crate) use az_aio_nature_generated::enums::{ComponentPropertyKind, ComponentShape};

pub use compiler::PageCompiler;
pub use component::{
    ComponentCatalogEntry, ComponentDefinition, ComponentEventSpec, ComponentPropertySpec,
    ComponentSpec, DynRemoteComponent, RemoteComponent,
};
pub use component_index::{ComponentIndex, IndexedComponent};
pub use contract::{UiNode, UiOp};
pub use definition::{
    ActionDefinition, ComponentNode, DataSourceDefinition, PAGE_SCHEMA_VERSION, PageDefinition,
    PropertyValue,
};
pub use parser::UiParser;
