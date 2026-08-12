# AIO

AIO 是数据库原生的 Dioxus 零代码应用。PostgreSQL 保存正式 `ProgramDefinition`，业务页面和交互由 Studio 编辑、编译为 `ApplicationImage` 缓存并原子发布，不生成业务 Rust 源码，也不依赖业务文件系统或服务重启。

```text
拖拽 / AI Vibe
    -> PostgreSQL Draft
    -> 类型、Effect、权限、组件门禁
    -> ApplicationImage cache
    -> ArcSwap 原子发布
    -> Dioxus 动态渲染与 Graph VM
```

## 目录

Rust 包只放在两个根目录：

```text
app/
  src/main.rs             单一全栈启动入口
  src/admin_shell/        AIO 的 AdminProvider 与消费方页面扩展
  src/application_startup.rs  服务端 starter 宿主状态
  src/application_starters/   服务端中间件与功能 starter
  plugins/                一个领域功能一个插件
  migrations/             PostgreSQL 正式协议迁移
  assets/                 应用静态资源
crates/
  plugin/core/            通用 Plugin<T> 与插件公共契约
```

`app/plugins/studio` 拥有 AIO 的 ProgramGraph、编辑器、编译器、发布器、Graph VM 和 REST/SSE，不再拥有应用壳层或基础组件。场景和菜单创建、约定文件选择、页面扩展编译与渲染统一由固定 Git 提交的 [`dioxus-admin-workbench`](https://github.com/zjarlin/dioxus-admin-workbench) crates 提供；AIO 只注册一个 `AdminProvider`，把 PostgreSQL Draft、GraphPatch 和运行时记录 API 接入该壳层。

`crates/plugin/core` 的 Cargo 工件名是 `az-plugin-core`，统一提供通用 `Plugin<T>`、默认启用语义、依赖排序、Rudi 发现与安装、插件 Provider/Contribution、HTTP 边界、共享 PostgreSQL 句柄、动态模型和 JSONB 记录，不实现业务功能。

## 依赖方向

```text
az-aio-app
  -> az-dioxus-admin-shell
  -> az-dioxus-admin-extension-crud
  -> az-admin-shell-core
  -> az-studio
  -> az-asset-hub / az-iot-center / ...
  -> az-plugin-core

各功能插件 -> az-plugin-core
扩展 ProgramGraph Capability 的插件 -> az-studio
```

页面扩展在 `PageDefinition::Extension` 中只保存 Rust 限定扩展类型、配置协议版本和 JSON 配置。默认增删改查由 `az-dioxus-admin-extension-crud` 提供；AIO Studio、树表等应用专属内容由 `app/src/admin_shell` 注册，壳层不包含 AIO 分支。

插件之间不得通过应用入口反向依赖。只有确实被多个插件共同使用的契约或基础能力才进入 `az-plugin-core`。

## 开发

```bash
cargo test --workspace
cd app && dx build --platform web
./scripts/preview.zsh
```

应用启动配置以明文保存在仓库根目录 `.env`。`AZ_AIO_DATABASE_URL` 配置正式 PostgreSQL，`AZ_AIO_DATABASE_MIGRATIONS_ENABLED` 控制 SQLx 迁移并默认启用，`AZ_AIO_WEB_PORT` 默认是 `8080`；`AZ_AIO_CONFIG_DIR`、`AZ_AIO_DATA_DIR` 和 `AZ_AIO_WEB_DIST` 可覆盖插件配置目录、数据目录和 Web 产物目录。`scripts/preview.zsh` 会先验证直连数据库；直连不可用时，经本机 SOCKS relay 回退到同一数据库，且不会修改 `.env`。启动后访问 `http://127.0.0.1:8080/`。

服务端 starter 统一实现 `Plugin<ApplicationStartup>`，并使用具体 Rust 类型的全限定名作为唯一标识。`enabled()` 默认返回 `true`，只有需要按宿主配置关闭的实现才重写。Rudi 以 `DynPlugin<ApplicationStartup>` 统一收集所有实现；注册名必须与插件类型标识一致。启用过滤发生在依赖解析前，随后复用通用插件拓扑排序，并以 `order` 和全限定名保持确定顺序。

当前数据库迁移、共享数据库、边缘网关内置数据、原生插件发现、Capability、约定 Provider、ProgramRuntime、约定路由、ProgramPatchAgent、FormStateExtractor、Studio HTTP、静态 Web 和原生 API 错误/超时中间件均由 starter 安装。`server.rs` 只保留配置装载、Rudi 容器、Tokio listener、统一 starter 安装和 `axum::serve`，这些是启动内核而不是可选功能。
