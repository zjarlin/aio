# SSH

Cargo 工件：`az-ssh-plugin`

本插件解决远程主机、命令模板、硬件探测和执行结果管理问题，通过 `az-ssh` 连接真实地址，并使用 `az-plugin-core` 的动态记录保存目标、命令和结果。

密码和私钥口令不写入业务记录，只能通过受控 Secret 环境变量读取。插件向 Studio 注册类型化 SSH Capability，但不允许 ProgramGraph 直接执行任意 Rust、Shell 路径或未声明操作，也不保存页面 renderer。

```bash
cargo test -p az-ssh-plugin
```
