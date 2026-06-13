use tauri::State;

use crate::{
    app_state::AppState,
    profile::{VpnProfile, VpnStatus},
};

type CommandResult<T> = Result<T, String>;

#[tauri::command]
pub async fn list_profiles(state: State<'_, AppState>) -> Vec<VpnProfile> {
    state.list_profiles().await
}

#[tauri::command]
pub async fn save_profile(
    state: State<'_, AppState>,
    profile: VpnProfile,
) -> CommandResult<VpnProfile> {
    state.save_profile(profile).await.map_err(redact_error)
}

#[tauri::command]
pub async fn delete_profile(state: State<'_, AppState>, id: String) -> CommandResult<()> {
    state.delete_profile(&id).await.map_err(redact_error)
}

#[tauri::command]
pub async fn connect(state: State<'_, AppState>, profile_id: String) -> CommandResult<()> {
    state.connect(&profile_id).await.map_err(redact_error)
}

#[tauri::command]
pub async fn disconnect(state: State<'_, AppState>) -> CommandResult<()> {
    state.disconnect().await.map_err(redact_error)
}

#[tauri::command]
pub async fn status(state: State<'_, AppState>) -> VpnStatus {
    state.status().await
}

#[tauri::command]
pub async fn recent_logs(state: State<'_, AppState>) -> Vec<String> {
    state.recent_logs().await
}

fn redact_error(error: anyhow::Error) -> String {
    error.to_string()
}
