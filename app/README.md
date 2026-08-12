# AIO App

Cargo 工件：`az-aio-app`

本包解决 AIO 的应用组装与启动问题：在一个 Dioxus Cargo package 中提供 WASM 入口和 Axum 入口，装配插件 Provider、执行应用 starter、启动 Studio，并发布静态 Web 产物。

`src/main.rs` 只选择浏览器或服务端入口；`src/server.rs` 只建立 Rudi、Tokio listener 并统一执行 starter；具体装配全部归属 `src/application_starters/<feature>.rs`。领域业务仍必须归属 `plugins/<domain>`。

服务端能力统一实现 `az-plugin-core` 的 `Plugin<ApplicationStartup>`，并通过同一 crate 的 `discover_plugins` 与 `install_plugins` 完成发现和安装。通用插件协议拥有身份、默认启用语义、依赖、顺序与异步安装。starter 使用 Rudi singleton 注册，Rudi 按共同的 `DynPlugin<ApplicationStartup>` 类型收集所有实现。

AIO 当前注册 13 个 starter，覆盖数据库、原生插件、Studio 运行时、约定接口、AI Agent、静态 Web、边缘网关 seed 和 API 中间件。`ApplicationStartup` 只保存各阶段产物并在依赖未执行时立即报错；它不主动构造任何功能。

Rust 使用静态链接，外部 starter crate 仍需由最终应用调用一次它导出的 `enable()`，确保注册项进入二进制；完成链接后，宿主不再手工构造、收集或排序 starter。配置项或配置 SPI 作为 Rudi singleton 注册，并由 starter 构造器直接注入；`ApplicationStartup` 只承载安装阶段的可变状态。

```bash
cd app && dx build --platform web
cargo run -p az-aio-app
```
