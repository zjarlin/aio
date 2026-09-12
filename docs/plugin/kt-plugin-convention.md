# Kotlin 全栈插件规约

新插件采用同仓 `frontend/`、`backend/`、`shared/`，共同发布和回滚。参考本地独立 `aio-plugin-kmp-example`；不要继续生成旧的 actions/PageBody 页面解释器。

## 模块

- `frontend`：`wasm-js/app`，真实 `@Composable` 与 `ComposeViewport`。Skiko/Skia 在浏览器 canvas 绘制，不由 Rust 重画控件。
- `backend`：默认 `wasm-wasi/app`，构建 v2 Component。实现生成的 `PluginRootFunctions.Exports`，数据库操作通过 `Database` 能力，不持有 JDBC 或连接凭据。
- `shared`：无 UI 依赖的跨平台业务模型；不引用 Compose、Dioxus 或宿主实现。

确实需要 JVM SDK、JDBC 驱动或常驻任务时，后端可以选 process；这不是访问宿主 PostgreSQL 的默认方式。

## 契约与工具链

唯一 WIT 源为 `aio-platform/lib/plugin/contract/wit/plugin.wit`，包名 `aio:plugin@2.0.0`。生成 `describe`、`health`、`lifecycle`、`handle` 和宿主能力绑定，不复制 ABI。描述只包含页面入口、场景根、菜单路径、权限与挂载面；二进制请求响应承载实际业务。

已验证参考版本：Toolchain `0.12.0-dev-4233`（wrapper SHA256 固定）、Kotlin `2.4.10`、Compose `1.12.0-beta03`、Material3 `1.12.0-alpha03`、Kotlin/wit-bindgen `700f2db5e1d01f7bee8d756750c6f631171f520e`、wasm-tools `1.240.0`。WASI Preview 1 使用官方 Wasmtime `43.0.2` reactor adapter 并验证 SHA256，不再使用伪随机或常量随机适配器。

```sh
WIT_BINDGEN=/path/to/wit-bindgen sh scripts/generate-bindings.sh
./kotlin test -m shared -p jvm
sh scripts/build.sh
```

示例的 `build.sh` 将 HTML、JS、Wasm、Skiko 与资源放入 `dist/frontend`，并构建用户要求的 Ktor JVM 目标 `dist/plugin.jar`。`build-component.sh` 单独构建 v2 Kotlin Component 到 `dist/plugin.wasm`；同一包只选择一个后端目标。插件作者构建，生产安装不执行这些脚本。前端资源使用包内相对路径，不依赖公网字体或 CDN。

Ktor 示例的 `backend/service` 提供真实 HTTP 任务 CRUD，`shared` 保存序列化模型，Compose 前端通过宿主桥展示后端 mock 数据。JAR 不打入 JVM；进程执行器使用按摘要锁定的外部 JRE 镜像。mock 状态按租户隔离，但进程重启即重置，不能当作正式数据源。

## 验证与发布状态

必须验证实际 canvas、按钮服务调用、数据库持久化、刷新恢复、桌面/移动端尺寸与控制台。平台 `lib/plugin/runtime/tests/component.rs` 用真实 Kotlin Component 覆盖租户隔离、能力拒绝和实例重建；浏览器脚本为示例 `scripts/test-browser.mjs`。

v2 校验/发布链尚在迁移，示例的 `backend/component/aio-plugin.toml` 显式标注 `schema_version = 2`，旧 CLI 应拒绝它。不要删除版本标记、改回 v1 或以旧 `aio plugin publish` 命令绕过门禁。根清单选择既有 process 协议的 Ktor 目标；该目标的发布不代表 v2 上线。上线状态以 `docs/refactor/README.md` 为准。

v2 整包开发工具位于 `lib/plugin/bundle/examples/package.rs`，一起打包前端、Component 与 SQL。清单仅声明产物位置、宿主版本和能力，不再填写 `runtime.kind`，也不重复声明 describe 已提供的页面。使用完整 Git SHA；页面、后端和迁移一起决定摘要。进程内在线替换只允许相同迁移集合，迁移变更必须进入受控维护流程。
