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
}

impl AppPreferences {
    /// 验证偏好是否合法
    pub(crate) fn validate(&self) -> Result<(), String> {
        match self.output_format.as_str() {
            "flac" | "wav" | "mp3" => {}
            _ => return Err(format!("不支持的格式: {}", self.output_format)),
        }
        let path = Path::new(&self.output_dir);
        if !path.exists() {
            return Err("保存路径不存在".to_string());
        }
        if !path.is_dir() {
            return Err("保存路径不是目录".to_string());
        }
        Ok(())
    }
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
    pub(crate) fn load(&self) -> AppPreferences {
        if self.path.exists() {
            match fs::read_to_string(&self.path) {
                Ok(content) => match toml::from_str::<AppPreferences>(&content) {
                    Ok(prefs) => return prefs,
                    Err(e) => {
                        eprintln!("[preferences] failed to parse TOML: {e}");
                    }
                },
                Err(e) => {
                    eprintln!("[preferences] failed to read file: {e}");
                }
            }
        }
        // 缺失或损坏时写入默认值（output_dir 使用下载目录兜底）
        let default_output_dir = dirs::download_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("SirenMusic")
            .to_string_lossy()
            .to_string();
        let default_prefs = AppPreferences {
            schema_version: 1,
            output_format: "flac".to_string(),
            output_dir: default_output_dir,
            download_lyrics: true,
            notify_on_download_complete: true,
            notify_on_playback_change: true,
        };
        if let Err(e) = self.save(&default_prefs) {
            eprintln!("[preferences] failed to write default preferences: {e}");
        }
        default_prefs
    }

    /// 原子写入偏好到 TOML 文件
    pub(crate) fn save(&self, prefs: &AppPreferences) -> Result<(), String> {
        let parent = self.path.parent().ok_or("no parent directory")?;
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create directory {:?}: {e}", parent))?;
        let content = toml::to_string_pretty(prefs)
            .map_err(|e| format!("failed to serialize preferences: {e}"))?;
        fs::write(&self.path, content.as_bytes())
            .map_err(|e| format!("failed to write preferences file: {e}"))?;
        Ok(())
    }

    /// 导出偏好到指定路径
    pub(crate) fn export_to(&self, prefs: &AppPreferences, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("failed to create export directory: {e}"))?;
            }
        }
        let content = toml::to_string_pretty(prefs)
            .map_err(|e| format!("failed to serialize preferences: {e}"))?;
        fs::write(path, content.as_bytes())
            .map_err(|e| format!("failed to write export file: {e}"))?;
        Ok(())
    }

    /// 从指定路径导入偏好（读取后验证）
    pub(crate) fn import_from(&self, path: &Path) -> Result<AppPreferences, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("failed to read import file: {e}"))?;
        let prefs: AppPreferences = toml::from_str(&content)
            .map_err(|e| format!("failed to parse TOML: {e}"))?;
        prefs.validate()?;
        Ok(prefs)
    }
}
