# v2 全栈整包

`az-plugin-bundle` 只处理 `aio:plugin@2` 的已构建 Component 整包，不解释旧页面协议，不执行构建或安装脚本。使用标准 gzip 压缩 JSON；前端资源、后端 Component、数据库迁移、来源完整提交与版本共同参与 SHA256。摘要标识内容，不是作者签名或授权。

仓库输入清单为 `schema_version = 2`，包含 `plugin.runtime.artifact`、`host_version`、`plugin.frontend.path`，可选数据库迁移目录及能力申请。运行目标由 Component 产物校验，不需要使用者填写 `runtime.kind`。JAR 不是 Component，进程包将在独立监督器迁移时接入，不能塞进这个入口。

解码限制压缩及展开大小、文件数量和路径；拒绝缺失产物、未知文件、核心 Wasm、尾随数据、未锁定 SHA 和内容篡改。`VerifiedBundle` 只提供只读资产访问；完整 WIT 类型与入口存在性由运行时实例化和 describe 阶段校验。源码与本地构建目录不属于安装包；构建时读取目录也拒绝符号链接，迁移目录只收集直接包含的 `.sql` 文件，不打包 README。

`plugin.marketplace` 声明标题、简介、许可证、标签和可选的 `parent` 仓库地址。父子归属参与整包摘要，不能引用自身；发布者和宿主共同校验关系。`plugin.permissions` 列出插件权限，禁止通配授权。安装父插件不意味着安装子插件，子插件只能在同一租户已启用父插件后安装。

可选 `parent_title` 为尚未发布的父插件提供显示名称，必须同时声明 `parent`。父插件发布后使用父包的正式标题，仓库地址继续作为安装关系标识。

产品通过原生 Component 发布入口接入持久注册表，源码摘要不替代发布身份校验。各产品上线状态见 `docs/refactor/README.md`。

`VerifiedBundle::frontend_files` 只枚举经过整包校验的前端相对路径与只读字节，用于宿主生成资源摘要和预热缓存；后端产物与数据库迁移不进入该清单。资源访问仍由宿主按当前会话和安装版本鉴权。
