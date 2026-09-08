# Kotlin 插件开发规约

Kotlin 插件根据能力选择目标，不强制把所有代码编译为 Wasm。

## 客户端与可移植逻辑

- 共享业务逻辑放在 `commonMain`。
- 只贡献宿主已有页面形态时，可以在 commonMain 建模，由 JVM 生成器产出 `PageDefinition` JSON，并声明 `kind = "page-definition"`。同一 commonMain 至少同时通过 JVM 与 wasmJs 编译。
- 需要 AIO 原生页面时，目标是 `wasm-component` 并实现 [`aio:plugin/page@1`](wit/page.wit)；组件通过 `definition` 返回 `PageDefinition` JSON 数组，通过 `handle` 返回受限请求结果，不直接操作宿主 DOM。
- 独立 Compose Multiplatform 页面只能作为隔离页面运行，不能伪装成 AIO 原生控件树。
- 在 AIO Component ABI 正式发布前，Kotlin/Wasm 页面属于预览目标，不得提交为可自动安装的稳定插件。

## 服务端

纯计算、短请求处理可使用 `wasmWasi`，产物在 `[plugin.runtime]` 声明为 `wasm-component`。需要 JDBC、协程常驻任务、第三方 JVM SDK 或不受 WASI 支持的系统能力时，应构建可执行 JAR 并使用 `process`，不允许插件自行守护或退化成未隔离子进程。

公网宿主已经通过容器监督器支持 `process` 在线安装、停用、启用、卸载和回滚。当前稳定权限档只接受预置且锁定 digest 的镜像，以及空 `network`、空 `filesystem`、`database = false` 的清单；声明额外能力会在启动前被拒绝。JVM 服务监听 `AIO_PLUGIN_PORT`，并提供清单中的健康检查、`GET /aio/definition` 和业务路由。

## 目录与验证

跨平台代码放在 Toolchain 模块的 `src/`，平台适配分别放入 `src@wasmJs/`、`src@wasmWasi/` 或 `src@jvm/`。每个功能包包含 `README.md`。

仓库提交固定版本和 SHA256 校验的 `kotlin` wrapper，并使用 `project.yaml`、`module.yaml` 描述模块。典型验证命令：

```bash
./kotlin build -m model -p jvm -p wasmJs
./kotlin test -m model -p jvm
./kotlin run -m generator -p jvm -- dist/pages.json
jq . dist/pages.json
aio plugin validate
```

可执行 JAR 使用 Toolchain 原生产物：

```bash
./kotlin package -m service -p jvm -f executable-jar
cp build/tasks/_service_executableJarJvm/service-jvm-executable.jar dist/plugin.jar
aio plugin validate
```

只运行实际声明目标的任务。产物生成后执行宿主协议校验，生产清单不得引用 Toolchain 临时目录。静态页面示例见 [aio-plugin-kmp-counter](https://github.com/zjarlin/aio-plugin-kmp-counter)，进程服务示例见 [aio-plugin-kmp-service](https://github.com/zjarlin/aio-plugin-kmp-service)。
