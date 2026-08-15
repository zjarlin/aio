use dioxus::prelude::*;

pub(super) fn render() -> Element {
    rsx! {
        studio::EndpointPage { page_id: "d379940c-a697-5f35-7673-c1565c252c2d".to_owned() }
    }
}
