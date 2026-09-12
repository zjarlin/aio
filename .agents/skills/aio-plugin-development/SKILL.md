---
name: aio-plugin-development
description: 为 AIO 创建、迁移或验证全栈运行时插件，覆盖同仓前后端、WIT、真实前端资源、宿主能力和可安装性；不用于修改宿主业务页面。
---

# AIO 全栈插件开发

先阅读根 `AGENTS.md` 和 `docs/refactor/README.md`，确认当前切换状态，再阅读对应语言规约。已有规约或 CLI 与已批准的 v2 边界冲突时，直接迁移调用点，不能增加旧协议适配层。

- 一个插件一个 Git 仓库，统一 `frontend/`、`backend/`、`shared/`；强耦合子功能同仓、同版本，不独立安装或回滚。
- 前端编译完整 Web 包，真实 UI 留在自身框架。KMP 必须验证 `@Composable`、`ComposeViewport` 和实际 canvas；不能把 JSON 控件描述当作 Compose UI 交付。
- 后端默认 Wasm Component；只有 JVM/Node/原生 SDK 等实际需求才选 process。数据库通过宿主能力访问，不因需要 PostgreSQL 就改成 process。
- v2 唯一契约是 `lib/plugin/contract/wit/plugin.wit`。导出 `describe`、`health`、`lifecycle`、`handle`；请求响应正文是二进制，页面定义仅为入口、导航、权限和挂载面。
- WIT 生成语言绑定，业务代码不得手工实现 ABI。Rust Dill/TypeId 仅用于进程内注入；来源 UUID、包摘要、页面 ID 分别用于安装、版本、导航。
- 能力声明是申请，实例实际授权由宿主提供。数据库连接凭据不进入 Wasm；业务数据按插件/租户隔离，请求结束时未提交事务回滚。
- Cookie 由宿主写入；插件前端只通过受限桥调用服务，不共享宿主 DOM、语言对象或会话秘密。
- 构建由作者工具链完成；生产安装只处理已编译包，禁止执行安装脚本。前后端、路由、页面和实例锁定同一整包版本。
- 平台和产品不能新增业务插件 Cargo 依赖。编译时 Rust 装配是 host extension，不是运行时市场插件。
- 新 v2 包在现役校验/发布链完成迁移前不得推到旧公网宿主。检查 `docs/refactor/README.md` 中的上线门槛，不把开发预览当作生产验收。

## 验证

Rust 使用独立 workbench 的 `az-ui-components`，不复制组件或 CSS。Kotlin 用仓库固定版本和 SHA256 的 Toolchain wrapper，按 `docs/plugin/kt-plugin-convention.md` 构建前后端。

验证真实后端持久化、能力拒绝、租户隔离、失败不替换版本。浏览器覆盖桌面/移动端 canvas 或 DOM 非空、按钮服务调用、刷新、卸载、导航和控制台；不使用模拟计数冒充数据库。涉及身份和数据切换时先在数据库副本预演，保留旧发布与备份。
