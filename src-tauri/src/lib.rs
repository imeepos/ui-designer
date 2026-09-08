//! Rudder desktop shell (Tauri 2).
//!
//! Thin shell only: business logic lives in `rudder-core`; commands wrap it
//! and normalize errors to `{code, message}` (docs/ARCHITECTURE.md §7).

mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::ping])
        .run(tauri::generate_context!())
        .expect("error while running rudder desktop shell");
}
