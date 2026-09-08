# 插件 JSON Schema

本目录是 `aio-plugin.toml`、`PageDefinition`、Wasm Component 请求/响应和页面动作结果的机器可读结构契约。Kotlin、TypeScript 和其他语言可以据此生成类型、提供 IDE 补全或在 CI 中校验 JSON。

Schema 只描述可序列化结构；字段长度、动作状态 64 KiB 上限、子插件依赖环、能力声明和 Wasm ABI 等语义约束继续由 `aio plugin validate` 统一执行。

在协议模型变化后重新生成：

```bash
cargo run --bin aio -- plugin schema docs/plugin/schema
git diff --exit-code -- docs/plugin/schema
```
