use az_ui_components::button::{Button, ButtonVariant};
use dioxus::prelude::*;

#[component]
pub(super) fn Counter() -> Element {
    let mut count = use_signal(|| 0_i64);
    rsx! {
        section {
            h2 { "Dioxus 全栈计数器" }
            p { "计数：{count}" }
            Button {
                r#type: "button",
                variant: ButtonVariant::Outline,
                onclick: move |_| count += 1,
                "+1"
            }
        }
    }
}
