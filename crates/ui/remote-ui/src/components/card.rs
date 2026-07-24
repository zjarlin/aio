use rudi::Singleton;

use crate::component::{
    ComponentDefinition, ComponentShape, RemoteComponent, bind_component, spec,
};

/// 复用现有组件库视觉语义的卡片容器。
#[derive(Clone, Debug)]
#[Singleton(name = module_path!(), binds = [bind_component::<Card>])]
pub struct Card;

impl RemoteComponent for Card {
    fn definition(&self) -> ComponentDefinition {
        ComponentDefinition {
            shape: ComponentShape::Container,
            spec: spec(
                "article",
                "remote-ui-card bg-card text-card-foreground flex flex-col gap-4 rounded-xl border py-6 shadow-sm",
            ),
        }
    }
}
