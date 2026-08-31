#![forbid(unsafe_code)]

#[cfg(target_arch = "wasm32")]
fn main() {
    az_aio_app::launch_application();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> anyhow::Result<()> {
    az_aio_app::run_server()
}
