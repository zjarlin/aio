use rudi::Singleton;

use crate::component::{
    ComponentDefinition, ComponentShape, RemoteComponent, bind_component, spec,
};

/// 卡片标题区域。
#[derive(Clone, Debug)]
#[Singleton(name = module_path!(), binds = [bind_component::<CardHeader>])]
pub struct CardHeader;

impl RemoteComponent for CardHeader {
    fn definition(&self) -> ComponentDefinition {
        ComponentDefinition {
            shape: ComponentShape::Container,
            spec: spec(
                "header",
                "remote-ui-card-header grid auto-rows-min grid-rows-[auto_auto] items-start gap-1.5 px-6",
            ),
        }
    }
}
