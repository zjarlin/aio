mod backend_demo;
mod counter;
mod transport;

use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    #[cfg(feature = "bundle-assets")]
    let stylesheets = rsx! { az_ui_components::UiStylesheets {} };
    #[cfg(not(feature = "bundle-assets"))]
    let stylesheets = rsx! {};
    rsx! {
        {stylesheets}
        main { class: "p-4 space-y-6",
            counter::Counter {}
            backend_demo::BackendDemo {}
        }
    }
}
