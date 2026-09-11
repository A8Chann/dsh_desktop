---
name: windows-toast-icon
description: >
  Windows toast 通知小图标的实现限制与正确做法（AUMID + 开始菜单快捷方式 + 隔离进程）。
whenToUse: >
  Windows toast 通知、通知图标、AUMID、开始菜单快捷方式
---

# Windows toast 通知图标

## tauri-plugin-notification 无法设置 toast 小图标

builder 的 `.icon()` 只会写 notify-rust 的 `icon` 字段，而 notify-rust 的 Windows 构建（`build_toast`）**从不读取该字段**。

toast 左上角小图标由 AppUserModelID（AUMID）对应实体的图标决定（`CreateToastNotifierWithId(aumid)`）。

## 非打包应用的官方途径 = 开始菜单快捷方式

快捷方式需带 `System.AppUserModel.ID`，图标指向 exe。

实测以下方式**全部不被 toast 平台采用**，一律回退通用「文件」图标：

- `appLogoOverride`（`file:///` 与 `http://` 源）；
- 注册表 `IconUri`。

## 「空壳」快捷方式

Windows 首次用该 AUMID 发 toast 时会自动创建一个「空壳」快捷方式（指向随机 `%TEMP%\xxx\` 目录、IconLocation 为空）——这就是「默认文件图标」的真正来源。

必须用真实 exe 覆盖它：`TargetPath` = 当前 exe、`IconLocation` = `exe,0`、AUMID 属性。

## ⚠️ 禁止进程内 Rust COM 创建 .lnk

进程内用 IShellLinkW + IPropertyStore + 手写 PROPVARIANT 会**堆损坏崩溃 0xc0000374**（ntdll.dll，Windows 事件日志 Application Error 1000）：第一次可能“成功”（快捷方式写出），但内存已坏，之后**每次 toast 触发或下次启动必崩**（「程序莫名其妙关」/「启动打不开」同根因）。

**必须用隔离进程**：启动时 spawn `powershell.exe`（`-WindowStyle Hidden` + `CREATE_NO_WINDOW`）执行经典 C# 片段（`win_toast.rs::ensure_start_menu_shortcut`，已在 win_toast.rs 内固化）。

## C# 片段要点

- IShellLinkW 设 Target / WorkingDirectory / IconLocation；
- IPropertyStore 写 AUMID（PKEY `{9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3}` pid=5，VT_LPWSTR，`Marshal.StringToHGlobalUni` + `FreeHGlobal`）→ Commit → IPersistFile::Save；
- **不要先 `IPersistFile::Load`**（只读打开 → SetValue/Commit 报 STG_E_ACCESSDENIED）；
- **C# 必须用单引号 here-string `@'…'@`**（`@"…"@` 会做 PowerShell 变量插值把代码改坏）。

## 保留项

- 注册表 AUMID（DisplayName + IconUri，无空格路径）；
- `Toast::icon()` appLogoOverride 正斜杠路径作双保险；
- 非 Windows 平台仍走 tauri-plugin-notification（`backend.rs::on_plugin_change` 的 `cfg` 分支）。
