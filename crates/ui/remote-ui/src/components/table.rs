use rudi::Singleton;

use crate::component::{
    ComponentBehavior, ComponentDefinition, ComponentShape, RemoteComponent, bind_component,
    spec_with_behavior,
};

/// 带滚动边界的数据表格。
#[derive(Clone, Debug)]
#[Singleton(name = module_path!(), binds = [bind_component::<Table>])]
pub struct Table;

impl RemoteComponent for Table {
    fn definition(&self) -> ComponentDefinition {
        ComponentDefinition {
            shape: ComponentShape::Container,
            spec: spec_with_behavior(
                "table",
                "remote-ui-table w-full text-sm caption-bottom",
                ComponentBehavior::Table,
            ),
        }
    }
}
