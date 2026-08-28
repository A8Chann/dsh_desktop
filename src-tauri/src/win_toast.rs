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
//! 修复（Win10/11 实测结论）：
//! - `appLogoOverride` 的 `file:///` 与 `http://` 图标源、注册表 `IconUri`
//!   在非打包应用上**均不被 toast 平台采用**（全部回退成通用「文件」图标）；
//! - **官方途径 = 开始菜单快捷方式**：把带 `System.AppUserModel.ID` 的
//!   `.lnk` 放进开始菜单，toast 身份与图标由该快捷方式提供（图标 = exe 内嵌图标）；
//! - ⚠️ **不能在应用进程内用 Rust COM（IShellLinkW + IPropertyStore）创建快捷方式**：
//!   实测导致进程堆损坏崩溃（0xc0000374，每次 toast 触发与启动时均复现）。
//!   改为 spawn 独立 `powershell.exe` 进程执行（隔离：坏也只坏子进程）；
//! - 注册表 AUMID（DisplayName + IconUri）与 appLogoOverride 保留作双保险。
#![cfg(windows)]

use std::path::PathBuf;
use std::{env, fs, process::Command};

use std::os::windows::process::CommandExt; // creation_flags

/// 与 tauri.conf.json 的 identifier 一致。
const APP_ID: &str = "io.dsh.desktop";
const APP_NAME: &str = "DSH Desktop";

/// 创建快捷方式用的 PowerShell 脚本（纯 ASCII；WScript 路径 + C# IShellLinkW/
/// IPropertyStore 设置 AUMID；经典写法，已在本机实测通过）。
/// 注意：C# 必须用单引号 here-string（@" 会做变量插值，把代码改坏——踩过）。
const MAKE_LNK_PS1: &str = r#"param([string]$TargetExe)
$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
using System.Text;

[ComImport, Guid("00021401-0000-0000-C000-000000000046")]
public class ShellLinkCls { }

