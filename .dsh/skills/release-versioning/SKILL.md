---
name: release-versioning
description: >
  打包发版、递增版本号、同步四处版本字段、cargo build --release 的权限要求。
---

# 构建与发布、版本号约定

## 构建（仅发版打包）

- `cd src-tauri && cargo build --release`（产物 `src-tauri/target/release/dsh-desktop.exe`）。
- 构建需 `danger-full-access`：cargo 要访问工作区外的 `~/.cargo` 缓存、rustc/link 工具链与 Temp 目录；产物路径本身在工作区内。
- 发布：把 release exe 复制为 `dist/DSH-Desktop-<version>-tauri.exe`。
- **不要把 exe 复制到用户桌面**——桌面部署由用户自己完成，助手只更新 dist。

## 版本约定（用户指定，2026-08-29 更新）

- **每次打包（构建新 exe）第三位 +1**：`2.0.2 → 2.0.3 → 2.0.4 → …`（未发版迭代无限递增）；
- **正式发版（打 tag/Release）除非另有说明：次版本 +1、第三位归 0**（如 2.0.3 迭代发版为 **2.1.0**；发版后下一轮迭代再继续第三位 +1）；
- **⚠️ 发布前必须核对：内部构建号（如 2.3.10）不能直接作为正式 Release tag；正确正式版本号 = 次版本 +1、第三位 0（如 2.4.0）。2026-08-31 教训：2.3.10 被误发，已按新规则改发 2.4.0。**
- 版本号需同步四处：`package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、`src-tauri/frontend/chrome.html`（badge + 「封装」两处）。
