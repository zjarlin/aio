# Git 全栈插件

以应用仓库的 `aio.toml` 为真源，负责 Git checkout、client/server 清单发现、Cargo feature 同步、两端 Dill 注册源码生成和卸载清理。`aio plugin validate [<仓库目录>]` 使用与运行时共享的无头清单契约，校验多语言 artifact、子插件依赖和 Component ABI；`aio plugin schema [<输出目录>]` 为其他语言输出同源 JSON Schema。

`aio plugin publish [<仓库目录>]` 只发布带市场元数据的在线 artifact。它要求清单和 artifact 与当前完整 Git SHA 中的 blob 完全一致，通过来源绑定凭证 gzip 上传，再轮询宿主后台任务到激活或失败。
