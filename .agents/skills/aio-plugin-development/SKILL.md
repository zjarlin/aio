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

- 一个功能 Git 仓库包含前端、后端、共享逻辑和可选子插件，不拆成前后端两个仓库。源码装配锁定 Git SHA；在线发布锁定 `.aio-plugin` 包内容 SHA-256。
- `aio-plugin.toml` 只描述可发现的构建产物和运行目标，不声明字符串运行时身份。Git 来源与提交只承担安装身份，Rust 运行时扩展继续由 Dill 按具体 `TypeId` 聚合。
- 页面、服务和共享模型分开；前后端可以只提供一端，但不得把服务端依赖带进浏览器 Wasm。
- Rust 页面只使用 `az-ui-components` 和 `az-dioxus-admin-shell`，不自带 CSS，不复制宿主组件。
- 多语言插件优先使用稳定的 AIO WIT/PageDefinition 边界；需要数据库、长任务、网络监听或系统权限时使用隔离进程或容器，不把 WASI 权限扩大成宿主权限。
- 在线 Wasm 插件必须使用 `docs/plugin/wit/page.wit`，在 `[plugin.runtime]` 声明 `kind = "wasm-component"` 与预构建 artifact，并验证 `definition`、`handle` 两个导出。
- Wasm Component 实例按租户、来源和 revision 隔离；安装/启用先通过 `definition` 校验页面，停用/卸载/回滚必须销毁对应实例，不能只切换数据库状态。
- KMP 静态页面插件可以用 commonMain 模型和 JVM 生成器产出 `PageDefinition` JSON，声明 `kind = "page-definition"`；使用仓库内 `kotlin` wrapper 验证 JVM 与 wasmJs，把生成产物放入二进制包，不要求提交到 Git。
- 当前公网宿主已经激活容器化 `process` 监督器；只为预置 digest 镜像和零额外能力清单声明在线可安装，额外网络、文件系统或数据库能力仍必须标注为未开放。
- 公网安装阶段不得执行仓库脚本。构建和测试在插件作者工具链中完成，安装器校验二进制包并原子切换；进程监督器停止或数据库切换失败时保留并恢复上一版本。
- 在线发布使用来源和租户绑定的发布凭证。先读 `docs/plugin/publish.md`，再执行 `aio plugin package . --version <SemVer>` 和 `aio plugin publish dist/plugin.aio-plugin`。已有包可在没有 Git 的目录直接上传；不要求 GitHub Actions、Git 证明或已提交 artifact。
- 各语言共用 `az-plugin-package` 协议和 CLI 上传/轮询实现，不在插件仓库拼接 JSON。PostgreSQL 保存包与元数据，同来源同版本不可覆盖不同内容；源码 SHA 只是可选溯源参考。

## 完成验证

Rust 插件至少执行 `cargo fmt --all --check`、`cargo test --workspace`，再安装进一个临时 AIO 宿主，分别检查 Web 与 Server feature。Kotlin 与 TypeScript 按各自规约产出后，还要在插件仓库执行 `aio plugin validate`；该命令使用与宿主共享的 `az-plugin-manifest` 校验清单、artifact、子插件依赖图、PageDefinition 和 Component ABI。

需要进入社区市场时，在 `aio-plugin.toml` 声明 `[plugin.marketplace]`，并可在 `marketplace/registry/` 增加用于冷启动发现的单独条目；展示名称、说明和标签只属于市场元数据，不进入运行时插件身份。配置来源绑定凭证后执行 `aio plugin publish`，确认后台任务进入 `active`。
