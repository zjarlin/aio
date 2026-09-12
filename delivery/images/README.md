# 固定工具链镜像

镜像在部署时构建一次，工作进程通过内容摘要使用。基础镜像通过 BASE_IMAGE 显式传入固定摘要，源码构建没有凭证和宿主 socket。

Rust 镜像预装 Dioxus CLI 0.7.9、wasm-bindgen 0.2.128、Binaryen 127 和 esbuild 0.27.3。构建时禁用 Dioxus 自动下载工具，避免隐式工具链变化；新增下载均验证固定摘要。
