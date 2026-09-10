# 二进制插件包

`az-plugin-package` 是 CLI 和插件中心共用的 `.aio-plugin` 契约。包是确定性 gzip 压缩的 UTF-8 JSON，包含发布来源、SemVer 版本、可选源码提交、清单和一个二进制产物。一个功能的页面、服务和子插件统一随该产物发布。

`PluginPackage::new` 创建包，`encode` 输出传输字节，`decode` 有界解压并验证，`verify` 返回已校验的清单和产物字节。artifact 上限 32 MiB，清单 128 KiB，压缩包及解压 JSON 各 48 MiB；base64 在解码前检查长度。压缩时间戳固定为零，不携带本地文件名。

`rev` 是域分隔的 SHA-256 内容摘要，覆盖格式、规范化 Git 来源、版本、可选源码提交、完整清单和 artifact SHA-256。Git 来源只用于发布者归属与审计，不证明源码和构建产物一致，也不要求有本地 Git 仓库。旧 GitProof JSON 请求不属于此协议。

HTTP 发布使用 `Content-Type: application/vnd.aio.plugin+gzip`，正文直接发送 `.aio-plugin` 字节，不设置 `Content-Encoding`。包的完整性校验不替代宿主的来源鉴权、能力授权、运行实例健康检查与激活事务。
