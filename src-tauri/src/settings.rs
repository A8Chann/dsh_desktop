//! 设置读写：%APPDATA%\DSH Desktop\settings.json（与 Electron 版完全兼容：
//! 序列化为 camelCase，读取同时兼容 camelCase 与 snake_case 键）。
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// dsh web 监听端口；0 = 系统分配
    pub port: u16,
    /// dsh 后端工作目录；None → 用户主目录
    #[serde(alias = "workspace")]
    pub workspace: Option<String>,
    /// 保留字段：插件变更后仅提示手动重启（不再自动重启）
    #[serde(alias = "auto_restart_after_plugin_change")]
    pub auto_restart_after_plugin_change: bool,
    /// 保留字段：旧版自动重启倒计时秒数（已不使用）
    #[serde(alias = "restart_countdown_sec")]
    pub restart_countdown_sec: u32,
    /// 要启动的 dsh profile
    #[serde(alias = "profile")]
    pub profile: String,
    /// 手动指定 node.exe 路径
    #[serde(alias = "node_bin")]
    pub node_bin: Option<String>,
    /// 手动指定 dsh 入口（bin.js 绝对路径）
    #[serde(alias = "dsh_bin")]
    pub dsh_bin: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            port: 3080,
            workspace: None,
            auto_restart_after_plugin_change: false,
            restart_countdown_sec: 6,
            profile: "web".to_string(),
            node_bin: None,
            dsh_bin: None,
        }
    }
}

/// 与 Electron 版一致的配置文件路径。
pub fn settings_dir() -> PathBuf {
    let base = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")));
    base.join("DSH Desktop")
}

pub fn settings_file() -> PathBuf {
    settings_dir().join("settings.json")
}

pub fn control_dir() -> PathBuf {
    if let Ok(d) = std::env::var("DSH_DESKTOP_CONTROL_DIR") {
        return PathBuf::from(d);
    }
    settings_dir().join("control")
}

pub fn logs_dir() -> PathBuf {
    settings_dir().join("logs")
}

pub fn load_settings() -> Settings {
    let path = settings_file();
    let mut s = match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str::<Settings>(&text)
            .unwrap_or_else(|_| Settings::default()),
        Err(_) => Settings::default(),
    };
    if s.workspace.is_none() {
        if let Ok(home) = std::env::var("USERPROFILE") {
            s.workspace = Some(home);
        }
    }
    if let Some(w) = s.workspace.clone() {
        if w.is_empty() || !std::path::Path::new(&w).exists() {
            s.workspace = std::env::var("USERPROFILE").ok();
        }
    }
    s
}

pub fn save_settings(s: &Settings) {
    let dir = settings_dir();
    let _ = std::fs::create_dir_all(&dir);
    let v = serde_json::to_string_pretty(s).unwrap_or_default();
    let _ = std::fs::write(settings_file(), v);
}
