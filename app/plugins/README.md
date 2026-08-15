# AIO Plugins

这里只保留 `studio`。第一方领域模型、页面、菜单和后端接口契约统一进入 PostgreSQL `ProgramDefinition`，不再为资产、IoT、SSH 等领域建立 Rust 插件。

`studio` 负责低代码定义、编辑、编译、发布和运行，并从正式定义同步页面函数以及 `lib/biz/<application-id>` 下的 Service/Controller。
