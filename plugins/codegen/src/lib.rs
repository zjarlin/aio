#![forbid(unsafe_code)]

//! AIO 的 nature-compiler 母语编译工作台。

pub mod contract;
pub mod dictionary_source;
pub mod gate;
pub mod inference_agent;
pub mod model;
pub mod plugin;
pub mod routes;
pub mod service;
pub mod store;
pub mod ui;

pub use plugin::CodegenPlugin;

rudi::enable! {}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use nature_compiler::CapabilityProvider;
    use rudi::Context;

    #[test]
    fn rudi_collects_nature_capabilities_and_models() {
        super::enable();
        let mut context = Context::auto_register();
        let capabilities = context.resolve_by_type::<Arc<dyn CapabilityProvider>>();
        let models =
            context.resolve_by_type::<az_aio_platform::core::db::ToastyModelContribution>();

        assert!(
            capabilities
                .iter()
                .any(|provider| provider.descriptor().native_name == "模拟采集")
        );
        assert!(!models.is_empty());
    }
}
