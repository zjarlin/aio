# Browser Bridge SDK

`guest.js` 为隔离 iframe 提供二进制请求与 JSON 辅助接口。`host.mjs` 绑定单个窗口，不读取插件 DOM，不向插件暴露 Cookie 或管理票据。

此 SDK 使用 v2 消息；正式产品会话撤销、全屏和账户挂载仍在迁移中，不能把开发预览服务用作公网宿主。
