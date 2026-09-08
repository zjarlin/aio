# Git 全栈插件

以应用仓库的 `aio.toml` 为真源，负责 Git checkout、client/server 清单发现、Cargo feature 同步、两端 Dill 注册源码生成和卸载清理。`aio plugin validate [<仓库目录>]` 使用与运行时共享的无头清单契约，校验多语言 artifact、子插件依赖和 Component ABI。
