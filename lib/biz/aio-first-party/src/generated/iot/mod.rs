pub(crate) mod contract;
mod controller;

use dill::CatalogBuilder;

pub(crate) fn register(builder: &mut CatalogBuilder) {
    builder.add::<crate::service::iot::IotServiceImpl>();
    controller::register(builder);
}
