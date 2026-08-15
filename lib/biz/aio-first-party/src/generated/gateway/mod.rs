pub(crate) mod contract;
mod controller;

use dill::CatalogBuilder;

pub(crate) fn register(builder: &mut CatalogBuilder) {
    builder.add::<crate::service::gateway::GatewayServiceImpl>();
    controller::register(builder);
}
