# Algorithm Center

Cargo 工件：`az-algorithm-center`

本插件解决算法组件发现、目录查询、输入上传和处理调用问题，直接复用 `az-algorithm` 的算法目录并暴露类型化 Axum API。

它不保存业务页面、不实现 Studio 组件，也不负责通用文件存储。算法中心页面由数据库中的 ProgramGraph 绑定这些 API 或 Capability。

```bash
cargo test -p az-algorithm-center
```
