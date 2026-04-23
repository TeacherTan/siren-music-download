//! 基于 Tauri 插件的 Windows/Linux 跨平台通知实现。

use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

pub fn show_playback(
    app: &AppHandle,
    title: String,
    body: String,
    cover_path: Option<PathBuf>,
) -> Result<(), String> {
    let mut builder = app.notification().builder().title(title).body(body);

    if let Some(path) = cover_path.and_then(|path| path.to_str().map(ToOwned::to_owned)) {
        builder = builder.icon(path);
    }

    builder.show().map_err(|error| error.to_string())?;

    Ok(())
}

pub fn show_download(app: &AppHandle, title: String, body: String) -> Result<(), String> {
    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .map_err(|error| error.to_string())?;

    Ok(())
}

pub fn show_test(app: &AppHandle) -> Result<(), String> {
    app.notification()
        .builder()
        .title("测试通知")
        .body("塞壬音乐下载器通知功能正常。")
        .show()
        .map_err(|error| error.to_string())?;

    Ok(())
}
