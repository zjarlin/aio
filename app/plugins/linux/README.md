# Linux

Cargo 工件：`az-linux`

本插件解决 Linux 主机接入规划、发行版识别、环境说明和引导脚本生成问题，暴露 `/api/linux/*` 类型化接口。首个明确支持的目标是 Ubuntu。

它不建立 SSH 连接、不持久化密钥，也不保存业务页面。真实远程执行归属 SSH 插件，Linux 管理页面归属数据库 ProgramGraph。

```bash
cargo test -p az-linux
```
