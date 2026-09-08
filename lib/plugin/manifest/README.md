# Plugin Manifest

Cargo 工件：`az-plugin-manifest`

本 crate 提供语言无关的 `aio-plugin.toml`、`PageDefinition` 和运行目标模型。`validation` feature 额外提供仓库 artifact、子插件依赖图和 Wasm Component ABI 校验，供 AIO CLI 与运行时宿主复用。
