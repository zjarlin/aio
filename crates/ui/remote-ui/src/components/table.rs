use rudi::Singleton;

use crate::component::{
    ComponentBehavior, ComponentDefinition, ComponentPropertyKind, ComponentShape,
    RemoteComponent, bind_component, properties, property, spec_with_behavior,
};

/// 带滚动边界的数据表格。
#[derive(Clone, Debug)]
#[Singleton(name = module_path!(), binds = [bind_component::<Table>])]
pub struct Table;

impl RemoteComponent for Table {
    fn semantic_names(&self) -> Vec<String> {
        vec!["表格".to_string(), "列表".to_string()]
    }

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

    fn properties(&self) -> std::collections::BTreeMap<String, crate::ComponentPropertySpec> {
        properties(&[
            ("source", property(ComponentPropertyKind::Text, false)),
            ("columns", property(ComponentPropertyKind::Text, false)),
        ])
    }
}
