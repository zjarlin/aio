# AIO

AIO 是数据库原生的 Dioxus 低代码应用。PostgreSQL 保存正式 `ProgramDefinition`，Studio 根据定义生成页面、领域 Service 契约与 Controller，再编译为 `ProgramImage` 发布。生成文件只是类型检查和实现扩展点，不反向成为业务定义真源。

```text
拖拽 / AI Vibe
    -> PostgreSQL Draft
    -> 类型、Effect、权限、组件门禁
    -> ApplicationImage cache
    -> ArcSwap 原子发布
    -> Dioxus 动态渲染与 Graph VM
```

## 目录

Rust 包按宿主、生成应用和公共运行时分为三个根目录：

```text
app/
  src/lib.rs              生成应用可复用的服务端入口
  src/main.rs             Studio 宿主目标选择入口
  src/admin_shell.rs      Web/Desktop 共用的发布应用入口
  src/application_startup.rs  服务端 starter 宿主状态
  src/application_starters/   Bevy 风格服务端插件组
  plugins/studio/         低代码定义、编译与运行时
  migrations/             PostgreSQL 正式协议迁移
  assets/                 应用静态资源
generated/
  apps/<application-id>/  从不可变 Revision 生成、可整体删除重建的 Web、Desktop、Server 工程
lib/
  biz/<application-id>/   生成 Service 契约、Controller 与人工 Service 实现槽
  plugin/core/            通用 Plugin<T> 与插件公共契约
```

`app/plugins/studio` 拥有 AIO 的 ProgramGraph、编辑器、编译器、发布器、Graph VM、发布应用适配和 REST/SSE。通用应用壳与基础组件统一来自 submodule [`dioxus-admin-workbench`](https://github.com/zjarlin/dioxus-admin-workbench) 中的 `az-dioxus-admin-shell` 和 `az-ui-components`，AIO 不保存壳层、组件或 CSS 副本。

`lib/plugin/core` 的 Cargo 工件名是 `az-plugin-core`，提供 Bevy 风格的 `App<T>`、`Plugin<T>`、`PluginGroup<T>`，以及 HTTP 边界、共享 PostgreSQL 句柄、动态模型和 JSONB 记录。Dill 聚合具体类型，插件唯一性只按 `TypeId` 校验。

## 依赖方向

```text
az-aio-app
  -> az-studio
  -> az-biz-<application-id>
  -> az-plugin-core

az-studio -> az-plugin-core
```

模型、页面、菜单与 REST 契约进入 PostgreSQL `ProgramDefinition`。`ApplicationCompiler` 只在 `generated/apps/<application-id>` 生成普通页面函数与发布应用壳；`BusinessModuleManager` 在 `lib/biz/<application-id>` 生成 Service trait 和 Controller，并只在缺失时创建 `src/generated/<feature>/service_impl.rs` 人工实现槽。Controller 由 Dill 按 `TypeId` 聚合，endpoint `SymbolId` 只用于连接业务元数据。

`lib/biz/<application-id>` 是业务 Service 与 Controller 库，`generated/apps/<application-id>` 是引用该业务库的可执行发布装配，两者不是重复实现。Service 骨架由内容哈希跟踪：未修改的骨架随接口元数据删除，人工修改后立即脱离生成器所有权。Studio 的“应用”视图通过 `ApplicationCompiler` 预览并原子替换生成目录；普通 GraphPatch、Vibe 和回滚都会同步清理失效页面源码。生成目录只包含可编译源码、Cargo/Dioxus 配置、Dockerfile 和部署说明；正式元数据继续以 PostgreSQL 为唯一来源。Web 与 Desktop 共享 `PublishedApplication`；Web 使用浏览器同源 HTTP，Desktop 读取 `AIO_API_BASE_URL`，Server feature 通过 `run_server_with` 注入对应业务模块。

## 开发

```bash
cargo test --workspace
cd app && dx build --platform web
./scripts/preview.zsh

# 已生成应用
dx build -p az-app-aio-first-party --platform web --release --no-default-features --features web --debug-symbols false
cargo run -p az-app-aio-first-party --no-default-features --features desktop
cargo run -p az-app-aio-first-party --no-default-features --features server
```

应用启动配置以明文保存在仓库根目录 `.env`。`AZ_AIO_DATABASE_URL` 配置正式 PostgreSQL，`AZ_AIO_DATABASE_MIGRATIONS_ENABLED` 控制 SQLx 迁移并默认启用，`AZ_AIO_WEB_PORT` 配置 Web 监听端口（未配置时默认为 `8080`），`AZ_AIO_WEB_DIST` 可覆盖 Web 产物目录。当前仓库 `.env` 将 Web 端口配置为 `3000`。`scripts/preview.zsh` 会先验证直连数据库；直连不可用时，经本机 SOCKS relay 回退到同一数据库，且不会修改 `.env`。启动后访问 `http://127.0.0.1:3000/`。

服务端 starter 统一实现 `Plugin<ApplicationStartup>`。`AioPlugins` 像 Bevy 的 `PluginGroup` 一样显式声明构建顺序，`App` 负责按 `TypeId` 做唯一性校验和逐个构建；Dill 负责 Starter 及其全部服务依赖的构造和 `AllOf` 聚合，`ApplicationStartup` 只保存最终顺序合并的 Router。

当前数据库迁移、共享数据库、Capability、Controller、ProgramRuntime、页面与业务模块同步、Studio HTTP 和静态 Web 均由 `AioPlugins` 构建。`server.rs` 只保留配置、Dill Catalog、Tokio listener、`App` 构建和 `axum::serve`。
