# AIO 开发约定

- 禁止新增旧名称、旧 API 或旧协议的兼容层；变更时直接迁移调用点。
- Admin shell 保持无头结构，业务内容由单一 `AdminProvider` 聚合提供。
- 插件、Provider、Controller 和 Service 统一使用 Dill；运行时扩展身份只允许 `TypeId`，禁止声明字符串 `key`、`name` 或等价身份。
- PostgreSQL 是正式持久化源；内存和内置数据只用于开发、导入或降级。
- Remote UI 正式保存 `PageDefinition`，不持久化渲染阶段的 `UiOp`。
- 第一方目录使用无发布前缀的领域名；Cargo 包名继续使用 `az-` 保证全局唯一。
- Rust 运行时错误默认使用 `anyhow::Result`，关键失败点补充 `Context`。
- 源码文件按职责命名，禁止新增 `api.rs`、`common.rs`、`utils.rs` 等泛化文件。
- 单个源码文件原则上不得超过 800 行；接近上限时必须按页面、领域或组件职责拆分，禁止继续向超大文件追加功能。
- `ui.rs`、`mod.rs` 等入口文件只负责模块声明、导出和顶层编排；具体 Panel、Dialog、表格、表单及其状态和辅助逻辑必须放入按职责命名的独立模块。
- 所有 Dioxus 基础控件及复合组件统一消费 `az-ui-components`；源码归属独立 `dioxus-admin-workbench/crates/ui/components`，AIO 禁止保留本地副本、同类组件目录或兼容转发层。
- 发布应用壳统一消费 submodule `lib/dioxus-admin-workbench` 中的 `az-dioxus-admin-shell::ApplicationShell`；AIO 只保留 `ProgramImage` 模型转换、页面内容和业务回调。
- AIO 禁止保存 CSS 文件、注入 Stylesheet 或编写 inline style；主题、布局、工具样式和控件样式全部由 `az-ui-components` 提供。
- 所有新增代码注释使用中文。
- 对象的新增、编辑 Form 必须由明确操作打开 Dialog，关闭后卸载；禁止把完整 Form 常驻列表旁或散落在页面内容流中。表格单元格内联编辑除外。
- 删除等破坏性操作必须先打开确认 Dialog，不得在操作按钮点击后直接执行。
