//! macOS-specific notification implementation using notify-rust.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;

use notify_rust::{set_application, Notification};

/// Ensures `set_application` is called at most once per process lifetime.
/// The underlying library has a global lock that errors on repeated calls.
/// In release mode, we skip this call entirely — the app bundle's identifier
/// is automatically used by the notification system.
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
