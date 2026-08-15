# AIO App

Cargo 工件：`az-aio-app`

本包同时是 Studio 宿主和生成应用的服务端运行库。`src/main.rs` 只选择目标，`src/lib.rs` 导出 Web/Desktop 启动和 `run_server_with`，`src/server.rs` 建立 Dill Catalog、Tokio listener 与 `App<ApplicationStartup>`。

服务端能力统一实现 `az-plugin-core::Plugin<ApplicationStartup>`。`AioPlugins` 显式声明 Bevy 风格的构建顺序；Dill 直接向每个 Starter 注入配置、Provider、数据库与运行时资源，`ApplicationStartup` 只保留 Router。唯一性只按 `TypeId` 校验，不声明字符串 `key` 或 `name`。

Studio 宿主不保存业务页面函数。页面函数只随独立应用生成到 `generated/apps/<application-id>/src/pages`。业务接口由 Studio 同步到 `lib/biz/<application-id>`：`src/generated` 保存契约和 Controller，`src/service` 保存首次创建后不覆盖的人工实现。

生成应用位于 `generated/apps/<application-id>`，每次生成按当前元数据原子替换，旧页面文件不会残留。Web 与 Desktop 共享 `studio::PublishedApplication`，Server 通过 `az_aio_app::run_server_with(business::register)` 注入同名业务模块。

```bash
cd app && dx build --platform web
cargo run -p az-aio-app
```
