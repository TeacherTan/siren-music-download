//! 库内搜索与索引查询相关的 Tauri command。
//!
//! 当前暴露的接口覆盖本地库存索引上的搜索请求执行，
//! 主要用于前端搜索框、筛选切换与结果页的本地召回。

use crate::app_state::AppState;
use siren_core::{SearchLibraryRequest, SearchLibraryResponse};
use tauri::State;

/// 在本地索引中执行库内搜索。
///
/// 适用于搜索框提交、筛选条件切换，或需要在本地库存索引中按范围执行召回的场景。
/// 入参 `request` 描述查询词、分页与搜索范围；返回值为本次搜索结果与索引状态。
/// 该接口依赖本地索引状态；当索引尚未就绪或正在重建时，调用方应结合返回值中的状态字段决定展示空结果、占位态还是重试提示。
#[tauri::command]
pub async fn search_library(
    state: State<'_, AppState>,
    request: SearchLibraryRequest,
) -> Result<SearchLibraryResponse, String> {
    state.library_search_service.search(request).await
}
