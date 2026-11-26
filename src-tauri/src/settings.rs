use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 双击或在访达/资源管理器用“打开方式”打开压缩包时的行为
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoubleClickAction {
    /// 直接在后台静默智能解压到同级目录（类似 Keka / 归档实用工具）
    SmartExtract,
    /// 打开 QPack 窗口预览压缩包内容
    Browse,
}

impl Default for DoubleClickAction {
    fn default() -> Self {
        Self::Browse
    }
}

/// QPack 用户偏好设置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    /// 双击/打开方式触发压缩包时的默认动作
    pub double_click_action: DoubleClickAction,
    /// 点击窗口关闭按钮时隐藏到后台托盘/菜单栏常驻
    pub close_to_tray: bool,
    /// 压缩时默认排除 .DS_Store / __MACOSX / Thumbs.db
    pub exclude_junk: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            double_click_action: DoubleClickAction::Browse,
            close_to_tray: true,
            exclude_junk: true,
        }
    }
}

fn settings_file_path() -> PathBuf {
    let base = if cfg!(target_os = "macos") {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|h| h.join("Library/Application Support/com.qpack.app"))
    } else if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|p| p.join("com.qpack.app"))
    } else {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|h| h.join(".config/qpack"))
    }
    .unwrap_or_else(|| std::env::temp_dir().join("qpack-config"));

    base.join("settings.json")
}

/// 加载用户设置（不存在时返回默认值）
pub fn load_settings() -> AppSettings {
    let path = settings_file_path();
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// 保存用户设置到本地磁盘
pub fn save_settings(settings: &AppSettings) -> Result<(), std::io::Error> {
    let path = settings_file_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(path, json)
}
