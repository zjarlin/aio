# IoT Center

Cargo 工件：`az-iot-center`

本插件解决产品、网关、设备、遥测和告警管理问题，使用 `az-plugin-core` 的动态模型与 JSONB 记录保存正式数据，并暴露 `/api/iot/*` 接口。

设备状态分别评估连接、心跳和业务数据新鲜度，输出 Online、HeartbeatLost、DataAnomaly、Offline 或 Unknown。插件不负责 MQTT 通用客户端，也不保存页面 renderer；物联网页面由数据库 ProgramGraph 定义。

```bash
cargo test -p az-iot-center
```
