use rudi::Singleton;

use crate::component::{
    ComponentDefinition, ComponentShape, RemoteComponent, bind_component, spec,
};

/// 普通说明段落。
#[derive(Clone, Debug)]
#[Singleton(name = module_path!(), binds = [bind_component::<P>])]
pub struct P;

impl RemoteComponent for P {
    fn definition(&self) -> ComponentDefinition {
        ComponentDefinition {
            shape: ComponentShape::Leaf,
            spec: spec(
                "p",
                "remote-ui-paragraph text-sm text-muted-foreground",
            ),
        }
    }
}
