---
name: tauri-dev
description: >
  日常迭代用 `npx @tauri-apps/cli dev`（不要 cargo build --release）时的产物路径、进程管理与热重载行为。
---

# 开发环境（tauri dev 快速迭代）

- 日常开发/迭代一律用 `npx --yes @tauri-apps/cli@latest dev`（项目根运行），不要每次 `cargo build --release`——后者属于发布打包流程，只在发版时用。
- 工作方式：
  - debug 编译（首次约 1~2 分钟拉 CLI + 全量编译；之后增量编译更快）；
  - `Watching D:\HTML\DSH_Desktop\src-tauri for changes...` 监视 Rust 源码，**保存 `.rs` 自动重编译 + 重启应用**；
  - 前端静态文件（`src-tauri/frontend/*.html`）dev 模式**从磁盘直接加载**，改完**刷新窗口页即生效**（无需编译）。
- 产物：`src-tauri/target/debug/dsh-desktop.exe`（debug 版，体量/性能与 release 不同，仅开发用）。
- 验证接口与 release 一样：`http://127.0.0.1:19431/status`；后端仍会接管外部 dsh 实例（`probe_port` 修复后 401 认证的 dsh 也能正确识别）。
- 前置：registry 已配 npmmirror（工作区 `.npmrc`）；tauri CLI 走 npx 按需拉取，无需全局安装。
- 进程管理：tauri dev 是长驻进程（后台 job），**保持运行即开发环境活跃**；停掉（Ctrl+C/退出）则 dev 环境停了，重跑上面命令即可。
