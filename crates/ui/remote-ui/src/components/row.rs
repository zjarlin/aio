use rudi::Singleton;

use crate::component::{
    ComponentDefinition, ComponentPropertyKind, ComponentShape, RemoteComponent, bind_component,
    choice_property, properties, property, spec,
};

/// 响应式组件行。
#[derive(Clone, Debug)]
#[Singleton(name = module_path!(), binds = [bind_component::<Row>])]
pub struct Row;

impl RemoteComponent for Row {
    fn definition(&self) -> ComponentDefinition {
        ComponentDefinition {
            shape: ComponentShape::Container,
            spec: spec("div", "remote-ui-row grid gap-4"),
        }
    }

    fn properties(&self) -> std::collections::BTreeMap<String, crate::ComponentPropertySpec> {
        properties(&[
            ("v", choice_property(&["metrics", "workbench"])),
            ("aria-label", property(ComponentPropertyKind::Text, false)),
        ])
    }
}
