# AIO 网页架构

## 入口与所有权

| 职责 | 文件 | 约束 |
| --- | --- | --- |
| HTTP 服务、静态资源、Bootstrap | `app/src/server.rs` | 启动时先绑定端口，再执行数据库初始化；`/app/*` 服务 wasm 产物 |
| 全局壳层、路由、侧栏、顶栏 | `app/plugins/studio/src/workbench.rs` | 壳层无业务状态，业务内容来自 Provider 与 ProgramImage |
| Studio 元数据编辑 | `app/plugins/studio/src/ui.rs` | 编辑 `ProgramDefinition`、`PageDefinition`，通过 Graph Patch 保存 |
| 发布后页面运行时 | `app/plugins/studio/src/page_runtime.rs` | 解释编译后的页面声明，渲染 CRUD、树表和 REST 表单 |
| 页面与接口定义 | `app/plugins/studio/src/definition.rs` | 只保存稳定、不可推导的定义数据 |
| 编译校验 | `app/plugins/studio/src/compiler.rs` | 从页面、模型和权限推导运行时结构与诊断 |
| PostgreSQL 存储 | `app/plugins/studio/src/program_store.rs` | 正式程序真源；schema 变更必须有直接迁移 |

## 官方组件

registry 固定为 DioxusLabs/dioxus-components 提交 `bf007c15d0cf4d04d3181cc46cf12325aa773955`。

| 页面能力 | 组件源码 | 使用要求 |
| --- | --- | --- |
| 命令与图标按钮 | `components/button` | 使用 `ButtonVariant`、`ButtonSize`，禁止原生 button |
| 状态与计数 | `components/badge` | 使用 Badge，不复制 badge 样式 |
| 文本、数字、路径输入 | `components/input` | 事件闭包显式标注 `FormEvent` |
| 多行文本 | `components/textarea` | 事件闭包显式标注 `FormEvent` |
| 布尔状态 | `components/checkbox` | 使用 Checkbox；它包含用于表单提交的隐藏 input |
| 对象新增、编辑与确认 | `components/dialog` | 由明确操作打开，关闭后卸载完整 Form |
| 下拉选项 | 原生 `select` | 当前官方 Select 未把 `name` 写入可提交字段，修复前不得迁移 |
| 数据表格 | `components/data_table` | 仓库复合组件；支持编辑、固定表头/列、右侧面板、合并单元格和树形表头 |

`app/assets/dx-components-theme.css` 是官方主题变量。`app/assets/tailwind.css` 只承担已有布局工具类，`app/assets/app.css` 保存 AIO 页面布局与领域样式。不要把页面领域样式写进官方组件 CSS。

## 数据流

```text
PostgreSQL ProgramDefinition/PageDefinition
  -> ProgramCompiler
  -> ApplicationImage/ProgramImage
  -> Rudi Provider 索引
  -> workbench 路由
  -> page_runtime 渲染
```

REST 功能定义以 `method + path` 作为身份。标题为空时从路径末段推导；需求文本只用于一次性生成，不进入持久化元数据。功能列表用表格展示，编辑与删除操作绑定稳定 `SymbolId`。

## 本地运行与验证

前端产物位置为 `target/dx/az-aio-app/release/web/public`，服务端按 release、debug 的顺序发现可用产物。前端修改后先执行 `cd app && dx build --platform web --release`，再启动根目录 `./scripts/preview.zsh`。

验证至少覆盖：

- `GET /health` 返回 HTTP 200 与 `ok`。
- `GET /api/studio/program/draft` 返回 HTTP 200 且 JSON 业务 `code` 为成功值。
- `/app/studio` 加载最新 wasm，不出现 definition 反序列化错误。
- 功能定义页使用 DataTable，能直接编辑路径；新增、编辑和删除操作使用 Dialog，不在页面常驻完整 Form。
- 375px 左右移动端和桌面端均无横向页面溢出；表格自身可以横向滚动。
