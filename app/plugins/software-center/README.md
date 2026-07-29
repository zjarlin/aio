# Software Center

Cargo 工件：`az-software-center`

本插件解决安装包扫描、名称匹配、归档整理和软件目录记录问题，拥有 `biz_software_center_*` PostgreSQL 模型以及 `/api/software-center/*` 后端接口。

它不执行任意脚本，也不保存业务页面；界面和交互由数据库 ProgramGraph 定义。

```bash
cargo test -p az-software-center
```
