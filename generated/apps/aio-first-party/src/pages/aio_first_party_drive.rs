use dioxus::prelude::*;

pub(super) fn render() -> Element {
    rsx! {
        studio::EndpointPage { page_id: "a2b391ae-2c26-f31f-93a8-348c5dfc1341".to_owned() }
    }
}
