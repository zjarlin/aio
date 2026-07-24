use rudi::Singleton;

use crate::component::{
    ComponentDefinition, ComponentShape, RemoteComponent, bind_component, spec,
};

/// 卡片主体内容区域。
#[derive(Clone, Debug)]
#[Singleton(name = module_path!(), binds = [bind_component::<CardContent>])]
pub struct CardContent;

impl RemoteComponent for CardContent {
    fn definition(&self) -> ComponentDefinition {
        ComponentDefinition {
            shape: ComponentShape::Container,
            spec: spec("div", "remote-ui-card-content px-6"),
        }
    }
}
