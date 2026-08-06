# AIO

AIO 是数据库原生的 Dioxus 零代码应用。PostgreSQL 保存正式 `ProgramDefinition`，业务页面和交互由 Studio 编辑、编译为 `ApplicationImage` 缓存并原子发布，不生成业务 Rust 源码，也不依赖业务文件系统或服务重启。

```text
拖拽 / AI Vibe
    -> PostgreSQL Draft
    -> 类型、Effect、权限、组件门禁
    -> ApplicationImage cache
    -> ArcSwap 原子发布
    -> Dioxus 动态渲染与 Graph VM
```

## 目录

Rust 包只放在两个根目录：

```text
app/
  src/main.rs             单一全栈启动入口
  plugins/                一个领域功能一个插件
  migrations/             PostgreSQL 正式协议迁移
  assets/                 应用静态资源
crates/
  plugin/core/            所有插件共同使用的唯一公共 crate
```

`app/plugins/studio` 同时拥有编辑器、组件目录、ProgramGraph、编译器、发布器、Graph VM、REST/SSE 和 WASM 界面，不再拆成 client、program、runtime、bootstrap、remote-ui 等技术包。场景只是菜单树的根节点，不存在单独的场景数据结构。

`crates/plugin/core` 的 Cargo 工件名是 `az-plugin-core`，统一提供插件 Provider/Contribution、HTTP 边界、共享 PostgreSQL 句柄、动态模型和 JSONB 记录。它不加载插件，也不实现业务功能。

## 依赖方向

```text
az-aio-app
  -> az-studio
  -> az-system-admin / az-asset-hub / az-iot-center / ...
  -> az-plugin-core

各功能插件 -> az-plugin-core
扩展 ProgramGraph Capability 的插件 -> az-studio
```

插件之间不得通过应用入口反向依赖。只有确实被多个插件共同使用的契约或基础能力才进入 `az-plugin-core`。

## 开发

```bash
cargo test --workspace
cd app && dx build --platform web
./scripts/preview.zsh
```

应用启动配置以明文保存在仓库根目录 `.env`。`scripts/preview.zsh` 会先验证直连数据库；直连不可用时，经本机 SOCKS relay 回退到同一数据库，且不会修改 `.env`。启动后访问 `http://127.0.0.1:8080/`。
