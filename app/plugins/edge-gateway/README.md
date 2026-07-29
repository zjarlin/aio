# Edge Gateway

Cargo 工件：`az-edge-gateway`

本插件解决边缘流程执行、请求模板渲染、响应捕获、调用审计和受控外部资产调用问题，拥有 `biz_edge_gateway_*` PostgreSQL 模型以及 `/api/edge-gateway/*` 后端接口。

外部天气等能力必须经过类型化资产与令牌门禁，不能让 ProgramGraph 直接执行任意 URL。插件不保存业务页面；边缘网关页面由数据库 ProgramGraph 绑定这些能力。

```bash
cargo test -p az-edge-gateway
```
