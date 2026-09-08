# Plugin Manifest

Cargo 工件：`az-plugin-manifest`

本 crate 提供语言无关的 `aio-plugin.toml`、`PageDefinition`、`PluginRequest`、`ComponentResponse` 和运行目标模型。`validation` feature 提供仓库 artifact、子插件依赖图和 Wasm Component ABI 校验；`schema` feature 从同一 Rust 模型生成多语言工具可消费的 JSON Schema。
