use crate::app_state::AppState;
use crate::local_inventory::{emit_local_inventory_state_changed, spawn_inventory_scan};
use siren_core::{LocalInventorySnapshot, VerificationMode};
use tauri::{AppHandle, State};

/// 获取当前本地库存扫描快照。
///
/// 适用于本地库存面板初始化、页面恢复后同步状态，或在事件流缺失时兜底拉取当前扫描结果。
/// 返回值为当前最新的本地库存快照。
/// 该接口不会主动启动扫描；若需要刷新结果，应显式调用重新扫描接口。
#[tauri::command]
pub async fn get_local_inventory_snapshot(
    state: State<'_, AppState>,
) -> Result<LocalInventorySnapshot, String> {
    Ok(state.local_inventory_service.snapshot().await)
}

/// 以当前输出目录重新触发本地库存扫描。
///
/// 适用于用户手动刷新本地库存、切换校验模式，或在下载目录内容发生变化后重建库存快照。
/// 入参 `verification_mode` 为可选校验模式覆盖；返回值为触发扫描当下的最新快照。
/// 该接口会异步启动扫描流程，返回时不代表扫描已经完成；调用方应结合后续事件或再次读取快照观察最终结果。
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

/// 取消当前进行中的本地库存扫描，并返回最新快照。
///
/// 适用于用户主动中止耗时扫描，或在即将切换目录/退出页面前停止当前扫描任务。
/// 返回值为取消动作之后的最新快照。
/// 该接口只会影响当前进行中的扫描；若此时没有活动扫描，返回的快照可能仅表示当前状态未发生变化。
#[tauri::command]
pub async fn cancel_local_inventory_scan(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<LocalInventorySnapshot, String> {
    let snapshot = state.local_inventory_service.cancel_scan().await;
    emit_local_inventory_state_changed(&app, &snapshot);
    Ok(snapshot)
}
