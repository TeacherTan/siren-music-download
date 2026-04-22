use crate::app_state::AppState;
use siren_core::{SearchLibraryRequest, SearchLibraryResponse};
use tauri::State;

#[tauri::command]
pub async fn search_library(
    state: State<'_, AppState>,
    request: SearchLibraryRequest,
) -> Result<SearchLibraryResponse, String> {
    state.library_search_service.search(request).await
}
