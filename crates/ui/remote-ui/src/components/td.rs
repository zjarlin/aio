use rudi::Singleton;

use crate::component::{
    ComponentDefinition, ComponentShape, RemoteComponent, bind_component, spec,
};

/// 可直接放文本或嵌套状态组件的表格单元格。
#[derive(Clone, Debug)]
#[Singleton(name = module_path!(), binds = [bind_component::<Td>])]
pub struct Td;

impl RemoteComponent for Td {
    fn definition(&self) -> ComponentDefinition {
        ComponentDefinition {
            shape: ComponentShape::Dual,
            spec: spec("td", "remote-ui-table-cell p-4 align-middle"),
        }
    }
}
