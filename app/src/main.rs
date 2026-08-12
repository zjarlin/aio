#![forbid(unsafe_code)]

#[cfg(target_arch = "wasm32")]
mod admin_shell;
#[cfg(not(target_arch = "wasm32"))]
mod application_starters;
#[cfg(not(target_arch = "wasm32"))]
mod application_startup;
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
    admin_shell::enable();
    az_dioxus_admin_extension_crud::enable();
    studio::enable();
    dioxus::launch(admin_shell::App);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> anyhow::Result<()> {
    server::run()
}
