use dioxus::prelude::*;
use fullstack_model::{IncrementRequest, IncrementResponse};
use serde::Deserialize;

#[derive(Deserialize)]
struct Response {
    status: u16,
    body: String,
}

pub(super) async fn increment(request: IncrementRequest) -> Result<IncrementResponse, String> {
    let mut bridge = document::eval(
        r#"
        const request = await dioxus.recv();
        try { dioxus.send({ response: await window.aioPlugin.request(request) }); }
        catch (error) { dioxus.send({ error: error.message }); }
    "#,
    );
    bridge
        .send(serde_json::json!({
            "method": "POST", "path": "/counter/increment",
            "body": serde_json::to_string(&request).map_err(|error| error.to_string())?
        }))
        .map_err(|error| error.to_string())?;
    let value: serde_json::Value = bridge.recv().await.map_err(|error| error.to_string())?;
    if let Some(error) = value.get("error").and_then(serde_json::Value::as_str) {
        return Err(error.to_owned());
    }
    let response: Response =
        serde_json::from_value(value["response"].clone()).map_err(|error| error.to_string())?;
    if response.status != 200 {
        return Err(response.body);
    }
    serde_json::from_str(&response.body).map_err(|error| error.to_string())
}
