use rudi::Singleton;

use crate::component::{
    ComponentDefinition, ComponentShape, RemoteComponent, bind_component, spec,
};

/// 页面二级标题。
#[derive(Clone, Debug)]
#[Singleton(name = module_path!(), binds = [bind_component::<H2>])]
pub struct H2;

impl RemoteComponent for H2 {
    fn definition(&self) -> ComponentDefinition {
        ComponentDefinition {
            shape: ComponentShape::Leaf,
            spec: spec("h2", "remote-ui-subheading text-base font-semibold"),
        }
    }
}
