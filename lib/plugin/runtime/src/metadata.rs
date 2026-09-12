use crate::bindings::aio::plugin::metadata::{Description, Surface};
use anyhow::{Result, ensure};
use std::collections::{HashMap, HashSet};

pub(crate) fn validate(description: &Description) -> Result<()> {
    ensure!(
        !description.label.trim().is_empty() && description.label.len() <= 256,
        "插件标题无效"
    );
    ensure!(description.pages.len() <= 128, "页面数量超过配额");
    let mut pages = HashSet::new();
    let mut scenes = HashMap::new();
    for page in &description.pages {
        ensure!(
            !page.id.is_empty() && page.id.len() <= 128 && pages.insert(&page.id),
            "页面 ID 为空、过长或重复"
        );
        ensure!(
            !page.label.trim().is_empty() && page.label.len() <= 256,
            "页面标题无效"
        );
        az_plugin_bundle::validate_relative_path(&page.entry)?;
        ensure!(
            page.menu_path.len() <= 8
                && page
                    .menu_path
                    .iter()
                    .all(|part| !part.trim().is_empty() && part.len() <= 256),
            "菜单路径无效"
        );
        if let Some(permission) = &page.permission {
            ensure!(
                !permission.trim().is_empty() && permission.len() <= 256,
                "页面权限无效"
            );
        }
        if let Some(scene) = &page.scene {
            ensure!(
                !scene.id.trim().is_empty()
                    && scene.id.len() <= 128
                    && !scene.label.trim().is_empty()
                    && scene.label.len() <= 256,
                "场景根无效"
            );
            if let Some(label) = scenes.insert(&scene.id, &scene.label) {
                ensure!(label == &scene.label, "同一场景根标题冲突");
            }
        }
        ensure!(
            page.surface != Surface::Workspace || page.scene.is_some(),
            "工作区页面必须声明场景根"
        );
        ensure!(
            page.surface == Surface::Workspace || page.menu_path.is_empty(),
            "独立挂载页面不应进入工作区菜单树"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::aio::plugin::metadata::{PageDefinition, Scene};

    fn description() -> Description {
        Description {
            label: "Example".into(),
            pages: vec![PageDefinition {
                id: "counter".into(),
                label: "Counter".into(),
                entry: "frontend/index.html".into(),
                scene: Some(Scene {
                    id: "workspace".into(),
                    label: "Workspace".into(),
                }),
                menu_path: vec![],
                permission: None,
                surface: Surface::Workspace,
            }],
        }
    }

    #[test]
    fn rejects_duplicate_pages_external_entries_and_wrong_surfaces() {
        assert!(validate(&description()).is_ok());
        let mut duplicate = description();
        duplicate.pages.extend(description().pages);
        assert!(validate(&duplicate).is_err());
        for entry in [
            "../index.html",
            "/index.html",
            "https://outside/index.html",
            "folder\\index.html",
            "index.html?token=x",
        ] {
            let mut definition = description();
            definition.pages[0].entry = entry.into();
            assert!(validate(&definition).is_err());
        }
        let mut fullscreen = description();
        fullscreen.pages[0].surface = Surface::Fullscreen;
        fullscreen.pages[0].menu_path.push("Settings".into());
        assert!(validate(&fullscreen).is_err());
    }
}
