//! 基于 notify-rust 的 macOS 专用通知实现。

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;

use notify_rust::{set_application, Notification};

/// 确保 `set_application` 在每个进程生命周期内最多只调用一次。
/// 底层库对重复调用持有全局锁限制；在 release 模式下会完全跳过该调用，
/// 因为通知系统会自动使用 app bundle 自带的标识符。
static APP_IDENTITY_SET: AtomicBool = AtomicBool::new(false);

fn set_app_identity(_app: &AppHandle) -> Result<(), String> {
    // In release mode, the app runs as a proper .app bundle.
    // notify_rust automatically uses the bundle identifier,
    // and calling set_application can cause unwanted Terminal window activation.
    if !cfg!(debug_assertions) {
        return Ok(());
    }

    if APP_IDENTITY_SET.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    // Debug mode: use Terminal as a fallback since the app runs from CLI.
    set_application("com.apple.Terminal")
        .map_err(|error| format!("set_application failed: {error}"))
}

pub fn show_playback(
    app: &AppHandle,
    title: &str,
    body: &str,
    cover_path: Option<&PathBuf>,
) -> Result<(), String> {
    set_app_identity(app)?;

    let mut notification = Notification::new();
    notification.summary(title).body(body);

    if let Some(path) = cover_path.and_then(|path| path.to_str()) {
        notification.image_path(path);
    }

    notification
        .show()
        .map_err(|error| format!("show playback failed: {error}"))?;

    Ok(())
}

pub fn show_download(app: &AppHandle, title: &str, body: &str) -> Result<(), String> {
    set_app_identity(app)?;

    Notification::new()
        .summary(title)
        .body(body)
        .show()
        .map_err(|error| format!("show download failed: {error}"))?;

    Ok(())
}

pub fn show_test(app: &AppHandle) -> Result<(), String> {
    set_app_identity(app)?;

    Notification::new()
        .summary("测试通知")
        .body("塞壬音乐下载器通知功能正常。")
        .show()
        .map_err(|error| format!("show test failed: {error}"))?;
    Ok(())
}
