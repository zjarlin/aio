# Rust 插件开发规约

## 初始化

```bash
cargo install --path cli
aio plugin init ../aio-plugin-hello --name aio-plugin-hello --title "Hello"
```

仓库由 `client`、`server` 两个 crate 组成。只需要一端时可删除另一目录和对应清单段；不要建立空转发 crate。

## 页面

页面插件实现 `az_dioxus_admin_shell::ApplicationPlugin`，并在 `register` 中绑定到 Dill。具体类型是唯一运行时身份；页面 `id` 只用于导航，不作为插件身份。交互控件使用 `az-ui-components`，样式由宿主提供，插件不得携带 CSS、Stylesheet 或 inline style。

```rust
pub fn register(builder: &mut dill::CatalogBuilder) {
    builder
        .add_value(HelloPages)
        .bind::<dyn ApplicationPlugin, HelloPages>();
}
```

## 服务

Service、Controller 分开并由 Dill 创建。`register` 只注册具体类型，`router` 只把已解析 Controller 转换成 Axum Router。业务错误使用 `anyhow::Result`，关键边界补充 `Context`。

## 验证

```bash
cargo fmt --all --check
cargo test --workspace

aio init /tmp/aio-host --name aio-host --title "AIO Host"
cd /tmp/aio-host
aio plugin install /path/to/plugin
cargo check --target wasm32-unknown-unknown --no-default-features --features web
cargo check --no-default-features --features server
```

提交市场前必须提交 `Cargo.lock`、`aio-plugin.toml` 和清晰的许可证；不得在安装阶段下载或执行额外代码。
