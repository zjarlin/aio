# az-aio-codegen

AIO 中的 `nature-compiler` 宿主插件。产品端只提交母语 `sourceText`；PostgreSQL 保存项目、
revision、生成运行和字段绑定，Rudi 收集能力 Provider，固定生成目录为
`crates/generated/nature`。

生成任务先在临时 crate 运行格式、策略、`cargo check`、生成测试和 Clippy，全部成功后才原子
替换当前生成目录。生成成功不会自动发布；只有运行中 AIO 注册的 artifact hash 与 revision
一致时才能显式发布。
