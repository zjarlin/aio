# Config Center

Cargo 工件：`az-config-center`

本插件解决机器配置项、dotfiles 监测和设备配对问题，拥有 `biz_config_center_*` PostgreSQL 模型以及 `/api/config-center/*` 后端接口。

它不负责应用启动配置，也不保存业务页面。应用启动配置归属 `app/src/config.rs`，配置中心页面归属数据库 ProgramGraph。

```bash
cargo test -p az-config-center
```
