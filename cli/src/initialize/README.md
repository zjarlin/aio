# 项目初始化

负责生成可直接构建的 Web/Desktop/Server 应用仓库，以及 Rust、Kotlin Toolchain、TypeScript 插件仓库。模板消费正式插件协议与独立 workbench 壳，不复制宿主组件、CSS 或私有模型。

语言直接决定常规模板：Rust 固定生成源码插件，Kotlin 默认生成 `process`，TypeScript 默认生成 `wasm-component`。`--runtime` 只对 Kotlin/TypeScript 开放，作为 `page-definition` 或其他模板的高级覆盖参数，常规初始化无需传入；Rust 页面扩展实现 trait 后由 Dill 按具体 `TypeId` 聚合，不提供运行目标选择。

模板组合为 Rust 源码插件，以及 Kotlin 与 TypeScript 各自的 `page-definition`、`wasm-component`、`process`。Kotlin Component 当前明确标记为预览。多语言模板生成锁定工具链、功能目录 README、严格请求模型和本地构建说明；在线发布通过统一 CLI 上传二进制包，生产安装器不执行构建，也不要求 artifact 提交到 Git。
