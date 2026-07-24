use rudi::Singleton;

use crate::component::{
    ComponentBehavior, ComponentDefinition, ComponentPropertyKind, ComponentShape,
    RemoteComponent, bind_component, choice_property, properties, property, spec_with_behavior,
};

/// 支持 `[upd]` 增量更新的进度组件。
#[derive(Clone, Debug)]
#[Singleton(name = module_path!(), binds = [bind_component::<Progress>])]
pub struct Progress;

impl RemoteComponent for Progress {
    fn definition(&self) -> ComponentDefinition {
        ComponentDefinition {
            shape: ComponentShape::Leaf,
            spec: spec_with_behavior(
                "div",
                "remote-ui-progress",
                ComponentBehavior::Progress,
            ),
        }
    }

    fn properties(&self) -> std::collections::BTreeMap<String, crate::ComponentPropertySpec> {
        properties(&[
            ("label", property(ComponentPropertyKind::Text, false)),
            ("v", property(ComponentPropertyKind::Number, true)),
            (
                "status",
                choice_property(&["idle", "running", "success", "error"]),
            ),
        ])
    }
}
