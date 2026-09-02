# Plugin Core

Cargo 工件：`az-plugin-core`

本 crate 提供 Bevy 风格的 `App<T>`、`Plugin<T>`、`PluginGroup<T>`，以及统一 HTTP 边界、共享 PostgreSQL 句柄、内置 Toasty 模型、动态模型和 JSONB 记录。

`Plugin<T>` 继承 `Any`。Dill 使用 `AllOf<dyn Plugin<T>>` 聚合实现，`PluginGroupBuilder` 只保存 `TypeId` 以恢复显式顺序，`App` 也只按 `TypeId` 拒绝重复插件。`Plugin::comment()` 可以提供诊断描述，但不参与注册、查找或持久化。

```bash
cargo test -p az-plugin-core
```
