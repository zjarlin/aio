# Kotlin 插件开发规约

Kotlin 插件根据能力选择目标，不强制把所有代码编译为 Wasm。

## 客户端与可移植逻辑

- 共享业务逻辑放在 `commonMain`。
- 需要 AIO 原生页面时，目标是 `wasm-component` 并实现 [`aio:plugin/page@1`](wit/page.wit)；组件通过 `definition` 返回 `PageDefinition` JSON 数组，通过 `handle` 返回受限请求结果，不直接操作宿主 DOM。
- 独立 Compose Multiplatform 页面只能作为隔离页面运行，不能伪装成 AIO 原生控件树。
- 在 AIO Component ABI 正式发布前，Kotlin/Wasm 页面属于预览目标，不得提交为可自动安装的稳定插件。

## 服务端

纯计算、短请求处理可使用 `wasmWasi`，产物在 `[plugin.runtime]` 声明为 `wasm-component`。需要 JDBC、协程常驻任务、第三方 JVM SDK 或不受 WASI 支持的系统能力时，应构建可执行 JAR 并使用 `process`；公网宿主在进程隔离监督器完成前会拒绝该目标，不允许插件自行守护或退化成未隔离子进程。

## 目录与验证

功能代码优先放在 `plugin/src/commonMain`，平台适配分别放入 `wasmJsMain`、`wasmWasiMain` 或 `jvmMain`。每个功能包包含 `README.md`。

```bash
./gradlew clean check
./gradlew wasmWasiTest
./gradlew jvmTest
```

只运行实际声明目标的任务。产物生成后执行宿主 ABI 校验，生产清单不得引用 Gradle 临时目录。
