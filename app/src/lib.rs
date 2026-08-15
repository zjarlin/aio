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
mod migration;
#[cfg(not(target_arch = "wasm32"))]
mod server;

#[cfg(target_arch = "wasm32")]
pub fn launch_studio() {
    dioxus::launch(admin_shell::App);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_server() -> anyhow::Result<()> {
    server::run(business::register)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_server_with(register_business: fn(&mut dill::CatalogBuilder)) -> anyhow::Result<()> {
    server::run(register_business)
}
