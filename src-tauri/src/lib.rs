//! Rudder desktop shell (Tauri 2).
//!
//! Thin shell only: business logic lives in `rudder-core::ops`; commands wrap
//! it, normalize errors to `{code, message, hint}` and emit job progress
//! events (docs/ARCHITECTURE.md §7).

mod commands;
mod error;
mod projects;
mod view;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::create_project,
            commands::list_projects,
            commands::get_project,
            commands::get_lineage,
            commands::generate_board,
            commands::update_board_brief,
            commands::pick_anchor,
            commands::add_page,
            commands::generate_page,
            commands::pick_page,
            commands::update_page,
            commands::add_component,
            commands::generate_component,
            commands::pick_component,
            commands::update_component,
            commands::record_generated_image,
            commands::get_cms_api_key,
            commands::get_generation_config,
            commands::export_project,
            commands::delete_artifact,
            commands::get_credential_status,
            commands::save_api_key,
            commands::clear_api_key,
            commands::save_base_url,
            commands::test_connection,
            commands::auth_login,
            commands::auth_register,
            commands::auth_me,
            commands::auth_status,
            commands::auth_logout,
        ])
        .run(tauri::generate_context!())
        .expect("error while running rudder desktop shell");
}
