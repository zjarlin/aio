# Remote UI

`az-remote-ui` 提供不依赖具体 UI 框架的声明式页面模型和流式界面协议。正式页面保存 `PageDefinition`，服务端使用当前 Rudi 组件 catalog 校验并编译为类型化 `UiOp`；浏览器 DOM、GPUI 或其他渲染器只消费操作流。

组件通过 Rudi `Singleton` 注册，provider 的 `module_path!()` 是 canonical ID，DSL 名称由组件模块文件名机械派生。组件同时声明属性、事件和渲染 schema；页面配置直接使用 canonical ID，不维护第二套组件枚举。组件 catalog 对齐 AIO 当前组件库的 Card、Button、Table 与 Badge 视觉语义，但协议内核不依赖 Dioxus。

```rust
use std::sync::Arc;

use az_remote_ui::{ComponentIndex, UiParser};
use rudi::Context;

let mut context = Context::auto_register();
let components = ComponentIndex::from_context(&mut context)?;
let mut parser = UiParser::new(Arc::new(components));
let operations = parser.feed("[card][card-title 设备状态][/card]")?;
parser.finish()?;
# Ok::<(), anyhow::Error>(())
```

紧凑 DSL 只用于增量生成或模型输出；正式持久化模型使用 `PageDefinition`，运行时通过 `PageCompiler` 生成操作流。属性值只允许字面量或只读数据路径，动作只能引用页面显式声明的 operation，不执行任意表达式。

运行测试：

```bash
cargo test -p az-remote-ui
```
