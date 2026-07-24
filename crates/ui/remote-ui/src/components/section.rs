use rudi::Singleton;

use crate::component::{
    ComponentDefinition, ComponentShape, RemoteComponent, bind_component, spec,
};

/// 页面内的顶层内容分区。
#[derive(Clone, Debug)]
#[Singleton(name = module_path!(), binds = [bind_component::<Section>])]
pub struct Section;

impl RemoteComponent for Section {
    fn definition(&self) -> ComponentDefinition {
        ComponentDefinition {
            shape: ComponentShape::Container,
            spec: spec("section", "remote-ui-section space-y-6"),
        }
    }
}
