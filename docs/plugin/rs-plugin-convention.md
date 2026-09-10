# Rust 插件开发规约

## 初始化

```bash
cargo install --path cli
aio plugin init ../aio-plugin-hello --name aio-plugin-hello --title "Hello" --language rust
```

当前 Rust/Dioxus 初始化使用源码装配，CLI 不接受 `--runtime` 选择。同一个 Git 仓库内的 `client`、`server` crate 分别拥有前端与后端，可以增加 `shared` crate 保存 DTO 和纯业务逻辑；它们不是两个 Git 仓库。只需要一端时可删除另一目录和对应清单段，不要建立空转发 crate。

## 页面

页面插件实现 `az_dioxus_admin_shell::ApplicationPlugin`，并在 `register` 中把具体实现绑定到 Dill。宿主聚合该 trait 的全部实现并按具体类型的 `TypeId` 校验唯一性，不需要字符串插件身份或 artifact 运行目标；页面 `id` 只用于导航。交互控件使用 `az-ui-components`，样式由宿主提供，插件不得携带 CSS、Stylesheet 或 inline style。

```rust
pub fn register(builder: &mut dill::CatalogBuilder) {
    builder
        .add_value(HelloPages)
        .bind::<dyn ApplicationPlugin, HelloPages>();
}
```

## 服务

Service、Controller 分开并由 Dill 创建。`register` 只注册具体类型，`router` 只把已解析 Controller 转换成 Axum Router。业务错误使用 `anyhow::Result`，关键边界补充 `Context`。

服务端依赖不得通过 shared crate 进入浏览器。Web 使用当前宿主的同源 API，Desktop 必须通过可配置后端地址建立连接，不能把浏览器专用请求实现作为跨平台 API。生成的主应用具有 Web、Desktop、Server 三个互斥目标；源码扩展随应用整体编译，不能作为单插件在线热替换的证据。

## 二进制边界

直接发布的包必须包含可独立运行的 PageDefinition、Wasm Component 或 process artifact，不能上传尚未编译的 `client/server` crate。Rust 编写的 Component 同样实现 [WIT 协议](wit/page.wit)，使用统一的 [二进制发布流程](publish.md)。[联合包契约](frontend-bundle.md) 已能将 Dioxus 编译输出和后端装入同一个有界、可校验的包；自定义前端的隔离挂载和后端调用桥尚未完成，不能将当前源码模板描述为这一能力。

## 验证

```bash
cargo fmt --all --check
cargo test --workspace
aio plugin validate /path/to/plugin

aio init /tmp/aio-host --name aio-host --title "AIO Host"
cd /tmp/aio-host
aio plugin install /path/to/plugin
cargo check --target wasm32-unknown-unknown --no-default-features --features web
cargo check --no-default-features --features server
```

提交市场前必须提交 `Cargo.lock`、`aio-plugin.toml` 和清晰的许可证；不得在安装阶段下载或执行额外代码。
