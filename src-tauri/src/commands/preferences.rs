use crate::app_state::AppState;
use crate::preferences::AppPreferences;
use std::path::Path;
use tauri::State;
use tauri_plugin_notification::NotificationExt;

/// 获取当前偏好
#[tauri::command]
pub async fn get_preferences(state: State<'_, AppState>) -> Result<AppPreferences, String> {
    Ok(state.preferences())
}

/// 设置偏好（验证后落盘）
#[tauri::command]
pub async fn set_preferences(
    state: State<'_, AppState>,
    preferences: AppPreferences,
) -> Result<AppPreferences, String> {
    preferences.validate()?;
    let store = state.preferences_store();
    store.save(&preferences)?;
    state.set_preferences(preferences.clone());
    Ok(preferences)
}

/// 导出偏好到指定路径
#[tauri::command]
pub async fn export_preferences(
    state: State<'_, AppState>,
    output_path: String,
) -> Result<AppPreferences, String> {
    let prefs = state.preferences();
    let store = state.preferences_store();
    store.export_to(&prefs, Path::new(&output_path))?;
    Ok(prefs)
}

/// 从指定路径导入偏好
#[tauri::command]
pub async fn import_preferences(
    state: State<'_, AppState>,
    input_path: String,
) -> Result<AppPreferences, String> {
    let store = state.preferences_store();
    let imported = store.import_from(Path::new(&input_path))?;
    store.save(&imported)?;
    state.set_preferences(imported.clone());
    Ok(imported)
}

// 以下两个保留（系统状态，非偏好）
#[tauri::command]
pub fn get_notification_permission_state(state: State<'_, AppState>) -> Result<String, String> {
    let app = state.player.app_handle();
    let permission = app
        .notification()
        .permission_state()
        .map_err(|e| format!("{e}"))?;
    Ok(match permission {
        tauri::plugin::PermissionState::Granted => "granted",
        tauri::plugin::PermissionState::Denied => "denied",
        tauri::plugin::PermissionState::Prompt => "prompt",
        tauri::plugin::PermissionState::PromptWithRationale => "prompt-with-rationale",
    }
    .to_string())
}

#[tauri::command]
pub fn send_test_notification(state: State<'_, AppState>) -> Result<(), String> {
    let app = state.player.app_handle();
    crate::notification::notify_test(app)
}
