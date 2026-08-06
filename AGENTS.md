# AIO 开发约定

- 禁止新增旧名称、旧 API 或旧协议的兼容层；变更时直接迁移调用点。
- Admin shell 保持无头结构，业务内容由单一 `AdminProvider` 聚合提供。
- Provider 和 Remote UI 组件统一使用 Rudi 编译期注册，不引入第二套 DI。
- PostgreSQL 是正式持久化源；内存和内置数据只用于开发、导入或降级。
- Remote UI 正式保存 `PageDefinition`，不持久化渲染阶段的 `UiOp`。
- 第一方目录使用无发布前缀的领域名；Cargo 包名继续使用 `az-` 保证全局唯一。
- Rust 运行时错误默认使用 `anyhow::Result`，关键失败点补充 `Context`。
- 源码文件按职责命名，禁止新增 `api.rs`、`common.rs`、`utils.rs` 等泛化文件。
- 所有新增代码注释使用中文。
- 对象的新增、编辑 Form 必须由明确操作打开 Dialog，关闭后卸载；禁止把完整 Form 常驻列表旁或散落在页面内容流中。表格单元格内联编辑除外。
- 删除等破坏性操作必须先打开确认 Dialog，不得在操作按钮点击后直接执行。
