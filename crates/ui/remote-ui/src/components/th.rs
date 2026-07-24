use rudi::Singleton;

use crate::component::{
    ComponentDefinition, ComponentShape, RemoteComponent, bind_component, spec,
};

/// 表格标题单元格。
#[derive(Clone, Debug)]
#[Singleton(name = module_path!(), binds = [bind_component::<Th>])]
pub struct Th;

impl RemoteComponent for Th {
    fn definition(&self) -> ComponentDefinition {
        ComponentDefinition {
            shape: ComponentShape::Leaf,
            spec: spec(
                "th",
                "remote-ui-table-head h-10 px-4 text-left align-middle font-medium text-muted-foreground whitespace-nowrap",
            ),
        }
    }
}
