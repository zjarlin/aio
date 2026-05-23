mod agent_preferences;
mod app_paths;
mod asset_items;
mod asset_variables;
mod auth;
mod commands;
mod db;
mod dictionary;
mod error;
mod home_paths;
mod notes;
mod rbac;
mod skills;
mod state;
mod web_bridge;

use commands::{
    agent_preference_create, agent_preference_delete, agent_preference_page,
    agent_preference_toggle, agent_preference_update, app_open_data_dir, asset_item_create,
    asset_item_delete, asset_item_deploy_preview, asset_item_deploy_save,
    asset_item_import_directory, asset_item_page, asset_item_toggle, asset_item_update,
    asset_variable_delete, asset_variable_page, asset_variable_refresh_page_globals,
    asset_variable_upsert, auth_access_codes, auth_current_user, auth_login, auth_logout,
    dict_item_create, dict_item_delete, dict_item_page, dict_item_update, dict_type_create,
    dict_type_delete, dict_type_page, dict_type_update, menu_list, note_archive, note_create,
    note_delete, note_favorite, note_page, note_update, openai_assistant_chat,
    openai_assistant_preview_context, permission_save, permission_tree, role_assign_permissions,
    role_create, role_delete, role_page, role_permission_ids, role_update, skill_create,
    skill_delete, skill_page, skill_toggle, skill_update, user_create, user_delete, user_disable,
    user_page, user_reset_password, user_update,
};
use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let handle = app.handle().clone();
            tauri::async_runtime::block_on(async move {
                let state = AppState::new(&handle).await?;
                #[cfg(debug_assertions)]
                web_bridge::spawn(state.clone());
                handle.manage(state);
                Ok::<(), error::AppError>(())
            })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_open_data_dir,
            openai_assistant_preview_context,
            openai_assistant_chat,
            agent_preference_page,
            agent_preference_create,
            agent_preference_update,
            agent_preference_delete,
            agent_preference_toggle,
            auth_login,
            auth_logout,
            auth_current_user,
            auth_access_codes,
            menu_list,
            user_page,
            user_create,
            user_update,
            user_disable,
            user_reset_password,
            user_delete,
            role_page,
            role_create,
            role_update,
            role_delete,
            role_assign_permissions,
            role_permission_ids,
            permission_tree,
            permission_save,
            dict_type_page,
            dict_type_create,
            dict_type_update,
            dict_type_delete,
            dict_item_page,
            dict_item_create,
            dict_item_update,
            dict_item_delete,
            note_page,
            note_create,
            note_update,
            note_delete,
            note_archive,
            note_favorite,
            skill_page,
            skill_create,
            skill_update,
            skill_delete,
            skill_toggle,
            asset_item_page,
            asset_item_import_directory,
            asset_item_create,
            asset_item_update,
            asset_item_delete,
            asset_item_toggle,
            asset_item_deploy_preview,
            asset_item_deploy_save,
            asset_variable_page,
            asset_variable_upsert,
            asset_variable_delete,
            asset_variable_refresh_page_globals,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
