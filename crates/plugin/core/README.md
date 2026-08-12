# Plugin Core

Cargo 工件：`az-plugin-core`

本 crate 解决插件接入 AIO 时共同面对的问题：通用 `Plugin<T>`、默认启用语义、依赖排序、Rudi 发现与安装、Provider 与 Contribution 契约、统一 HTTP 请求响应、共享 PostgreSQL 句柄、Toasty 模型贡献、动态模型、字段定义、JSONB 业务记录、分页查询、校验和计算字段。

它不持有具体应用状态，不实现任何业务功能，也不负责界面渲染或 Studio 编译。只有至少两个插件共同依赖的接入契约或基础能力才允许放入这里。

源码职责：

- `plugin.rs`：插件 Provider、Contribution 和宿主上下文契约。
- `installable_plugin.rs`：通用 `Plugin<T>`、类型依赖和拓扑排序。
- `plugin_discovery.rs`：从 Rudi 收集插件、校验类型身份并统一安装。
- `http.rs`：统一 Axum 请求提取与错误响应。
- `database.rs`：共享 PostgreSQL/Toasty 句柄。
- `records.rs`：动态模型、字段和 JSONB 记录。
- `record_validation.rs`：记录校验与计算字段。
- `upload.rs`：插件共享的受控上传边界。

依赖使用 `PluginType::of::<DatabaseStarter>()` 声明，不维护字符串 ID。插件默认启用，只有确实需要按宿主配置关闭时才重写 `enabled()`。宿主通过 `discover_plugins` 和 `install_plugins` 完成 Rudi 收集与安装。

运行测试：

```bash
cargo test -p az-plugin-core
AZ_AIO_TEST_DATABASE_URL=postgresql://postgres:postgres@127.0.0.1/aio_test \
  cargo test -p az-plugin-core --test postgres
```
