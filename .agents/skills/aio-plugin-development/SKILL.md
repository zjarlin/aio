---
name: aio-plugin-development
description: 为 AIO 创建、迁移或验证社区插件时使用，覆盖 Git 仓库清单、页面与服务能力、Rust/Kotlin/TypeScript 运行目标、市场条目和可安装性验证。不用于修改 AIO 宿主自身页面。
---

# AIO 插件开发

先阅读仓库根 `AGENTS.md` 和 `docs/plugin/README.md`。根据插件实现语言只继续阅读一份规约：

- Rust：`docs/plugin/rs-plugin-convention.md`
- Kotlin：`docs/plugin/kt-plugin-convention.md`
- TypeScript：`docs/plugin/ts-plugin-convention.md`

## 交付边界

- 一个 Git 仓库就是一个发布和回滚单元；运行版本必须锁定到提交 SHA。
- `aio-plugin.toml` 只描述可发现的构建产物和运行目标，不声明字符串运行时身份。Git 来源与提交只承担安装身份，Rust 运行时扩展继续由 Dill 按具体 `TypeId` 聚合。
- 页面、服务和共享模型分开；前后端可以只提供一端，但不得把服务端依赖带进浏览器 Wasm。
- Rust 页面只使用 `az-ui-components` 和 `az-dioxus-admin-shell`，不自带 CSS，不复制宿主组件。
- 多语言插件优先使用稳定的 AIO WIT/PageDefinition 边界；需要数据库、长任务、网络监听或系统权限时使用隔离进程或容器，不把 WASI 权限扩大成宿主权限。
- 在线 Wasm 插件必须使用 `docs/plugin/wit/page.wit`，在 `[plugin.runtime]` 声明 `kind = "wasm-component"` 与预构建 artifact，并验证 `definition`、`handle` 两个导出。
- Wasm Component 实例按租户、来源和 revision 隔离；安装/启用先通过 `definition` 校验页面，停用/卸载/回滚必须销毁对应实例，不能只切换数据库状态。
- KMP 静态页面插件可以用 commonMain 模型和 JVM 生成器产出 `PageDefinition` JSON，声明 `kind = "page-definition"`；必须用仓库内 `kotlin` wrapper 同时编译 JVM 与 wasmJs，并提交生成产物。
- 当前公网宿主已经激活容器化 `process` 监督器；只为预置 digest 镜像和零额外能力清单声明在线可安装，额外网络、文件系统或数据库能力仍必须标注为未开放。
- 公网安装阶段不得执行仓库脚本。构建和测试在插件作者 CI 或受控发布器中完成，安装器只校验已提交 artifact 并原子切换；失败保留上一版本。

## 完成验证

Rust 插件至少执行 `cargo fmt --all --check`、`cargo test --workspace`，再安装进一个临时 AIO 宿主，分别检查 Web 与 Server feature。Kotlin 与 TypeScript 按各自规约产出后，还要在插件仓库执行 `aio plugin validate`；该命令使用与宿主共享的 `az-plugin-manifest` 校验清单、artifact、子插件依赖图、PageDefinition 和 Component ABI。

需要进入社区市场时，在 `marketplace/registry/` 增加单独条目；展示名称、说明和标签只属于市场元数据，不进入运行时插件身份。
