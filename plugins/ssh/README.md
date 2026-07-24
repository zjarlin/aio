# SSH 服务器运维插件

该插件为 AIO 提供“服务器运维”上下文，通过 `az-ssh` 直接连接真实主机地址，不依赖运行机器上的 SSH Host 别名。

## 低代码模型

- `ssh_target`：保存主机、端口、用户、认证方式和密钥路径等连接配置。
- `ssh_command`：保存硬件探测脚本、监测命令、超时和排序，可在低代码工作台直接调整。
- `ssh_command_result`：保存每个目标和命令的最近一次执行结果。

密码和私钥口令不会写入模型。密码认证使用 `password_env` 指向运行 AIO 进程的环境变量；私钥口令使用 `passphrase_env`。

## 内置硬件探测

默认命令覆盖通用 Linux、海光 DCU/HCU、NVIDIA GPU、AMD ROCm、Intel XPU、IPMI、lm-sensors、SMART、NVMe、网络、systemd 和容器运行时。每条命令先执行 `detect_script`，不匹配当前硬件或软件环境时标记为“不支持”，不会阻断其他命令。
