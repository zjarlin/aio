use rudi::Singleton;

use crate::component::{
    ComponentBehavior, ComponentDefinition, ComponentPropertyKind, ComponentShape,
    RemoteComponent, bind_component, choice_property, event, events, properties, property,
    spec_with_behavior,
};

/// 带标签和变更事件的输入框。
#[derive(Clone, Debug)]
#[Singleton(name = module_path!(), binds = [bind_component::<Input>])]
pub struct Input;

impl RemoteComponent for Input {
    fn semantic_names(&self) -> Vec<String> {
        vec!["输入框".to_string(), "表单字段".to_string()]
    }

    fn definition(&self) -> ComponentDefinition {
        ComponentDefinition {
            shape: ComponentShape::Leaf,
            spec: spec_with_behavior(
                "input",
                "remote-ui-input aio-input",
                ComponentBehavior::Input,
            ),
        }
    }

    fn properties(&self) -> std::collections::BTreeMap<String, crate::ComponentPropertySpec> {
        properties(&[
            ("label", property(ComponentPropertyKind::Text, false)),
            ("ph", property(ComponentPropertyKind::Text, false)),
            ("name", property(ComponentPropertyKind::Text, false)),
            (
                "type",
                choice_property(&["text", "email", "number", "password", "search"]),
            ),
            (
                "disabled",
                property(ComponentPropertyKind::Boolean, false),
            ),
            (
                "required",
                property(ComponentPropertyKind::Boolean, false),
            ),
            (
                "readonly",
                property(ComponentPropertyKind::Boolean, false),
            ),
        ])
    }

    fn events(&self) -> std::collections::BTreeMap<String, crate::ComponentEventSpec> {
        events(&[(
            "change",
            event(&[
                ("id", ComponentPropertyKind::Text),
                ("value", ComponentPropertyKind::Text),
            ]),
        )])
    }
}
