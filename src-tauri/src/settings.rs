//! 设置读写：%APPDATA%\DSH Desktop\settings.json（与 Electron 版完全兼容：
//! 序列化为 camelCase，读取同时兼容 camelCase 与 snake_case 键）。
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 一个受管理的 dsh 版本条目：安装/更新到独立目录，互不影响。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DshVersionEntry {
    /// 稳定 id（生成后不变）
    pub id: String,
    /// 用户备注名（可改）
    pub label: String,
    /// 请求的 npm 版本说明（latest / 精确版本 / semver 范围）
    pub spec: String,
    /// 实际安装到的版本（安装成功后从 package.json 回填）
    pub installed: Option<String>,
    /// 独立安装前缀目录（node_modules/@deepseek-ai/dsh 所在根）
    pub dir: String,
    /// 解析出的 bin.js 绝对路径
    pub bin: Option<String>,
    /// managed（桌面端管理）/ global（npm 全局）/ manual（手动路径）
    pub source: String,
}

impl Default for DshVersionEntry {
    fn default() -> Self {
        Self {
            id: String::new(),
            label: String::new(),
            spec: "latest".to_string(),
            installed: None,
            dir: String::new(),
            bin: None,
            source: "managed".to_string(),
        }
    }
}

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
    /// 受管理的 dsh 版本列表（独立目录安装，互不覆盖）
    #[serde(default)]
    pub dsh_versions: Vec<DshVersionEntry>,
    /// 当前激活的版本条目 id（与 dsh_bin 联动）
    #[serde(default)]
    pub active_version_id: Option<String>,
    /// 下载保存目录；None → 系统下载目录
    #[serde(alias = "download_dir")]
    pub download_dir: Option<String>,
    /// 下载前是否询问每个文件的保存位置
    #[serde(alias = "ask_download_location")]
    pub ask_download_location: bool,
    /// 下载开始时是否自动显示下载面板
    #[serde(alias = "show_downloads_on_start")]
    pub show_downloads_on_start: bool,
    /// 标题栏主题色浓度（0-100）：越低越透、云母材质越明显；越高越贴合内容区配色
    #[serde(alias = "titlebar_tint")]
    pub titlebar_tint: u8,
    /// 弹窗（菜单/下载/设置/环境等）背景主题色浓度（0-100），独立于标题栏
    #[serde(alias = "popup_tint")]
    pub popup_tint: u8,
    /// 窗口材质：acrylic（亚克力，实时模糊下层窗口）/ mica（云母，仅壁纸色底纹）/ none（纯色）
    #[serde(alias = "window_material")]
    pub window_material: String,
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
            dsh_versions: Vec::new(),
            active_version_id: None,
            download_dir: None,
            ask_download_location: false,
            show_downloads_on_start: true,
            titlebar_tint: 18,
            popup_tint: 55,
            window_material: "acrylic".to_string(),
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
