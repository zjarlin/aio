wit_bindgen::generate!({
    path: "../../../contract/wit",
    world: "plugin",
});

use aio::plugin::{
    host,
    metadata::{PageDefinition, Scene, Surface},
};

struct Fixture;

impl Guest for Fixture {
    fn describe() -> Description {
        Description {
            label: "Lifecycle fixture".into(),
            pages: vec![PageDefinition {
                id: "fixture".into(),
                label: "Fixture".into(),
                entry: "index.html".into(),
                scene: Some(Scene {
                    id: "test".into(),
                    label: "Test".into(),
                }),
                menu_path: vec![],
                permission: None,
                surface: Surface::Workspace,
            }],
        }
    }

    fn health() -> Result<(), String> {
        if cfg!(feature = "unhealthy") {
            Err("Unhealthy fixture".into())
        } else {
            Ok(())
        }
    }

    fn lifecycle(phase: Phase) -> Result<(), String> {
        let permission = match phase {
            Phase::Prepare => "prepare",
            Phase::Activate => "activate",
            Phase::Deactivate => "deactivate",
        };
        host::authorize(permission).map(|_| ())
    }

    fn handle(request: Request) -> Response {
        if request.path == "/environment" {
            return Response {
                status: 200,
                headers: vec![],
                body: std::env::vars().count().to_string().into_bytes(),
            };
        }
        if request.path == "/wait" {
            let _ = host::authorize("wait");
        }
        Response {
            status: 200,
            headers: vec![],
            body: request.body,
        }
    }
}

export!(Fixture);
