use crate::app_state::AppState;
use crate::local_inventory::{emit_local_inventory_state_changed, spawn_inventory_scan};
use siren_core::{LocalInventorySnapshot, VerificationMode};
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn get_local_inventory_snapshot(
    state: State<'_, AppState>,
) -> Result<LocalInventorySnapshot, String> {
    Ok(state.local_inventory_service.snapshot().await)
}

#[tauri::command]
pub async fn rescan_local_inventory(
    app: AppHandle,
    state: State<'_, AppState>,
    verification_mode: Option<VerificationMode>,
) -> Result<LocalInventorySnapshot, String> {
    let root_output_dir = state.preferences().output_dir;
    spawn_inventory_scan(
        app,
        state.inner().clone(),
        root_output_dir,
        verification_mode,
    );
    Ok(state.local_inventory_service.snapshot().await)
}

#[tauri::command]
pub async fn cancel_local_inventory_scan(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<LocalInventorySnapshot, String> {
    let snapshot = state.local_inventory_service.cancel_scan().await;
    emit_local_inventory_state_changed(&app, &snapshot);
    Ok(snapshot)
}
