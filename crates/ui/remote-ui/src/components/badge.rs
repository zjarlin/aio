use rudi::Singleton;

use crate::component::{
    ComponentBehavior, ComponentDefinition, ComponentPropertyKind, ComponentShape,
    RemoteComponent, bind_component, choice_property, properties, property, spec_with_variants,
};

/// 表达状态和分类的徽标。
#[derive(Clone, Debug)]
#[Singleton(name = module_path!(), binds = [bind_component::<Badge>])]
pub struct Badge;

impl RemoteComponent for Badge {
    fn definition(&self) -> ComponentDefinition {
        ComponentDefinition {
            shape: ComponentShape::Leaf,
            spec: spec_with_variants(
                "span",
                "remote-ui-badge inline-flex items-center font-semibold rounded-md border transition-colors w-fit px-2 py-1 text-xs",
                &[
                    ("default", "bg-primary text-primary-foreground"),
                    ("outline", "text-foreground"),
                    ("success", "remote-ui-badge-success"),
                    ("warning", "remote-ui-badge-warning"),
                    ("danger", "bg-destructive text-destructive-foreground"),
                ],
                "default",
                ComponentBehavior::Generic,
            ),
        }
    }

    fn properties(&self) -> std::collections::BTreeMap<String, crate::ComponentPropertySpec> {
        properties(&[
            ("tx", property(ComponentPropertyKind::Text, false)),
            (
                "v",
                choice_property(&["default", "outline", "success", "warning", "danger"]),
            ),
        ])
    }
}
