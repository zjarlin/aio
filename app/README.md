# AIO App

Cargo 工件：`az-aio-app`

本包解决 AIO 的应用组装与启动问题：在一个 Dioxus Cargo package 中提供 WASM 入口和 Axum 入口，装配插件 Provider、执行数据库迁移、建立共享数据库、启动 Studio，并发布静态 Web 产物。

`src/main.rs` 只选择浏览器或服务端入口；配置、迁移、插件聚合和服务启动分别放在职责明确的文件中。业务逻辑不得写进应用入口，必须归属 `plugins/<domain>`。

```bash
cd app && dx build --platform web
cargo run -p az-aio-app
```
