use dioxus::prelude::*;

pub(super) fn render() -> Element {
    rsx! {
        studio::EndpointPage { page_id: "545103ac-56a3-5c43-a15c-d1db7263afae".to_owned() }
    }
}
