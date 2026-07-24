#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

pub mod command_catalog;
pub mod contract;
pub mod plugin;
pub mod routes;
pub mod service;
pub mod state;
pub mod ui;

pub use plugin::SshPlugin;

rudi::enable! {}
