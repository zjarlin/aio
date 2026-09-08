# AIO 插件协议

AIO 宿主负责壳、租户组合、权限、安装事务和回滚。社区仓库负责贡献页面、服务或两者。租户只保存 Git 来源与锁定提交，不复制插件源码，也不以市场条目作为运行时身份。

## 运行形态

| 形态 | 适用能力 | 生命周期 |
| --- | --- | --- |
| `rust-source` | 与宿主同版本的 Dioxus 页面、Dill Service/Controller | 拉取源码后隔离编译，整套版本原子切换 |
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

稳定 Component ABI 发布后，多语言仓库改为声明构建产物，不改变 Git 安装模型：

```toml
[plugin.client]
runtime = "wasm-component"
artifact = "dist/client.wasm"
world = "aio:plugin/page@1"

[plugin.server]
runtime = "process"
command = ["java", "-jar", "dist/plugin.jar"]
health = "/health"
```

当前 CLI 只接受 `rust-source` 的目录清单；在 WIT Host 和进程监督器落地前，非 Rust 清单必须明确报告“不支持”，不得伪装安装成功。

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
