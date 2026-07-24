# AIO

AIO 是独立的 Rust 管理工作台仓库，使用 Axum、Dioxus、Rudi、PostgreSQL 和声明式 Remote UI。

## 目录

- `web/`：Axum 服务端与工作台入口。
- `client/`：Dioxus Web 客户端。
- `platform/`：无头 admin shell、插件契约和共享数据库底座。
- `plugins/`：按大功能拆分的业务插件。
- `crates/`：AIO 独占或紧密耦合的基础能力。
- `migrations/`：PostgreSQL 正式迁移。

通用 Rust crate 只在 [`addzero-lib-rust`](https://github.com/zjarlin/addzero-lib-rust) 维护；本仓库通过固定 Git 提交消费，不保留源码副本。

## 开发

```bash
cargo test -p az-remote-ui -p az-engine -p az-aio-web
cargo run -p az-aio-web
```

服务端默认读取 `DATABASE_URL`。未配置 PostgreSQL 时，只允许明确声明的开发态降级页面，不作为正式数据源。

Remote UI 组件只通过 Rudi 注册，页面以 `PageDefinition` 保存到 PostgreSQL，运行时编译为 `UiOp`。
