use dioxus::prelude::*;

pub(super) fn render() -> Element {
    rsx! {
        studio::EndpointPage { page_id: "9606cfae-2c43-f884-2a4c-a0ad01b36b49".to_owned() }
    }
}
