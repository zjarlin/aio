# AIO Plugins

这里按用户可识别的领域功能拆包。每个插件拥有自己的模型、服务、后端 API、Capability 和 README；业务页面与交互统一保存在 PostgreSQL `ProgramDefinition` 中，不在插件源码中注册页面 renderer。

`studio` 是零代码创作与执行插件，系统管理页面也由其 PostgreSQL 元数据直接驱动；其余目录分别解决资产、网盘、配置、边缘网关、物联网、Linux、软件、SSH 和算法领域问题。
