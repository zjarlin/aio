use rudi::Singleton;

use crate::component::{
    ComponentDefinition, ComponentShape, RemoteComponent, bind_component, spec,
};

/// 保留流式正文空格的代码区域。
#[derive(Clone, Debug)]
#[Singleton(name = module_path!(), binds = [bind_component::<Code>])]
pub struct Code;

impl RemoteComponent for Code {
    fn definition(&self) -> ComponentDefinition {
        ComponentDefinition {
            shape: ComponentShape::Container,
            spec: spec(
                "pre",
                "remote-ui-code overflow-auto rounded-md bg-muted p-4 text-sm font-mono",
            ),
        }
    }
}
