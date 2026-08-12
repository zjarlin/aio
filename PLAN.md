# AIO 应用编译器计划

状态：设计基线
日期：2026-08-11

## 目标

AIO 不定位为传统拖拽式低代码平台，而是一个由中文领域建模驱动的应用编译器。

系统接收中文需求、Studio 操作、数据库元数据和现有代码扫描结果，将它们转换为统一的结构化语义模型，再从同一份已解析中间表示生成运行时镜像、Rust、TypeScript、SQL、接口契约、测试、文档和部署描述。

终极形态不是在 DSL 与代码生成之间二选一，而是：

```text
自然语言 / Studio UI / 元数据 / 代码扫描
                    |
              GraphPatchBatch
                    |
             ProgramDefinition
                    |
       normalize -> resolve -> validate -> link
                    |
              ResolvedProgram
           /        |         \
   ProgramImage   源码产物    契约与部署产物
```

## 核心决策

1. PostgreSQL 中版本化的 `ProgramDefinition` 是正式程序真源。
2. `ProgramDefinition` 是结构化领域 DSL，不要求先设计一套新的文本语法。
3. Studio 手工操作和 AI 修改统一输出强类型 `GraphPatchBatch`，不直接改文件或整份覆盖定义。
4. 编译器先把 `ProgramDefinition` 解析为与具体输出技术无关的 `ResolvedProgram`。
5. 运行时解释和代码生成是消费同一 `ResolvedProgram` 的两个并列后端。
6. `ProgramImage` 是运行时产物，不是持久化源；`UiOp`、Dioxus `Element`、HTML、CSS 和 JavaScript 不进入正式定义。
7. 生成代码是可重建产物，不允许与正式模型形成双向同步。
8. 无法声明表达的能力通过 Rudi Provider 扩展，不无限扩张 DSL。
9. Admin shell 保持无头，AIO 只提供一个聚合 `AdminProvider`。
10. 不为旧 DSL、旧生成目录或旧协议增加兼容层，迁移时直接更新调用点和持久化版本。

## 语言与身份模型

业务对象同时保存稳定身份、中文业务语言和规范代码名称：

```json
{
  "id": "稳定 SymbolId",
  "title": "设备生产状态",
  "name": "device_production_status",
  "aliases": ["设备工作状态"],
  "description": "设备当前生产、空闲或停机状态"
}
```

约束：

- `SymbolId` 是引用身份，改名不能破坏模型、页面、权限和函数之间的关联。
- `title` 是面向业务人员的中文名称。
- `name` 是确认后的规范英文领域名，使用 `snake_case` 保存。
- `aliases` 只用于术语识别、导入和历史名称搜索，不作为代码身份。
- Rust `PascalCase`、TypeScript `camelCase`、SQL `snake_case` 等目标名称从 `name` 确定性推导，不重复持久化。
- 大模型只提出结构化命名候选；领域词典、保留字、重名、长度和格式由确定性校验器处理。
- 已确认的领域名进入术语表，后续生成不得重新自由翻译。

## 编译器分层

### 输入前端

以下输入都只负责产生定义或 Patch，不拥有独立业务语义：

- Studio 表格、表单和图编辑器。
- 中文自然语言 Agent。
- JSON、YAML 或其他结构化导入器。
- PostgreSQL、OpenAPI 和外部元数据扫描器。
- Rust、Kotlin、TypeScript 等源码扫描器。

暂不建设新的文本 DSL。只有出现明确的 Git diff、批量手写或外部生态需求时，才增加文本语法，并将它作为 `ProgramDefinition` 的另一个前端。

### 正式定义

`ProgramDefinition` 保存不可推导的领域声明：

- 应用、场景、菜单和路由。
- 模型、字段、关系、索引、查询、校验和审计语义。
- 页面、页面扩展和页面消费的自定义接口。
- 函数、端口、节点、连线和 Effect。
- 权限和 Capability 引用。

显示名称、生成文件路径、目标语言大小写、内置 CRUD 路由等可推导信息不重复保存。

### 解析中间表示

新增 `ResolvedProgram` 作为所有输出后端的唯一输入，至少完成：

- 稳定 ID 与名称解析。
- 类型、泛型、可空性和关联目标绑定。
- 跨页面、模型、函数和权限的引用解析。
- Capability 选择与 Provider 绑定。
- REST method、path、输入和输出解析。
- 权限传播、Effect 检查和执行目标划分。
- 默认值展开和可推导定义补全。
- 跨领域符号链接、冲突检测和确定性排序。

禁止各输出后端直接重复解释原始 `ProgramDefinition`，避免 UI、server、SQL 和 SDK 产生不同语义。

### 编译后端

