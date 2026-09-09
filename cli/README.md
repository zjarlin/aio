# AIO CLI

`aio` 初始化 Web、Desktop、Server 共用的应用壳，并从一个 Git 仓库发现、安装、更新和卸载前后端插件能力。

插件开发规约见仓库 `docs/plugin/`，可运行示例见 <https://github.com/zjarlin/aio-plugin-hello>。

```bash
cargo install --path cli
aio init my-app --title "我的应用"
aio plugin init my-plugin --title "业务插件"
aio plugin init my-kmp-plugin --title "KMP 服务" --language kotlin --runtime process
aio plugin init my-component --title "TS 页面" --language typescript --runtime wasm-component
cd my-app
aio plugin install https://example.com/team/my-plugin.git
aio plugin list
aio plugin validate ../my-plugin
aio plugin uninstall https://example.com/team/my-plugin.git
```

应用仓库的 `aio.toml` 是插件来源真源。每个来源只需配置 `git` 和可选 `rev`；`aio plugin sync` 会拉取仓库、读取根目录 `aio-plugin.toml`、分别发现 client/server Cargo 包、更新 feature 依赖并生成 Dill 注册入口。`aio plugin validate` 则为 Rust、Kotlin 和 TypeScript 仓库提供相同的无副作用协议校验。

Rust 源码插件由应用编译期装配；Kotlin 与 TypeScript 模板生成可在线替换的 `process` 或 `wasm-component` 仓库。安装器不执行远端仓库脚本，只校验已提交 artifact。Rust 运行时插件唯一性只由具体类型的 `TypeId` 决定；Git 地址、Cargo 包名和页面 id 分别只用于来源、构建和业务导航。
