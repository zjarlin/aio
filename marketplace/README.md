# AIO 插件市场

市场是 Git 驱动的静态发现目录。插件作者在 `registry/` 增加一个 TOML；合并即发布，是否要求审核由分支保护配置决定。租户也可以绕过市场直接配置任意 Git 仓库。

市场元数据只用于展示与检索，不参与 Dill 或 Wasm 运行时身份。安装器必须重新读取目标仓库的 `aio-plugin.toml` 并锁定提交 SHA。

`aio marketplace build` 校验全部条目并原子生成 `index.json`。Pull Request 和 main push 都会运行同一校验；合并策略决定自动收录还是人工审核，协议本身不绑定审核模式。
