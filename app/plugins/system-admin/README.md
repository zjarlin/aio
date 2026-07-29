# System Admin

Cargo 工件：`az-system-admin`

本插件解决 AIO 自身的用户、角色、组织、租户、字典、菜单、审计、认证会话、API Key 和客户端配置管理问题。它通过 `az-plugin-core` 注册 Provider、后端 API 和 PostgreSQL 模型。

它不包含 Studio、资产或设备领域功能，也不保存系统管理页面 renderer；页面和交互由数据库 ProgramGraph 定义。

```bash
cargo test -p az-system-admin
```
