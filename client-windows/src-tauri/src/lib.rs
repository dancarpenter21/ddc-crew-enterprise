mod app_state;
mod commands;
mod profile;
mod storage;
mod tunnel;

use app_state::AppState;

pub fn run() {
    tauri::Builder::default()
        .manage(AppState::load())
        .invoke_handler(tauri::generate_handler![
            commands::list_profiles,
            commands::save_profile,
            commands::delete_profile,
            commands::connect,
            commands::disconnect,
            commands::status,
            commands::recent_logs,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run DDC VPN client");
}
