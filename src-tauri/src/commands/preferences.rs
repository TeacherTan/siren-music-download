use crate::app_state::{AppState, NotificationPreferences};
use tauri::State;
use tauri_plugin_notification::NotificationExt;

#[tauri::command]
pub fn get_notification_preferences(
    state: State<'_, AppState>,
) -> Result<NotificationPreferences, String> {
    Ok(state.notification_preferences())
}

#[tauri::command]
pub fn set_notification_preferences(
    state: State<'_, AppState>,
    preferences: NotificationPreferences,
) -> Result<NotificationPreferences, String> {
    state.set_notification_preferences(preferences.clone());
    Ok(preferences)
}

#[tauri::command]
pub fn get_notification_permission_state(
    state: State<'_, AppState>,
) -> Result<String, String> {
    let app = state.player.app_handle();
    let permission = app
        .notification()
        .permission_state()
        .map_err(|error| error.to_string())?;

    Ok(match permission {
        tauri::plugin::PermissionState::Granted => "granted",
        tauri::plugin::PermissionState::Denied => "denied",
        tauri::plugin::PermissionState::Prompt => "prompt",
        tauri::plugin::PermissionState::PromptWithRationale => "prompt-with-rationale",
    }
    .to_string())
}

#[tauri::command]
pub fn send_test_notification(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let app = state.player.app_handle();
    crate::notification::notify_test(app)
}
