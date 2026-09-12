mod counter;

use az_plugin_manifest::ComponentResponse;

wit_bindgen::generate!({ path: "wit", world: "page" });

struct Component;

impl Guest for Component {
    fn definition() -> String {
        serde_json::json!([{
            "id": "dioxus-fullstack-counter",
            "label": "Dioxus 全栈计数器",
            "scene": { "id": "community", "label": "社区插件" },
            "menu_path": [],
            "body": { "kind": "frontend", "entry": "index.html" }
        }])
        .to_string()
    }

    fn handle(request: String) -> String {
        let response = match counter::handle(&request) {
            Ok(body) => ComponentResponse {
                status: 200,
                content_type: "application/json".to_owned(),
                body,
            },
            Err(error) => ComponentResponse {
                status: 400,
                content_type: "application/json".to_owned(),
                body: serde_json::json!({ "error": error.to_string() }).to_string(),
            },
        };
        serde_json::to_string(&response).expect("固定响应模型可序列化")
    }
}

export!(Component);
