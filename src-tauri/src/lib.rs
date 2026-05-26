mod agent_preferences;
mod app_paths;
mod app_runtime;
mod asset_items;
mod asset_variables;
mod auth;
mod capability_broker;
mod commands;
mod db;
mod dictionary;
pub mod error;
mod event_bus;
pub mod extension_host;
mod home_paths;
mod notes;
mod permission_approval;
mod permission_consent;
mod permission_core;
pub mod plugin_factory;
pub mod plugin_migration;
pub mod plugin_registry;
pub mod plugin_store;
mod rbac;
mod skills;
mod state;
pub mod web_bridge;

use commands::{
    agent_preference_create, agent_preference_delete, agent_preference_page,
    agent_preference_toggle, agent_preference_update, app_open_data_dir, app_runtime_reload,
    app_runtime_session, app_runtime_snapshot, app_runtime_start, app_runtime_stop,
    app_runtime_workspace, asset_item_create, asset_item_delete, asset_item_deploy_preview,
    asset_item_deploy_save, asset_item_import_directory, asset_item_page, asset_item_toggle,
    asset_item_update, asset_variable_delete, asset_variable_page,
    asset_variable_refresh_page_globals, asset_variable_upsert, auth_access_codes,
    auth_current_user, auth_login, auth_logout, capability_audit_log, capability_browser_open_url,
    capability_clipboard_read, capability_clipboard_write, capability_fs_read, capability_fs_write,
    capability_invoke, capability_notification_send, capability_process_exec, dict_item_create,
    dict_item_delete, dict_item_page, dict_item_update, dict_type_create, dict_type_delete,
    dict_type_page, dict_type_update, event_bus_publish, event_bus_snapshot, event_bus_stream,
    menu_list, note_archive, note_create, note_delete, note_favorite, note_page, note_update,
    openai_assistant_chat, openai_assistant_preview_context, permission_approval_approve,
    permission_approval_deny, permission_approval_list, permission_audit_log,
    permission_consent_grant, permission_consent_list, permission_consent_revoke, permission_save,
    permission_tree, plugin_child_capability_approve, plugin_child_capability_revoke,
    plugin_create_from_prompt, plugin_host_activate, plugin_host_deactivate, plugin_host_dispose,
    plugin_host_load, plugin_host_reload, plugin_host_snapshot, plugin_publish_gate,
    plugin_publish_local, plugin_registry_disable, plugin_registry_enable, plugin_registry_install,
    plugin_registry_local_state, plugin_registry_reload, plugin_registry_rollback,
    plugin_registry_snapshot, plugin_registry_uninstall, plugin_repair_from_diagnostics,
    plugin_verify_draft, role_assign_permissions, role_create, role_delete, role_page,
    role_permission_ids, role_update, skill_create, skill_delete, skill_page, skill_toggle,
    skill_update, user_create, user_delete, user_disable, user_page, user_reset_password,
    user_update,
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
            let state_handle = handle.clone();
            let state =
                tauri::async_runtime::block_on(async move { AppState::new(&state_handle).await })?;
            web_bridge::spawn(state.clone());
            handle.manage(state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_open_data_dir,
            app_runtime_snapshot,
            app_runtime_start,
            app_runtime_stop,
            app_runtime_reload,
            app_runtime_workspace,
            app_runtime_session,
            capability_audit_log,
            capability_browser_open_url,
            capability_clipboard_read,
            capability_clipboard_write,
            capability_fs_read,
            capability_fs_write,
            capability_invoke,
            capability_notification_send,
            capability_process_exec,
            event_bus_publish,
            event_bus_stream,
            event_bus_snapshot,
            permission_audit_log,
            permission_approval_list,
            permission_approval_approve,
            permission_approval_deny,
            permission_consent_grant,
            permission_consent_list,
            permission_consent_revoke,
            plugin_host_load,
            plugin_host_activate,
            plugin_host_deactivate,
            plugin_host_reload,
            plugin_host_dispose,
            plugin_host_snapshot,
            openai_assistant_preview_context,
            openai_assistant_chat,
            plugin_registry_reload,
            plugin_registry_local_state,
            plugin_child_capability_approve,
            plugin_child_capability_revoke,
            plugin_registry_install,
            plugin_registry_enable,
            plugin_registry_disable,
            plugin_registry_uninstall,
            plugin_registry_rollback,
            plugin_create_from_prompt,
            plugin_publish_gate,
            plugin_publish_local,
            plugin_repair_from_diagnostics,
            plugin_verify_draft,
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
            plugin_registry_snapshot,
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
