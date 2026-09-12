# 整包开发工具

`cargo run -p az-plugin-bundle --example package -- <仓库> <相对清单路径> <HTTPS Git> <完整 SHA> <SemVer> <输出文件>`。

此工具只打包和往返校验，不执行构建，不发布公网，也不改写产品 Cargo.toml。完整 CLI 正在迁移，此开发命令不是旧发布协议的适配入口。
