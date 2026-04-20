use crate::logging::{LogCenter, LogLevel, LogPayload};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// 统一应用偏好模型（TOML 序列化格式：snake_case 字段名）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppPreferences {
    /// Schema version for future migrations. Defaults to 1 if missing.
    #[serde(default)]
    pub(crate) schema_version: i32,
    pub(crate) output_format: String,
    pub(crate) output_dir: String,
    pub(crate) download_lyrics: bool,
    pub(crate) notify_on_download_complete: bool,
    pub(crate) notify_on_playback_change: bool,
    #[serde(default = "default_log_level")]
    pub(crate) log_level: String,
}

impl AppPreferences {
    /// 验证偏好是否合法
    pub(crate) fn validate(&self) -> Result<(), String> {
        match self.output_format.as_str() {
            "flac" | "wav" | "mp3" => {}
            _ => return Err(format!("不支持的格式: {}", self.output_format)),
        }
        if LogLevel::parse(&self.log_level).is_none() {
            return Err(format!("不支持的日志等级: {}", self.log_level));
        }
        let path = Path::new(&self.output_dir);
        if path.as_os_str().is_empty() || !path.is_absolute() {
            return Err("保存路径必须是绝对目录路径".to_string());
        }
        if !path.exists() {
            return Err("保存路径不存在".to_string());
        }
        ensure_not_symlink(path, "保存路径不能是符号链接")?;
        if !path.is_dir() {
            return Err("保存路径不是目录".to_string());
        }
        Ok(())
    }
}

fn default_log_level() -> String {
    LogLevel::Error.as_str().to_string()
}

fn validate_explicit_export_path(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() || !path.is_absolute() {
        return Err("导出路径必须是绝对文件路径".to_string());
    }
    if path.exists() {
        ensure_not_symlink(path, "导出路径不能是符号链接")?;
    }
    if path.exists() && path.is_dir() {
        return Err("导出路径必须是文件路径".to_string());
    }
    Ok(())
}

fn validate_explicit_import_path(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() || !path.is_absolute() {
        return Err("导入路径必须是绝对文件路径".to_string());
    }
    if !path.exists() {
        return Err("导入文件不存在".to_string());
    }
    ensure_not_symlink(path, "导入路径不能是符号链接")?;
    if !path.is_file() {
        return Err("导入路径必须是文件".to_string());
    }
    Ok(())
}

fn ensure_not_symlink(path: &Path, message: &str) -> Result<(), String> {
    if fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err(message.to_string());
    }
    Ok(())
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            schema_version: 1,
            output_format: "flac".to_string(),
            output_dir: String::new(),
            download_lyrics: true,
            notify_on_download_complete: true,
            notify_on_playback_change: true,
            log_level: default_log_level(),
        }
    }
}

/// 偏好持久化管理器
#[derive(Clone)]
pub(crate) struct PreferencesStore {
    path: PathBuf,
}

impl PreferencesStore {
    pub(crate) fn new(app_data_dir: PathBuf) -> Self {
        let path = app_data_dir.join("preferences.toml");
        Self { path }
    }

    /// 从 TOML 文件加载偏好，缺失或损坏时用默认值初始化并写入
    pub(crate) fn load(&self, log_center: Option<&LogCenter>) -> AppPreferences {
        if self.path.exists() {
            match fs::read_to_string(&self.path) {
                Ok(content) => match toml::from_str::<AppPreferences>(&content) {
                    Ok(prefs) => match prefs.validate() {
                        Ok(()) => return prefs,
                        Err(error) => {
                            if let Some(log_center) = log_center {
                                log_center.record(
                                    LogPayload::new(
                                        LogLevel::Error,
                                        "preferences",
                                        "preferences.invalid_persisted",
                                        "Persisted preferences are invalid",
                                    )
                                    .user_message("偏好配置无效，已回退到默认设置")
                                    .details(error.clone()),
                                );
                            }
                            eprintln!("[preferences] invalid persisted preferences: {error}");
                        }
                    },
                    Err(e) => {
                        if let Some(log_center) = log_center {
                            log_center.record(
                                LogPayload::new(
                                    LogLevel::Error,
                                    "preferences",
                                    "preferences.parse_failed",
                                    "Failed to parse persisted preferences",
                                )
                                .user_message("偏好配置损坏，已回退到默认设置")
                                .details(e.to_string()),
                            );
                        }
                        eprintln!("[preferences] failed to parse TOML: {e}");
                    }
                },
                Err(e) => {
                    if let Some(log_center) = log_center {
                        log_center.record(
                            LogPayload::new(
                                LogLevel::Error,
                                "preferences",
                                "preferences.read_failed",
                                "Failed to read persisted preferences",
                            )
                            .user_message("读取偏好配置失败，已回退到默认设置")
                            .details(e.to_string()),
                        );
                    }
                    eprintln!("[preferences] failed to read file: {e}");
                }
            }
        }
        // 缺失或损坏时写入默认值（output_dir 使用下载目录兜底）
        let default_output_dir = dirs::download_dir()
            .unwrap_or_else(|| {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"))
            })
            .join("SirenMusic");
        let resolved_output_dir = if fs::create_dir_all(&default_output_dir).is_ok() {
            default_output_dir
        } else {
            self.path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| std::path::PathBuf::from("/"))
        };
        let default_prefs = AppPreferences {
            schema_version: 1,
            output_format: "flac".to_string(),
            output_dir: resolved_output_dir.to_string_lossy().to_string(),
            download_lyrics: true,
            notify_on_download_complete: true,
            notify_on_playback_change: true,
            log_level: default_log_level(),
        };
        if let Err(e) = self.save(&default_prefs) {
            if let Some(log_center) = log_center {
                log_center.record(
                    LogPayload::new(
                        LogLevel::Error,
                        "preferences",
                        "preferences.write_default_failed",
                        "Failed to write default preferences",
                    )
                    .details(e.clone()),
                );
            }
            eprintln!("[preferences] failed to write default preferences: {e}");
        }
        default_prefs
    }

    /// 原子写入偏好到 TOML 文件
    pub(crate) fn save(&self, prefs: &AppPreferences) -> Result<(), String> {
        let parent = self.path.parent().ok_or("偏好目录无效")?;
        fs::create_dir_all(parent).map_err(|_| "创建偏好目录失败".to_string())?;
        let content = toml::to_string_pretty(prefs)
            .map_err(|e| format!("failed to serialize preferences: {e}"))?;
        fs::write(&self.path, content.as_bytes()).map_err(|_| "写入偏好文件失败".to_string())?;
        Ok(())
    }

    /// 导出偏好到指定路径
    pub(crate) fn export_to(&self, prefs: &AppPreferences, path: &Path) -> Result<(), String> {
        validate_explicit_export_path(path)?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|_| "创建导出目录失败".to_string())?;
            }
        }
        let content = toml::to_string_pretty(prefs)
            .map_err(|e| format!("failed to serialize preferences: {e}"))?;
        fs::write(path, content.as_bytes()).map_err(|_| "写入导出文件失败".to_string())?;
        Ok(())
    }

    /// 从指定路径导入偏好（读取后验证）
    pub(crate) fn import_from(&self, path: &Path) -> Result<AppPreferences, String> {
        validate_explicit_import_path(path)?;
        let content = fs::read_to_string(path).map_err(|_| "读取导入文件失败".to_string())?;
        let prefs: AppPreferences =
            toml::from_str(&content).map_err(|e| format!("failed to parse TOML: {e}"))?;
        prefs.validate()?;
        Ok(prefs)
    }
}
