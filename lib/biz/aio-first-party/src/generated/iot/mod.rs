mod controller;
pub(crate) mod model;
pub(crate) mod service;
mod service_impl;
pub(crate) mod util;

use dill::CatalogBuilder;

pub(crate) fn register(builder: &mut CatalogBuilder) {
    builder.add::<service_impl::IotServiceImpl>();
    controller::register(builder);
}
