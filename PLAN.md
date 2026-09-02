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
8. 无法声明表达的能力通过 Dill 类型扩展，不无限扩张 DSL。
9. 发布应用壳只消费 `ProgramImage` 和生成页面函数，不持久化运行时组件身份。
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

## 功能定义结构

第一方功能不再按领域建立 Rust 插件。正式结构由 PostgreSQL `ProgramDefinition` 保存，工作区只承载从定义生成的薄扩展点：

```text
app/
└── plugins/studio/       ProgramDefinition、编译器和运行时
generated/apps/<application-id>/ 可删除重建的 ConventionFile 页面函数与发布入口
lib/biz/<application-id>/ Service 契约、生成 Controller 与人工实现
```

- 页面、菜单、模型、权限、method/path 和输入输出只在 `ProgramDefinition` 定义。
- 页面生成文件复用 Studio 通用运行时，不复制领域页面实现。
- 生成 Controller 以 endpoint `SymbolId` 绑定业务契约，人工实现位于独立 Service 文件。
- Dill 只按具体 Rust 类型注册，运行时扩展身份只允许 `TypeId`。
- 生成器保留仍有效的人工实现，删除定义中已经不存在的文件。

## 接口契约

`PageDefinition.endpoints` 是唯一接口真源，保存稳定 `SymbolId`、HTTP method/path、Path/Query/Header/Body 输入和结构化输出。编译器生成 `ProgramImage` 路由，`BusinessModuleManager` 生成领域 Service 契约与 Controller，禁止再建立 Native endpoint 或 `app/src/contracts` 分支。

## 约定文件

约定文件是显式源码扩展点，不是第二份页面或接口定义。

- UI 文件位于 `generated/apps/<application-id>/src/pages` 并导出普通 `render()` 函数。
- server 生成文件位于 `lib/biz/<application-id>/src/generated`，每个 feature 固定拆分为 `controller.rs`、`service.rs`、`service_impl.rs`、`model.rs` 和 `util.rs`；人工修改后的 `service_impl.rs` 由生成器保留。
- Controller 和 Service 统一通过 Dill 注册，不声明字符串 `key`、`name` 或等价身份。
- 页面标题、接口 method/path 和输入输出仍来自 `PageDefinition`。
- 约定源码不得被反向解析后覆盖 `ProgramDefinition`。
- 重新生成不得覆盖仍在正式定义中的人工实现。

## 生成物与版本

- Draft、Revision 和正式定义存入 PostgreSQL。
- 每次发布生成不可变、带内容哈希的 `ResolvedProgram` 和输出产物快照。
- 预览读取当前 Revision 的产物快照，不读取可能已变化的工作区文件。
- CI 或离线构建可导出并提交规范化定义快照，但快照必须能关联数据库 Revision 和 compiler version。
- 所有生成结果排序稳定、换行规范化，相同输入、编译器版本和 Capability 集合必须产生相同哈希。
- schema 不兼容时执行显式迁移或失效旧快照，不在运行时增加兼容分支。

## 实现边界

通用模型、CRUD、表单、列表、树表、权限和流程进入正式定义。无法由运行时直接解释的业务逻辑只能实现到生成的约定 Provider 中；它仍消费同一份页面或接口契约，不得反向修改或覆盖 `ProgramDefinition`。

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

### 阶段四：收敛低代码契约

- [x] 删除资产、配置、IoT、SSH 等第一方领域插件和 contract crate。
- [x] 删除 Native endpoint 协议，统一由 `PageDefinition.endpoints` 生成契约。
- [x] 页面与后端约定文件按正式定义自动同步。
- [x] 服务端启动迁移为 Bevy 风格 `App + Plugin + PluginGroup`。

### 阶段五：增加确定性代码生成后端

- [ ] 实现 Rust server、Dioxus UI、SQL、HTTP contract 和测试后端接口。
- [ ] 先选择一个完整领域做纵向样板，覆盖模型、操作、页面、导航、权限和数据绑定。
- [ ] 生成结构化包，不生成单一超大源码文件。
- [ ] 生成测试并执行 Cargo、wasm、数据库迁移和 HTTP 契约门禁。
- [ ] 验证 Runtime Image 与生成应用的关键行为一致。

### 阶段六：重组约定扩展

- [x] 将接口 Service 与 Controller 生成到 `lib/biz/<application-id>`。
- [x] 保留 `PageDefinition` 和 endpoint `SymbolId` 作为绑定真源。
- [x] 增加重新生成不覆盖人工实现的测试。
- [x] 删除旧 `app/src/contracts` 生成路径及全部调用点。

### 阶段七：Application Linker

- [ ] 聚合多个领域的 `ResolvedProgram`。
- [ ] 保留无关领域模型和产物，不再用单个生成任务覆盖整个应用。
- [ ] 解析跨领域模型、权限、Capability 和接口引用。
- [ ] 检测重复类型、路径和数据库对象冲突。
- [x] 输出一个可部署的应用镜像和 Web、Desktop、Server 多目标生成包。

## 完成标准

- 中文需求和 Studio 操作产生可审计、可回放的结构化 Patch。
- 同一语义模型可以同时运行预览并生成可编译源码。
- Rust、TypeScript、SQL、OpenAPI 和运行时镜像共享同一类型与接口理解。
- 修改中文显示名称不会破坏稳定引用，修改规范英文名会产生明确迁移诊断。
- 第一方功能的 UI、server 和 contract 可以在一个领域包边界内定位。
- 生成代码可删除并从 Revision 完整重建，人工扩展不会被覆盖。
- 多领域链接不会丢失无关模型，并能拒绝跨领域冲突。
- 发布仍遵守 Dill `TypeId` 注册和 PostgreSQL 正式持久化边界。

## 非目标

- 不建设以自由拖拽组件树为核心的页面搭建器。
- 不持久化渲染阶段组件或 `UiOp`。
- 不让自然语言提示词成为正式业务定义。
- 不把生成源码作为第二份可独立修改的真源。
- 不为了覆盖所有业务而把 DSL 扩张成通用编程语言。
- 不拆分独立前端、后端仓库，除非未来出现独立发布、扩缩容或安全边界需求。
