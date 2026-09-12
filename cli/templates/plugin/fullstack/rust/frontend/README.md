# 前端 Dioxus 页面

独立编译的真实 Dioxus Web 产物。只使用共享 az-ui-components，后端请求通过宿主注入的受控 aioPlugin 桥发送。该产物由 Web 或 Desktop 宿主的隔离 WebView 承载；不能把裸浏览器直接打开产物当成完整全栈运行。

`counter.rs` 是纯本地 `Signal` 与 `rsx!`，不依赖通信桥、网络和后端。`backend_demo.rs` 单独演示显式请求，`transport.rs` 只负责服务调用，不接管 UI 事件或状态。

构建先由 Dioxus CLI 收集共享组件样式，再由匹配 Rust 依赖版本的 `wasm-bindgen 0.2.128` 生成相对模块和 Wasm 地址。产物不假定宿主部署根路径，也不改写编译后的 JavaScript。
