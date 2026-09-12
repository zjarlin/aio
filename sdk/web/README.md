# Browser Bridge SDK

`guest.js` 为隔离 iframe 提供二进制请求与 JSON 辅助接口。`host.mjs` 绑定单个窗口，不读取插件 DOM，不向插件暴露 Cookie 或管理票据。

`aioPlugin.copy(text)` 请求宿主复制文本，要求挂载时显式授予 `{clipboard: true}`、页面具有焦点及当前用户手势。文本有长度上限，拒绝或超时返回错误；不会把剪贴板内容转发给插件服务或模型。该授权不提供读取剪贴板能力。

此 SDK 使用 v2 消息；正式产品会话撤销、全屏和账户挂载仍在迁移中，不能把开发预览服务用作公网宿主。
