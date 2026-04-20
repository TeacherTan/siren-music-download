use crate::app_state::AppState;
use crate::logging::{LogFileStatus, LogViewerPage, LogViewerQuery};
use tauri::State;

#[tauri::command]
pub fn list_log_records(
    state: State<'_, AppState>,
    query: LogViewerQuery,
) -> Result<LogViewerPage, String> {
    state.log_center.list_records(query)
}

#[tauri::command]
pub fn get_log_file_status(state: State<'_, AppState>) -> Result<LogFileStatus, String> {
    Ok(state.log_center.file_status())
}
