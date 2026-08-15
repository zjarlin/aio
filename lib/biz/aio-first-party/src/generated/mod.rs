pub(crate) mod algorithms;
pub(crate) mod assets;
pub(crate) mod config;
pub(crate) mod drive;
pub(crate) mod gateway;
pub(crate) mod iot;
pub(crate) mod linux;
pub(crate) mod software;
pub(crate) mod ssh;

use dill::CatalogBuilder;

pub fn register(builder: &mut CatalogBuilder) {
    config::register(builder);
    assets::register(builder);
    drive::register(builder);
    software::register(builder);
    algorithms::register(builder);
    iot::register(builder);
    ssh::register(builder);
    gateway::register(builder);
    linux::register(builder);
}
