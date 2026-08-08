#![forbid(unsafe_code)]

#[cfg(not(target_arch = "wasm32"))]
mod config;
#[cfg(not(target_arch = "wasm32"))]
mod contracts;
#[cfg(not(target_arch = "wasm32"))]
mod migration;
#[cfg(target_arch = "wasm32")]
mod pages;
#[cfg(not(target_arch = "wasm32"))]
mod plugin_host;
#[cfg(not(target_arch = "wasm32"))]
mod server;

#[cfg(target_arch = "wasm32")]
fn main() {
    dioxus::launch(studio::App);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> anyhow::Result<()> {
    server::run()
}
