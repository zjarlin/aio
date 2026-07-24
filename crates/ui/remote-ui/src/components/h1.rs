use rudi::Singleton;

use crate::component::{
    ComponentDefinition, ComponentShape, RemoteComponent, bind_component, spec,
};

/// 页面一级标题。
#[derive(Clone, Debug)]
#[Singleton(name = module_path!(), binds = [bind_component::<H1>])]
pub struct H1;

impl RemoteComponent for H1 {
    fn definition(&self) -> ComponentDefinition {
        ComponentDefinition {
            shape: ComponentShape::Leaf,
            spec: spec("h1", "remote-ui-heading text-lg font-semibold"),
        }
    }
}
