# 构建网络出口

此服务为隔离构建容器提供无凭据 HTTP 代理。上游连接凭据单独保存在服务器 `config.json`，由专用服务账户只读访问，不传入源码容器，也不提交到仓库。

实现固定为 [Mihomo v1.19.16](https://github.com/MetaCubeX/mihomo/releases/tag/v1.19.16) 的 Linux amd64 compatible 发行包。安装器验证官方发行资产的 SHA256，仅监听 Docker 默认网桥地址，限制来源到该网桥子网。

在 252 使用 `node install.cjs <官方发行包.gz> <最小代理配置.json>` 安装。输入配置应只包含所需代理、故障转移组和规则；安装器关闭 TUN、外部控制接口及自动地理数据库下载。服务通过 systemd 启动并限制资源，工作进程的 `AIO_BUILD_HTTP_PROXY` 指向 `http://172.17.0.1:17892`。

更新上游凭据时只替换服务器配置并重启 `aio-egress`。此出口不依赖开发机在线；上游网络不可用时，交付任务仍按已有策略退避，活动插件继续运行。
