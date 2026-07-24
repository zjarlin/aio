use rudi::Singleton;

use crate::component::{
    ComponentDefinition, ComponentShape, RemoteComponent, bind_component, spec,
};

/// 表格标题行组。
#[derive(Clone, Debug)]
#[Singleton(name = module_path!(), binds = [bind_component::<Thead>])]
pub struct Thead;

impl RemoteComponent for Thead {
    fn definition(&self) -> ComponentDefinition {
        ComponentDefinition {
            shape: ComponentShape::Container,
            spec: spec("thead", "remote-ui-table-header"),
        }
    }
}
