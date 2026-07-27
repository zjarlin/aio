use rudi::Singleton;

use crate::component::{
    ComponentBehavior, ComponentDefinition, ComponentPropertyKind, ComponentShape,
    RemoteComponent, bind_component, choice_property, event, events, properties, property,
    spec_with_variants,
};

/// 通过 action ID 上报交互的按钮。
#[derive(Clone, Debug)]
#[Singleton(name = module_path!(), binds = [bind_component::<Button>])]
pub struct Button;

impl RemoteComponent for Button {
    fn semantic_names(&self) -> Vec<String> {
        vec!["按钮".to_string(), "操作".to_string()]
    }

    fn definition(&self) -> ComponentDefinition {
        ComponentDefinition {
            shape: ComponentShape::Leaf,
            spec: spec_with_variants(
                "button",
                "remote-ui-button inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-all shrink-0 w-fit h-9 px-4 py-2",
                &[
                    (
                        "primary",
                        "bg-primary text-primary-foreground shadow-xs hover:bg-primary/90",
                    ),
                    (
                        "destructive",
                        "bg-destructive text-white shadow-xs hover:bg-destructive/90",
                    ),
                    (
                        "outline",
                        "border bg-background shadow-xs hover:bg-accent hover:text-accent-foreground",
                    ),
                    ("ghost", "hover:bg-accent hover:text-accent-foreground"),
                ],
                "primary",
                ComponentBehavior::Button,
            ),
        }
    }

    fn properties(&self) -> std::collections::BTreeMap<String, crate::ComponentPropertySpec> {
        properties(&[
            ("tx", property(ComponentPropertyKind::Text, false)),
            ("act", property(ComponentPropertyKind::Action, false)),
            (
                "v",
                choice_property(&["primary", "destructive", "outline", "ghost"]),
            ),
            (
                "disabled",
                property(ComponentPropertyKind::Boolean, false),
            ),
        ])
    }

    fn events(&self) -> std::collections::BTreeMap<String, crate::ComponentEventSpec> {
        events(&[(
            "click",
            event(&[("action", ComponentPropertyKind::Action)]),
        )])
    }
}
