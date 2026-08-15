use dioxus::prelude::*;

pub(super) fn render() -> Element {
    rsx! {
        studio::EndpointPage { page_id: "c080eb9d-00ee-3bcb-7219-4f2b3a5ed84d".to_owned() }
    }
}
