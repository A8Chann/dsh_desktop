//! Windows toast 通知（插件变更提示）。
//!
//! 为什么不用 tauri-plugin-notification 直接发：该插件在 Windows 上把 builder 的
//! `.icon()` 转成 notify-rust 的 `icon` 字段，而 notify-rust 的 Windows 构建
//! （`build_toast`）**从不读取该字段**——toast 左上角小图标只由 AppUserModelID
//! （AUMID）决定（`ToastNotificationManager::CreateToastNotifierWithId`）。
//! 本应用的 AUMID `io.dsh.desktop` 在便携版 exe（非 NSIS 安装、无快捷方式注册）
//! 下从未注册，Windows 只能回退到默认/通用图标；dev 构建则直接退化为
//! PowerShell 图标（notify-rust 回退 `Toast::POWERSHELL_APP_ID`）。
//!
//! 修复按官方 unpackaged 模式（tauri-winrt-notification `examples/unpackaged_app.rs`）：
//! 1. 把嵌入的图标落盘到 `%LOCALAPPDATA%\DSH Desktop\icon.png`；
//! 2. 注册 `HKCU\Software\Classes\AppUserModelId\io.dsh.desktop`
//!    （DisplayName + IconUri），让 toast 以本应用身份显示；
//! 3. 用 `tauri-winrt-notification` 直接发 toast，并显式设置
//!    `appLogoOverride` 图标作为双保险（不依赖注册表也能显示正确图标）。
#![cfg(windows)]

use std::{env, fs, path::PathBuf, process::Command};
#[cfg(windows)]
use std::os::windows::process::CommandExt; // creation_flags

/// 与 tauri.conf.json 的 identifier 一致。
const APP_ID: &str = "io.dsh.desktop";
const APP_NAME: &str = "DSH Desktop";

/// 把嵌入的 256px 图标写到用户本地目录并返回绝对路径。
fn icon_path() -> Result<PathBuf, String> {
    let base = env::var("LOCALAPPDATA")
        .or_else(|_| env::var("TEMP"))
        .or_else(|_| env::var("TMP"))
        .map_err(|e| format!("无本地数据目录: {e}"))?;
    let dir = PathBuf::from(base).join(APP_NAME);
    fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败: {e}"))?;
    let icon = dir.join("icon.png");
    fs::write(&icon, include_bytes!("../icons/128x128@2x.png"))
        .map_err(|e| format!("写入图标失败: {e}"))?;
    Ok(icon)
}

/// 注册 AUMID（每次刷新，幂等），返回图标文件路径。
fn ensure_app_identity() -> Result<PathBuf, String> {
    let icon = icon_path()?;
    let key = r"HKCU\Software\Classes\AppUserModelId\io.dsh.desktop";
    let icon_uri = icon.to_string_lossy().to_string();
    for (value, data) in [("DisplayName", APP_NAME), ("IconUri", icon_uri.as_str())] {
        let mut cmd = Command::new("reg.exe");
        cmd.args(["add", key, "/v", value, "/t", "REG_SZ", "/d", data, "/f"])
            .creation_flags(0x08000000); // CREATE_NO_WINDOW：GUI 应用不弹控制台
        let out = cmd.output();
        match out {
            Ok(o) if o.status.success() => {}
            Ok(o) => {
                return Err(format!(
                    "reg add {value} 失败: {}",
                    String::from_utf8_lossy(&o.stderr).trim()
                ))
            }
            Err(e) => return Err(format!("reg add {value} 失败: {e}")),
        }
    }
    Ok(icon)
}

/// 弹出「插件已变更」toast（显示应用图标）。
pub fn show_plugin_change_toast() -> Result<(), String> {
    let icon = ensure_app_identity()?;
    // appLogoOverride 的 src="file:///..." 用正斜杠路径（反斜杠可能加载失败）
    let icon_uri = icon.to_string_lossy().replace('\\', "/");
    tauri_winrt_notification::Toast::new(APP_ID)
        .icon(std::path::Path::new(&icon_uri), tauri_winrt_notification::IconCrop::Square, APP_NAME)
        .title("DSH Desktop — 插件已变更")
        .text2("请手动重启后端（悬浮条「重启」按钮）以加载新插件")
        .show()
        .map_err(|e| format!("toast 发送失败: {e:?}"))
}
