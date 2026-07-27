# az-operation-agent

`az-operation-agent` 使用 Rig typed extractor，把自然语言接口需求转换为 AIO engine 可校验的强类型 `OperationPlan`。

本 crate 只负责生成强类型草稿，不直接写 PostgreSQL、不发布 operation，也不生成或执行 Rust、SQL、Rhai、WASM。能力策略、资源限制和发布状态由 engine 宿主决定。

运行时读取：

- `OPENAI_API_KEY` 或 `API_KEY`
- `OPENAI_BASE_URL`、`OPENAI_BASEURL` 或 `API_BASEURL`
- `AZ_AIO_OPERATION_AGENT_MODEL` 或 `OPENAI_MODEL`

未配置 API key 时，`OperationVibeAgent::from_env` 返回 `None`，AIO 的手写 operation 和已发布接口仍可正常运行。
