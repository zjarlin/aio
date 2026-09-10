# Kotlin 插件开发规约

Kotlin 插件根据能力选择目标，不强制把所有代码编译为 Wasm。页面模型、事件处理、服务端和共享逻辑全部属于同一个功能 Git 仓库，不拆分客户端和服务端仓库。

## 客户端与可移植逻辑

- 共享业务逻辑放在 `commonMain`。
- 只贡献宿主已有页面形态时，可以在 commonMain 建模，由 JVM 生成器产出 `PageDefinition` JSON，并声明 `kind = "page-definition"`。同一 commonMain 至少同时通过 JVM 与 wasmJs 编译。
- 需要 AIO 原生页面时，目标是 `wasm-component` 并实现 [`aio:plugin/page@1`](wit/page.wit)；组件通过 `definition` 返回 `PageDefinition` JSON 数组，通过 `handle` 返回受限请求结果，不直接操作宿主 DOM。
- 需要运行时按钮事件时使用 `actions` 页面体；`wasm-component` 在 `handle` 中返回新的 `PageBody`，`process` 在 `POST /aio/action` 返回相同结果。插件从宿主注入的当前 `body.state` 计算下一状态，不把租户页面状态保存在 JVM 或 Wasm 实例内。纯 `page-definition` 没有事件实例，校验器会拒绝动作页面。
- 独立 Compose Multiplatform 页面只能作为隔离页面运行，不能伪装成 AIO 原生控件树。
- Kotlin/Wasm Component 当前属于预览目标；进入市场时必须标记 `preview`，锁定 Kotlin、WIT 生成器和 `wasm-tools` 版本，并声明最低宿主版本。

## 服务端

纯计算、短请求处理可使用 `wasmWasi`，产物在 `[plugin.runtime]` 声明为 `wasm-component`。需要 JDBC、协程常驻任务、第三方 JVM SDK 或不受 WASI 支持的系统能力时，应构建可执行 JAR 并使用 `process`，不允许插件自行守护或退化成未隔离子进程。

公网宿主已经通过容器监督器支持 `process` 在线安装、停用、启用、卸载和回滚。当前稳定权限档只接受预置且锁定 digest 的镜像，以及空 `network`、空 `filesystem`、`database = false` 的清单；声明额外能力会在启动前被拒绝。JVM 服务监听 `AIO_PLUGIN_PORT`，并提供清单中的健康检查、`GET /aio/definition` 和业务路由。

## 目录与验证

省略 `--runtime` 时初始化默认的 `process` 仓库；静态页面和预览 Component 是显式覆盖目标：

```bash
aio plugin init ../aio-plugin-kmp --title "KMP 服务" --language kotlin
aio plugin init ../aio-plugin-kmp-pages --title "KMP 页面" --language kotlin --runtime page-definition
aio plugin init ../aio-plugin-kmp-component --title "KMP Component" --language kotlin --runtime wasm-component
```

跨平台代码放在 Toolchain 模块的 `src/`，平台适配分别放入 `src@wasmJs/`、`src@wasmWasi/` 或 `src@jvm/`。每个功能包包含 `README.md`。

仓库提交固定版本和 SHA256 校验的 `kotlin` wrapper，并使用 `project.yaml`、`module.yaml` 描述模块。典型验证命令：

```bash
./kotlin build -m model -p jvm -p wasmJs
./kotlin test -m model -p jvm
./kotlin run -m generator -p jvm -- dist/pages.json
jq . dist/pages.json
aio plugin validate
```

Component 模板把同一模型编译到 JVM、wasmJs 和 wasmWasi，并使用 Kotlin 官方 fork 的 WIT 生成器与 `wasm-tools` 封装：

```bash
./kotlin build -m model -p jvm -p wasmJs -p wasmWasi
./kotlin test -m model -p jvm
WIT_BINDGEN=/path/to/wit-bindgen ./scripts/generate-bindings.sh --check
./scripts/build-component.sh
aio plugin validate
```

当前 Kotlin 运行时会导入 WASI Preview 1 `random_get`。零能力模板使用仓库内最小适配器消除最终宿主导入；它不提供密码学安全随机数，业务代码不得用 `Random.Default` 生成令牌。需要真实随机或其他 WASI 能力时，先扩展清单能力和宿主授权，不得绕过空导入校验。`aio plugin validate` 会实际实例化 Component 并调用 `definition()`，因此该函数必须是无状态、无外部导入且能返回与清单一致的页面。

需要生成 Kotlin DTO 或在 CI 中做结构校验时，先执行 `aio plugin schema schemas` 获取正式 JSON Schema；不要从文档示例或某个宿主实现反推协议模型。

可执行 JAR 使用 Toolchain 原生产物：

```bash
./kotlin package -m service -p jvm -f executable-jar
cp build/tasks/_service_executableJarJvm/service-jvm-executable.jar dist/plugin.jar
aio plugin validate
```

只运行实际声明目标的任务。产物生成后执行宿主协议校验，生产清单不得引用 Toolchain 临时目录。静态页面示例见 [aio-plugin-kmp-counter](https://github.com/zjarlin/aio-plugin-kmp-counter)，Component 示例见 [aio-plugin-kmp-component](https://github.com/zjarlin/aio-plugin-kmp-component)，进程服务示例见 [aio-plugin-kmp-service](https://github.com/zjarlin/aio-plugin-kmp-service)。

三种目标都在清单声明 `[plugin.marketplace]`。完成本地构建后执行 `aio plugin package . --version 1.0.0`，再用来源绑定凭证执行 `aio plugin publish dist/plugin.aio-plugin`。包内容摘要锁定实际运行字节，产物可以被 Git 忽略，发布不依赖 CI。CLI 会等待静态页面、Wasmtime 实例或 JVM 容器激活；完整规则见 [二进制发布](publish.md)。
