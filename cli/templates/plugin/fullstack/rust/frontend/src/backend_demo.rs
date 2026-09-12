use az_ui_components::button::{Button, ButtonVariant};
use dioxus::prelude::*;
use fullstack_model::IncrementRequest;

#[component]
pub(super) fn BackendDemo() -> Element {
    let mut result = use_signal(|| 0_i64);
    let mut tenant = use_signal(|| None::<String>);
    let mut pending = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    rsx! {
        section {
            h2 { "后端计算" }
            p { "服务端结果：{result}" }
            if let Some(tenant) = tenant() { p { "租户：{tenant}" } }
            Button {
                r#type: "button",
                variant: ButtonVariant::Outline,
                disabled: pending(),
                onclick: move |_| {
                    let request = IncrementRequest { count: result() };
                    pending.set(true);
                    error.set(None);
                    spawn(async move {
                        match super::transport::increment(request).await {
                            Ok(response) => {
                                result.set(response.count);
                                tenant.set(Some(response.tenant_id));
                            }
                            Err(message) => error.set(Some(message)),
                        }
                        pending.set(false);
                    });
                },
                "请求后端 +1"
            }
            if let Some(message) = error() { p { role: "alert", "{message}" } }
        }
    }
}
