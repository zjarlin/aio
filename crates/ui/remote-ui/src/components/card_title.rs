use rudi::Singleton;

use crate::component::{
    ComponentDefinition, ComponentShape, RemoteComponent, bind_component, spec,
};

/// 卡片主标题。
#[derive(Clone, Debug)]
#[Singleton(name = module_path!(), binds = [bind_component::<CardTitle>])]
pub struct CardTitle;

impl RemoteComponent for CardTitle {
    fn definition(&self) -> ComponentDefinition {
        ComponentDefinition {
            shape: ComponentShape::Leaf,
            spec: spec(
                "h2",
                "remote-ui-card-title leading-none font-semibold",
            ),
        }
    }
}
