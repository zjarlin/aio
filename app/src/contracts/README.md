# Studio 后端约定契约

本目录只保存 Studio 为 `convention` 接口生成的后端扩展文件。

- 在生成文件的 `handle` 函数中补充业务实现。
- 路由、HTTP 方法、统一响应和 Provider 聚合由 Studio 运行时负责。
- 删除接口元数据时，对应 Rust 文件会同步删除。
- 内置 CRUD 等完全推导代码只写入 `target/aio/studio`，不会进入本目录。
