# 运行时插件契约

语言无关的 `aio:plugin@2.0.0` WIT。浏览器资源和服务端 Component 是同包中的不同产物。此 crate 不包含宿主、数据库驱动或 UI 框架；身份、租户、权限和业务实现均由插件提供。

WIT 的接口名是跨语言协议符号，来源 UUID 是安装身份，二者都不替代 Rust 进程内 Dill 的 TypeId。
