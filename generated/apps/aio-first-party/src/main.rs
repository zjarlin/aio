#![forbid(unsafe_code)]

#[cfg(any(feature = "web", feature = "desktop"))]
use dioxus::prelude::*;

#[cfg(any(feature = "web", feature = "desktop"))]
mod pages;
#[cfg(any(
    all(feature = "web", feature = "desktop"),
    all(feature = "web", feature = "server"),
    all(feature = "desktop", feature = "server")
))]
compile_error!("web、desktop、server 每次只能启用一个目标");

#[cfg(not(any(feature = "web", feature = "desktop", feature = "server")))]
fn main() {
    eprintln!("请选择 --features web、desktop 或 server");
}

#[cfg(any(feature = "web", feature = "desktop"))]
fn main() {
    dioxus::launch(App);
}

#[cfg(any(feature = "web", feature = "desktop"))]
#[allow(non_snake_case)]
fn App() -> Element {
    rsx! {
        studio::PublishedApplication {
            render_convention: studio::ConventionPageRenderer::new(pages::render),
            admin_enabled: false,
            user: studio::ApplicationUser {
                label: "zjarlin".to_owned(),
                handle: "@zjarlin".to_owned(),
                initials: "ZJ".to_owned(),
            },
        }
    }
}

#[cfg(feature = "server")]
fn main() -> anyhow::Result<()> {
    az_aio_app::run_server_with(business::register)
}