计划提供以下并列后端：

| 后端 | 输出 | 用途 |
| --- | --- | --- |
| Runtime Image | `ProgramImage`、endpoint image | Studio 预览、热切换、通用 CRUD 和流程运行 |
| Rust Server | 类型、服务、Provider、路由装配 | 正式服务端构建 |
| Dioxus UI | 页面扩展、表格、表单和 Dialog 装配 | 正式 Web 构建 |
| SQL | schema、索引和版本化迁移 | PostgreSQL 持久化 |
| HTTP Contract | OpenAPI、共享 DTO、类型化客户端 | 前后端协作和外部集成 |
| Test | 契约、校验、权限和行为测试 | 生成结果门禁 |
| Deployment | 应用清单、Capability 和环境需求 | 发布与运维 |

开发预览优先使用 Runtime Image；需要编译检查、性能、外部集成、离线构建或代码审查时生成源码和契约。二者必须通过同一组语义一致性测试。

## 功能包结构

第一方功能按领域纵向组织，不拆成全局 frontend/backend 两棵目录。一个功能使用一个领域插件，插件内部区分 `ui` 和 `server`，共享类型放入独立 contract crate：

```text
app/plugins/asset-hub/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── descriptor.rs
    ├── ui/
    │   ├── mod.rs
    │   ├── asset_workspace.rs
    │   ├── asset_table.rs
    │   └── asset_dialog.rs
    └── server/
        ├── mod.rs
        ├── asset_routes.rs
        ├── asset_store.rs
        └── skill_scanner.rs

crates/asset-hub-contract/
└── src/
    ├── lib.rs
    ├── asset.rs
    └── endpoints.rs
```

- `ui` 仅在 `wasm32` 编译，负责页面、交互状态和类型化请求。
- `server` 仅在 native 编译，负责路由、业务服务、Repository 和外部资源。
- contract 不依赖 Dioxus、Axum、SQLx，拥有 DTO 和 endpoint 声明。
- `descriptor.rs` 只声明插件元数据、菜单、Capability 和贡献。
- `lib.rs`、`ui/mod.rs`、`server/mod.rs` 只负责声明、导出和顶层编排。
- 具体源码按职责命名，不新增 `api.rs`、`common.rs`、`utils.rs` 等泛化文件。

## 接口契约

第一方原生接口由 contract crate 提供唯一声明：

- 稳定接口标识。
- HTTP method 和 path。
- Path、Query、Header、Body 输入类型。
- 成功响应 `data` 类型。
- 错误响应和权限要求。

UI client、Axum Router、插件 Contribution 和 OpenAPI 后端共同消费该声明，禁止分别硬编码路径。

Studio 动态接口继续以 `PageDefinition.endpoints` 为真源。`method + path` 构成接口身份，输入输出使用结构化类型；Rudi 后端 Provider 使用稳定 endpoint `SymbolId` 绑定具体实现。

## 约定文件与原生扩展

约定文件是显式源码扩展点，不是第二份页面或接口定义。

建议把同一页面的扩展源码聚合到同一功能目录：

```text
app/src/features/<application>/<page>/
├── ui/
│   ├── mod.rs
│   ├── page.rs
│   └── result_table.rs
└── server/
    ├── mod.rs
    ├── submit_order.rs
    └── query_orders.rs
```

- UI 文件实现 `ConventionPageProvider` 或正式页面扩展 Provider。
- server 文件实现 `ConventionEndpointProvider`。
- Provider 统一由 Rudi 编译期注册，不引入第二套 DI 或手写注册表。
- 页面标题、接口 method/path 和输入输出仍来自 `PageDefinition`。
- 约定源码不得被反向解析后覆盖 `ProgramDefinition`。
- 生成源码与手写扩展分目录保存，重新生成不得覆盖人工实现。

## 生成物与版本

- Draft、Revision 和正式定义存入 PostgreSQL。
- 每次发布生成不可变、带内容哈希的 `ResolvedProgram` 和输出产物快照。
- 预览读取当前 Revision 的产物快照，不读取可能已变化的工作区文件。
- CI 或离线构建可导出并提交规范化定义快照，但快照必须能关联数据库 Revision 和 compiler version。
- 所有生成结果排序稳定、换行规范化，相同输入、编译器版本和 Capability 集合必须产生相同哈希。
- schema 不兼容时执行显式迁移或失效旧快照，不在运行时增加兼容分支。

## 逃生边界

DSL 只覆盖可稳定声明和验证的能力，不尝试表达任意程序。

