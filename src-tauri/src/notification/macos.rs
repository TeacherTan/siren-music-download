//! macOS-specific notification implementation using notify-rust.

use std::path::PathBuf;
use tauri::AppHandle;

use notify_rust::{set_application, Notification};

fn set_app_identity(app: &AppHandle) -> Result<(), String> {
    let app_id = if cfg!(debug_assertions) {
        "com.apple.Terminal"
    } else {
        app.config().identifier.as_str()
    };

    set_application(app_id).map_err(|error| {
        let message = format!("set_application failed: {error}");
        eprintln!("[notification:macos] {message}");
        message
    })
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

    notification.show().map_err(|error| {
        let message = format!("show playback failed: {error}");
        eprintln!("[notification:macos] {message}");
        message
    })?;

    Ok(())
}

pub fn show_download(app: &AppHandle, title: &str, body: &str) -> Result<(), String> {
    set_app_identity(app)?;

    Notification::new()
        .summary(title)
        .body(body)
        .show()
        .map_err(|error| {
            let message = format!("show download failed: {error}");
            eprintln!("[notification:macos] {message}");
            message
        })?;

    Ok(())
}

pub fn show_test(app: &AppHandle) -> Result<(), String> {
    set_app_identity(app)?;

    Notification::new()
        .summary("测试通知")
        .body("塞壬音乐下载器通知功能正常。")
        .show()
        .map_err(|error| {
            let message = format!("show test failed: {error}");
            eprintln!("[notification:macos] {message}");
            message
        })?;

    eprintln!("[notification:macos] test notification delivered");
    Ok(())
}
