#!/bin/zsh
set -euo pipefail
cd "${0:A:h}"
cargo test --workspace
cargo build -p fullstack-backend --target wasm32-unknown-unknown --release
rm -rf dist
mkdir -p dist/web/assets
wasm-tools component new target/wasm32-unknown-unknown/release/fullstack_backend.wasm -o dist/backend.wasm

# dx 只负责解析并收集共享组件样式，正式启动模块由 wasm-bindgen 生成相对地址。
dx build --package fullstack-frontend --web --release --features bundle-assets --inject-loading-scripts false --debug-symbols false
cargo build -p fullstack-frontend --target wasm32-unknown-unknown --release
wasm-bindgen --target web --no-typescript --out-name frontend --out-dir dist/web/assets target/wasm32-unknown-unknown/release/fullstack-frontend.wasm
find target/dx/fullstack-frontend/release/web/public/assets -type f -name '*.css' -print0 | sort -z | xargs -0 sh -c 'cat "$@"' _ > dist/web/assets/shared.css
cp frontend/index.html frontend/bootstrap.js dist/web/