[ComImport, InterfaceType(ComInterfaceType.InterfaceIsIUnknown), Guid("000214F9-0000-0000-C000-000000000046")]
public interface IShellLinkW {
  void GetPath([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszFile, int cch, IntPtr pfd, uint fFlags);
  void GetIDList(out IntPtr ppidl);
  void SetIDList(IntPtr pidl);
  void GetDescription([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszName, int cch);
  void SetDescription([MarshalAs(UnmanagedType.LPWStr)] string pszName);
  void GetWorkingDirectory([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszDir, int cch);
  void SetWorkingDirectory([MarshalAs(UnmanagedType.LPWStr)] string pszDir);
  void GetArguments([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszArgs, int cch);
  void SetArguments([MarshalAs(UnmanagedType.LPWStr)] string pszArgs);
  void GetHotkey(out ushort pwHotkey);
  void SetHotkey(ushort wHotkey);
  void GetShowCmd(out int piShowCmd);
  void SetShowCmd(int iShowCmd);
  void GetIconLocation([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszIconPath, int cch, out int piIcon);
  void SetIconLocation([MarshalAs(UnmanagedType.LPWStr)] string pszIconPath, int iIcon);
  void SetRelativePath([MarshalAs(UnmanagedType.LPWStr)] string pszPathRel, uint dwReserved);
  void Resolve(IntPtr hwnd, uint fFlags);
  void SetPath([MarshalAs(UnmanagedType.LPWStr)] string pszFile);
}

[StructLayout(LayoutKind.Sequential)]
public struct PropertyKey { public Guid fmtid; public uint pid; }

[StructLayout(LayoutKind.Explicit)]
public struct PropVariant {
  [FieldOffset(0)] public ushort vt;
  [FieldOffset(8)] public IntPtr lpstr;
}

[ComImport, Guid("886d8eeb-8cf2-4446-8d02-cdba1dbdcf99"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
public interface IPropertyStore {
  void GetCount(out uint cProps);
  void GetAt(uint iProp, out IntPtr pKey);
  void GetValue(ref PropertyKey key, out PropVariant pv);
  void SetValue(ref PropertyKey key, ref PropVariant pv);
  void Commit();
}

public class LnkMaker {
  public static void Make(string targetExe, string lnkPath) {
    ShellLinkCls obj = new ShellLinkCls();
    IShellLinkW link = (IShellLinkW)obj;
    link.SetPath(targetExe);
    link.SetWorkingDirectory(System.IO.Path.GetDirectoryName(targetExe));
    link.SetIconLocation(targetExe, 0);
    link.SetDescription("DSH Desktop");

    // AUMID: 在内存对象上写属性并 Commit（不可先 IPersistFile::Load——会
    // 以只读方式打开，随后 SetValue/Commit 报 STG_E_ACCESSDENIED——踩过）
    IPropertyStore ps = (IPropertyStore)obj;
    PropertyKey key = new PropertyKey();
    key.fmtid = new Guid("9f4c2855-9f79-4b39-a8d0-e1d42de1d5f3");
    key.pid = 5;
    IntPtr buf = Marshal.StringToHGlobalUni("io.dsh.desktop");
    PropVariant pv = new PropVariant();
    pv.vt = 31;
    pv.lpstr = buf;
    try {
      ps.SetValue(ref key, ref pv);
      ps.Commit();
    } finally {
      Marshal.FreeHGlobal(buf);
    }

    IPersistFile pf = (IPersistFile)obj;
    pf.Save(lnkPath, true);
  }
}
'@
$lnkPath = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\DSH Desktop.lnk'
[LnkMaker]::Make($TargetExe, $lnkPath)
Write-Output 'OK'
"#;

/// 把嵌入的 256px 图标写到用户本地目录并返回绝对路径。
/// 目录**必须无空格**：appLogoOverride 的 src="file:///..." 是原始路径拼接，
/// 带空格的路径会让图标加载失败、回退成通用「文件」图标（实测踩坑）。
fn icon_path() -> Result<PathBuf, String> {
    let base = env::var("LOCALAPPDATA")
        .or_else(|_| env::var("TEMP"))
        .or_else(|_| env::var("TMP"))
        .map_err(|e| format!("无本地数据目录: {e}"))?;
    let dir = PathBuf::from(base).join("DSHDesktop"); // 注意：无空格，与 APP_NAME 故意不同
    fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败: {e}"))?;
    let icon = dir.join("icon.png");
    fs::write(&icon, include_bytes!("../icons/128x128@2x.png"))
        .map_err(|e| format!("写入图标失败: {e}"))?;
    Ok(icon)
}

/// 注册 AUMID（每次刷新，幂等），返回图标文件路径。
fn ensure_registry_identity() -> Result<PathBuf, String> {
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

/// 创建/刷新开始菜单快捷方式（带 AppUserModelID）——**隔离进程**方案。
/// 非打包 Win32 应用 toast 图标的官方途径：快捷方式提供身份 + 图标。
/// 由独立 powershell.exe 执行（经典 C# IShellLinkW+IPropertyStore 片段），
/// 崩溃也只影响该子进程，不伤应用本体（进程内 COM 实测堆损坏 0xc0000374）。
pub fn ensure_start_menu_shortcut() -> Result<(), String> {
    let exe = env::current_exe().map_err(|e| format!("获取 exe 路径失败: {e}"))?;
    let ps1 = env::temp_dir().join("dsh-desktop-make-lnk.ps1");
    fs::write(&ps1, MAKE_LNK_PS1.as_bytes())
        .map_err(|e| format!("写脚本失败: {e}"))?;
    let out = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-File",
            ps1.to_str().unwrap_or(""),
        ])
        .arg(exe.to_str().unwrap_or(""))
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()
        .map_err(|e| format!("启动 powershell 失败: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "powershell 创建快捷方式失败: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(())
}

/// 弹出「插件已变更」toast（显示应用图标）。
/// 只做纯 WinRT 发通知（快捷方式在应用启动时经隔离进程刷新；watcher 线程
/// 严禁进程内 COM——实测堆损坏崩溃 0xc0000374）。
pub fn show_plugin_change_toast() -> Result<(), String> {
    let icon = ensure_registry_identity()?;
    // appLogoOverride 的 src="file:///..." 用正斜杠路径（反斜杠可能加载失败）
    let icon_uri = icon.to_string_lossy().replace('\\', "/");
    tauri_winrt_notification::Toast::new(APP_ID)
        .icon(std::path::Path::new(&icon_uri), tauri_winrt_notification::IconCrop::Square, APP_NAME)
        .title("DSH Desktop — 插件已变更")
        .text2("请手动重启后端（悬浮条「重启」按钮）以加载新插件")
        .show()
        .map_err(|e| format!("toast 发送失败: {e:?}"))
}
