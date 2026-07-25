#![allow(non_snake_case)]

//! nature-compiler 母语工作台页面。

use az_aio_platform::plugin::contract::NativeRenderContext;
use dioxus::prelude::*;
use registry::ui::{
    button::Button,
    card::{Card, CardContent, CardHeader, CardTitle},
};

use crate::contract::UI_ACTION_PATH;

pub fn NatureCompilerPage(context: NativeRenderContext) -> Element {
    let revision = query_value(&context.active_route, "revision");
    let error = query_value(&context.active_route, "error");
    rsx! {
        div { class: "space-y-5",
            if let Some(revision_id) = revision {
                div { class: "border-l-4 border-green-600 bg-green-600/10 px-4 py-3 text-sm",
                    "Revision 已进入生成队列："
                    code { "{revision_id}" }
                }
            }
            if let Some(message) = error {
                div { class: "border-l-4 border-destructive bg-destructive/10 px-4 py-3 text-sm text-destructive",
                    "{message}"
                }
            }
            Card {
                CardHeader { CardTitle { "nature-compiler" } }
                CardContent {
                    form { method: "post", action: UI_ACTION_PATH, class: "grid gap-4",
                        label { class: "grid gap-2 text-sm",
                            span { class: "font-medium", "项目" }
                            input {
                                class: "aio-input",
                                name: "project_id",
                                value: "环境采集",
                                required: true,
                            }
                        }
                        label { class: "grid gap-2 text-sm",
                            span { class: "font-medium", "母语需求与建模" }
                            textarea {
                                class: "aio-input min-h-96 font-mono",
                                name: "source_text",
                                required: true,
                                value: default_source(),
                            }
                        }
                        div { class: "flex justify-end",
                            Button { button_type: "submit", "生成 Revision" }
                        }
                    }
                }
            }
        }
    }
}

fn default_source() -> &'static str {
    include_str!("../../../crates/generated/nature/blueprint-source.txt")
}

fn query_value(route: &str, key: &str) -> Option<String> {
    let query = route.split_once('?')?.1;
    for pair in query.split('&') {
        let (pair_key, pair_value) = pair.split_once('=').unwrap_or((pair, ""));
        if pair_key == key {
            return Some(
                urlencoding::decode(pair_value)
                    .map(|value| value.into_owned())
                    .unwrap_or_else(|_| pair_value.to_string()),
            );
        }
    }
    None
}
