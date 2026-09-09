# 项目初始化

负责生成可直接构建的 Web/Desktop/Server 应用仓库，以及 Rust、Kotlin Toolchain、TypeScript 插件仓库。模板消费正式插件协议与独立 workbench 壳，不复制宿主组件、CSS 或私有模型。

模板组合为 `rust + rust-source`，Kotlin 与 TypeScript 各自的 `page-definition`、`wasm-component`、`process`。Kotlin Component 当前明确标记为预览，其余组合为稳定目标。多语言模板都生成锁定工具链、功能目录 README、严格请求模型和预构建产物说明；生产安装器仍只读取开发者提交的 artifact。
