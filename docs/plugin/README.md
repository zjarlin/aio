# AIO 插件协议

AIO 宿主负责壳、租户组合、权限、安装事务和回滚。社区仓库负责贡献页面、服务或两者。租户只保存 Git 来源与锁定提交，不复制插件源码，也不以市场条目作为运行时身份。

## 运行形态

| 形态 | 适用能力 | 生命周期 |
| --- | --- | --- |
| `rust-source` | 与宿主同版本的 Dioxus 页面、Dill Service/Controller | 拉取源码后隔离编译，整套版本原子切换 |
| `page-definition` | 多语言生成的静态页面定义 | 校验 JSON 后直接按租户挂载，可单独卸载 |
| `wasm-component` | 多语言纯计算、PageDefinition 生成、受限请求处理 | Wasmtime 实例化，按声明授予 WASI 能力，可单独卸载 |
| `process` | JVM/Node、数据库驱动、长任务、第三方 SDK | 独立进程或容器，健康检查后挂载路由，停止即卸载 |

Wasm 是多语言 ABI 的优先选项，但不是所有插件的唯一运行时。浏览器插件不得直接接管宿主 DOM；它返回 `PageDefinition` 和事件结果，由宿主统一渲染。服务端 Wasm 只获得清单声明且租户允许的 WASI 能力。需要任意网络、原生数据库驱动或长期后台任务时使用 `process`。

## 仓库清单

Rust 第一阶段使用两个独立 crate：

```toml
[plugin.client]
path = "client"

[plugin.server]
path = "server"
```

至少声明一端。`client` 导出 `register(&mut dill::CatalogBuilder)`；`server` 导出同名注册函数和 `router(&dill::Catalog)`。宿主生成装配代码，运行时扩展只依赖具体 Rust 类型的 `TypeId`。

在线安装的多语言仓库统一声明一个已经构建好的运行产物，不改变 Git 安装模型：

```toml
[plugin.runtime]
kind = "wasm-component"
artifact = "dist/plugin.wasm"

[plugin.capabilities]
network = []
filesystem = []
database = false

[[plugin.subplugins]]
id = "hello-screen"
pages = ["hello"]
routes = ["hello"]
```

`process` 仓库必须把启动和停止契约明确写入清单，不允许安装器猜测脚本：

```toml
[plugin.runtime]
kind = "process"
artifact = "dist/plugin.jar"
host_version = ">=2026.5.10"
entrypoint = ["java", "-jar", "dist/plugin.jar"]
health_check = "/health"
shutdown_timeout_seconds = 10
```

只贡献静态页面定义的 Kotlin/TypeScript 插件可以声明 `kind = "page-definition"`，artifact 是 `PageDefinition` JSON 数组。它适合由跨平台 common 模型生成页面；需要动态后端请求处理时再升级为 Component。

Wasm Component 必须实现 [`aio:plugin/page@1`](wit/page.wit)：`definition() -> string` 返回 `PageDefinition` JSON 数组，`handle(request: string) -> string` 返回包含 `status`、`content_type` 和 `body` 的 JSON。宿主为每次调用创建无默认 WASI 权限且带 fuel 上限的实例。

源码宿主 CLI 只负责 `rust-source` 的 Cargo 装配。公网运行时已支持 `wasm-component` 在线安装、停用、启用、卸载和回滚。进程隔离监督器未激活前，`process` 清单会明确拒绝安装；不得把普通子进程伪装成已经具备网络、文件系统和资源隔离的正式运行时。

当前 `PageDefinition.body` 支持以下稳定形态：

```json
{
  "id": "hello",
  "label": "Hello",
  "icon": "panels-top-left",
  "scene": { "id": "workspace", "label": "工作区" },
  "required_permission": null,
  "body": { "kind": "counter", "title": "Hello Runtime", "button": "计数加一" }
}
```

纯文本页面把 `body` 改为 `{ "kind": "text", "title": "...", "content": "..." }`。渲染阶段的 `UiOp`、HTML、CSS 和 Dioxus `Element` 都不能作为持久化协议。

## 租户组合

每个租户拥有独立的组合文件和锁文件。组合文件可跟踪分支、标签或提交，生产锁文件只保存解析后的完整提交 SHA：

```toml
[[plugins]]
git = "https://github.com/example/aio-plugin-orders.git"
rev = "9d0b7d16f9f5a4c5a3b4c0e1e6c43ae8d47aa001"
```

安装流程固定为：解析来源、拉取到隔离区、校验清单、构建两端、运行测试与健康检查、写锁文件、原子切换。卸载执行相反切换并停止进程/Wasm 实例；失败时不修改活动版本。

## 市场

`marketplace/registry/` 是可审计的静态目录。每个插件一个 TOML，合并后即可被索引；是否要求人工审核由仓库分支保护决定，不进入协议。市场只负责发现，租户始终可以直接配置未收录的 Git 仓库。

语言细节见 [Rust 规约](rs-plugin-convention.md)、[Kotlin 规约](kt-plugin-convention.md) 和 [TypeScript 规约](ts-plugin-convention.md)。

提交市场前必须在插件仓库执行语言自身的构建与测试，然后使用宿主共享校验器：

```bash
aio plugin validate
```

该命令不执行仓库脚本；它只读取已生成 artifact，校验清单、页面声明与 Wasm Component WIT 边界。
