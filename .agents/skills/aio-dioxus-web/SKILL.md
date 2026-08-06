---
name: aio-dioxus-web
description: 在 AIO 仓库中创建、迁移或调试 Dioxus Web 页面、Studio 工作台、运行时表格、表单和组件样式时使用。强制使用仓库内固定版本的 Dioxus Components 源码组件，遵守 AdminProvider、Rudi、PageDefinition 持久化边界，并完成 wasm、服务端和浏览器验证。
---

# AIO Dioxus Web

## 目标

在不引入第二套组件库或兼容层的前提下维护整个 AIO 网页。组件、页面元数据、运行时 Provider 和服务端持久化各守自己的职责边界。

开始修改前完整阅读 [references/site-architecture.md](references/site-architecture.md)。涉及表格时继续阅读 [references/data-table.md](references/data-table.md)，并检查仓库根目录 `AGENTS.md` 与当前 `git status`。

## 组件规则

- 基础交互控件只使用 `app/plugins/studio/src/components` 中由官方 Dioxus Components registry 拷入的组件。
- 页面代码禁止直接渲染原生 `button`、文本 `input`、`textarea` 或自行实现同名包装组件。
- 不新增 `design_system.rs`，不增加旧组件 API 适配层；直接迁移所有调用点。
- 数据表格统一使用仓库 `components/data_table`。这是上游缺少 Table 后的正式复合组件，页面不得再手写数据表格结构。
- DataTable 必须保持语义化 table，并通过列树、单元格/表头 renderer、编辑器和右侧面板插槽扩展；不得把业务字段写死进组件。
- 完整业务 Form 由新增、编辑操作打开官方 Dialog，关闭后卸载；不得常驻在 DataTable `right_panel` 或页面内容流中。单元格内联编辑不受此限制。
- 删除等破坏性表格操作必须先打开确认 Dialog。
- 原生 `select` 暂时保留。固定提交中的 Select 虽声明 `name`，但未渲染可提交字段；迁移前必须先验证上游已修复表单提交契约。
- 只纳入实际使用的 registry 组件。需要新增时运行：

```bash
cd app/plugins/studio
dx components add <name> \
  --module-path src/components \
  --global-assets-path ../../assets \
  --git https://github.com/DioxusLabs/dioxus-components.git \
  --rev bf007c15d0cf4d04d3181cc46cf12325aa773955
```

- CLI 生成后把 `dioxus-primitives` 和 `dioxus-icons` 保持在 wasm32 依赖区，并给 Git 依赖保留 `rev`。不要让前端组件依赖进入服务端构建。
- 官方组件源码可以按项目契约做窄修改，但不改变公开语义；新增注释必须使用中文。

## 实施流程

1. 从实际路由、页面截图或失败请求复现问题，确认 HTTP 状态、JSON 业务码和浏览器控制台状态。
2. 定位页面所有者：壳层改 `workbench.rs`，Studio 配置改 `ui.rs`，发布后动态页面改 `page_runtime.rs`。
3. 先复用现有官方组件；缺少组件时才用固定 registry 提取源码。图标继续使用仓库 `icons`，按钮必须提供可访问名称。
4. 表格先声明列树和稳定 row key，再提供 renderer。可编辑列必须同时声明 `editable`、`can_edit` 和 `render_editor`；合并单元格只传 `DataTableSpan`；操作列只触发 Dialog 或明确命令。
5. 元数据遵循可推导可省略：REST 方法与路径组成身份，显示名称等可从稳定字段推导的值不重复持久化。
6. Admin shell 保持无头；业务页面由单一 AdminProvider 聚合，Provider 继续由 Rudi 编译期注册。
7. PostgreSQL 只保存 `PageDefinition` 等正式定义，不保存渲染阶段 `UiOp`、Dioxus `Element`、HTML、CSS 或 JavaScript。
8. 修改运行时行为后重建 wasm，停止旧 8080 进程再启动新服务，避免旧二进制读取新 schema。

## 完成门槛

依次执行并记录结果：

```bash
cargo fmt --all --check
cargo test -p az-studio
cargo test -p az-aio-app
cd app && dx build --platform web --release
```

启动 `./scripts/preview.zsh` 后，在桌面和移动端视口验证目标路由。至少检查：页面非空、表格列与操作可用、对话框不溢出、无重叠、控制台无错误、关键保存请求返回成功业务码。