- 通用模型、CRUD、表单、列表、树表、权限和常见流程进入正式定义。
- 可复用的复杂能力实现为带类型契约的标准 Provider。
- 特殊算法、工业协议、设备驱动和复杂交互实现为原生扩展。
- AI 可以生成扩展实现和测试，但生成代码必须经过正常编译、测试和人工审查。
- 不建立生成源码到语义模型的通用双向同步；源码导入只能形成待确认的结构化候选。

## 实施阶段

### 阶段一：固定语义基线

- [ ] 补充 `ProgramDefinition`、`GraphPatchBatch` 和 `ProgramImage` 的职责文档。
- [ ] 为稳定 ID、可推导字段和持久化边界增加契约测试。
- [ ] 建立同输入同哈希的确定性编译测试。
- [ ] 记录现有 Runtime Image 输出，作为后续重构基线。

### 阶段二：建立命名编译器

- [ ] 定义领域术语和命名提案结构。
- [ ] 建立中文术语、规范英文名和历史别名的管理入口。
- [ ] 实现重名、保留字、格式和目标语言冲突校验。
- [ ] 统一生成 Rust、TypeScript、SQL 和 HTTP 字段名。
- [ ] 让 AI 只输出命名 Patch，并支持人工确认后冻结。

### 阶段三：引入 `ResolvedProgram`

- [ ] 定义与 Dioxus、Axum、SQLx 无关的解析 IR。
- [ ] 从 `ProgramCompiler` 提取名称、类型、引用、权限和 Capability 解析阶段。
- [ ] 让现有 `ProgramImage` 后端只消费 `ResolvedProgram`。
- [ ] 建立诊断定位，所有错误关联稳定 `SymbolId` 和 compiler stage。

### 阶段四：收敛功能包和接口契约

- [ ] 将资产中心、配置中心等业务 UI 移回对应领域插件的 `ui/`。
- [ ] 将现有 `backend/` 直接迁移为 `server/`，不保留兼容转发层。
- [ ] 调整插件 Cargo target 依赖，阻止 Dioxus 进入 server 构建、Axum 进入 wasm 构建。
- [ ] 在各 contract crate 中收敛 DTO 和 endpoint 声明。
- [ ] 消除 UI、Router 和 Contribution 中重复的 method/path 字符串。

### 阶段五：增加确定性代码生成后端

- [ ] 实现 Rust server、Dioxus UI、SQL、HTTP contract 和测试后端接口。
- [ ] 先选择一个完整领域做纵向样板，覆盖模型、操作、页面、导航、权限和数据绑定。
- [ ] 生成结构化包，不生成单一超大源码文件。
- [ ] 生成测试并执行 Cargo、wasm、数据库迁移和 HTTP 契约门禁。
- [ ] 验证 Runtime Image 与生成应用的关键行为一致。

### 阶段六：重组约定扩展

- [ ] 将约定页面和约定接口生成到同一功能目录的 `ui/server` 子目录。
- [ ] 保留 `PageDefinition` 和 endpoint `SymbolId` 作为绑定真源。
- [ ] 增加重新生成不覆盖人工实现的测试。
- [ ] 删除旧 `app/src/pages`、`app/src/contracts` 生成路径及全部调用点。

### 阶段七：Application Linker

- [ ] 聚合多个领域的 `ResolvedProgram`。
- [ ] 保留无关领域模型和产物，不再用单个生成任务覆盖整个应用。
- [ ] 解析跨领域模型、权限、Capability 和接口引用。
- [ ] 检测重复类型、路径、Provider key 和数据库对象冲突。
- [ ] 输出一个可部署的应用镜像和多目标生成包。

## 完成标准

- 中文需求和 Studio 操作产生可审计、可回放的结构化 Patch。
- 同一语义模型可以同时运行预览并生成可编译源码。
- Rust、TypeScript、SQL、OpenAPI 和运行时镜像共享同一类型与接口理解。
- 修改中文显示名称不会破坏稳定引用，修改规范英文名会产生明确迁移诊断。
- 第一方功能的 UI、server 和 contract 可以在一个领域包边界内定位。
- 生成代码可删除并从 Revision 完整重建，人工扩展不会被覆盖。
- 多领域链接不会丢失无关模型，并能拒绝跨领域冲突。
- 发布仍遵守单一 `AdminProvider`、Rudi 编译期注册和 PostgreSQL 正式持久化边界。

## 非目标

- 不建设以自由拖拽组件树为核心的页面搭建器。
- 不持久化渲染阶段组件或 `UiOp`。
- 不让自然语言提示词成为正式业务定义。
- 不把生成源码作为第二份可独立修改的真源。
- 不为了覆盖所有业务而把 DSL 扩张成通用编程语言。
- 不拆分独立前端、后端仓库，除非未来出现独立发布、扩缩容或安全边界需求。
