# az-engine

`az-engine` 是 addzero 低代码引擎核心库，负责元模型、动态字段、Rhai 钩子、JSON payload 记录、集合化计算字段求值和可版本化 operation 运行时。

同时提供可复用的低代码文本校验内置，例如 `text(value).trim().not_blank(...).starts_with_any(...)`；规则按声明顺序执行，失败时返回规则配置的业务错误。

首版设计只面向 PostgreSQL 正式持久化，不提供内存降级，不迁移旧 `biz_lowcode_*` 数据表。

## 声明式页面

页面以 `PageDefinition` 保存到 `engine_page_definitions`，REST API 和 CLI contract 共用 `engine.pages.*` 操作定义。数据库不保存 `UiOp`；每次渲染都使用当前 Rudi 组件 catalog 校验页面配置并编译操作流，因此组件 schema 是唯一能力来源。

页面属性只接受字面量和数据路径绑定，数据源及动作只能引用 engine operation。内置 JSON 页面只用于无数据库开发预览或首次导入，不作为正式持久化源。

## 动态 Operation

在线接口由稳定的 `OperationDefinition` 和不可变 `OperationRevision` 组成。创建或 Agent 生成后先进入 `draft`，试运行不改变线上版本；只有显式发布的 revision 才能通过统一网关调用：

```text
GET|POST /api/engine/invoke/{operation_key}
```

Rhai revision 可读取以下变量：

- `request`：完整请求上下文
- `body`：JSON body，空 body 为 `{}`
- `query`：多值 query map，每个字段都是字符串数组
- `operation_key`：当前 operation 标识
- `method`：`GET` 或 `POST`

脚本运行时不注册文件、网络、数据库或 shell 能力，并限制操作数、调用深度、集合大小和墙钟执行时间。每次试运行和正式调用都会写入 `engine_operation_runs` 审计表。

自然语言生成由独立的 `az-operation-agent` 完成。Agent 只生成强类型草稿，不能决定执行器类型、capability policy、资源限制或发布状态。

如需要在确认无回滚需求后人工清理旧表，可单独执行：

```sql
DROP TABLE IF EXISTS biz_lowcode_record;
DROP TABLE IF EXISTS biz_lowcode_app_screen;
DROP TABLE IF EXISTS biz_lowcode_meta_field;
DROP TABLE IF EXISTS biz_lowcode_meta_model;
```
