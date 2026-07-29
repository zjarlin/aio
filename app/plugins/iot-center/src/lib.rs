#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

pub mod contract;
pub mod online_status;
pub mod plugin;
pub mod routes;
pub mod service;
pub mod state;
pub mod telemetry;

pub use plugin::IotCenterPlugin;

rudi::enable! {}
