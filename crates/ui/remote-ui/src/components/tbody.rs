use rudi::Singleton;

use crate::component::{
    ComponentDefinition, ComponentShape, RemoteComponent, bind_component, spec,
};

/// 表格数据行组。
#[derive(Clone, Debug)]
#[Singleton(name = module_path!(), binds = [bind_component::<Tbody>])]
pub struct Tbody;

impl RemoteComponent for Tbody {
    fn definition(&self) -> ComponentDefinition {
        ComponentDefinition {
            shape: ComponentShape::Container,
            spec: spec("tbody", "remote-ui-table-body"),
        }
    }
}
