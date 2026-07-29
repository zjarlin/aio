# Drive Center

Cargo 工件：`az-drive-center`

本插件解决网盘任务、托管队列和任务状态查询问题，拥有 `biz_drive_center_*` PostgreSQL 模型以及 `/api/drive-center/*` 后端接口。

它不实现通用对象存储，也不保存页面 renderer。网盘页面由数据库 ProgramGraph 组合本插件能力。

```bash
cargo test -p az-drive-center
```
