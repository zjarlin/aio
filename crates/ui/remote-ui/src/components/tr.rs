use rudi::Singleton;

use crate::component::{
    ComponentDefinition, ComponentShape, RemoteComponent, bind_component, spec,
};

/// 表格行。
#[derive(Clone, Debug)]
#[Singleton(name = module_path!(), binds = [bind_component::<Tr>])]
pub struct Tr;

impl RemoteComponent for Tr {
    fn definition(&self) -> ComponentDefinition {
        ComponentDefinition {
            shape: ComponentShape::Container,
            spec: spec(
                "tr",
                "remote-ui-table-row border-b transition-colors hover:bg-muted/50",
            ),
        }
    }
}
