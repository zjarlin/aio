use dioxus::prelude::*;

pub(super) fn render() -> Element {
    rsx! {
        studio::EndpointPage { page_id: "f355b92d-cda9-b63c-06d7-4e44f0c4735c".to_owned() }
    }
}
