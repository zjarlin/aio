use rudi::Singleton;

use crate::component::{
    ComponentDefinition, ComponentShape, RemoteComponent, bind_component, spec,
};

/// 卡片辅助说明。
#[derive(Clone, Debug)]
#[Singleton(name = module_path!(), binds = [bind_component::<CardDescription>])]
pub struct CardDescription;

impl RemoteComponent for CardDescription {
    fn definition(&self) -> ComponentDefinition {
        ComponentDefinition {
            shape: ComponentShape::Leaf,
            spec: spec(
                "p",
                "remote-ui-card-description text-muted-foreground text-sm",
            ),
        }
    }
}
