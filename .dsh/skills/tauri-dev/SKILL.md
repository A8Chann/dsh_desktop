---
name: tauri-dev
description: >
  日常迭代用 `npx @tauri-apps/cli dev`（不要 cargo build --release）时的产物路径、进程管理与热重载行为。
---

# 开发环境（tauri dev 快速迭代）

- 日常开发/迭代一律用 `npx --yes @tauri-apps/cli@latest dev`（项目根运行），不要每次 `cargo build --release`——后者属于发布打包流程，只在发版时用。

## 🔴 必读：构建产物必须放在工作区外，否则 dev 起来一片空白

**DSH 沙箱按「可执行镜像是否在工作区内」限制写权限**：镜像在工作区内 → 只能写工作区，
写 `%TEMP%` / `%APPDATA%` / `%LOCALAPPDATA%` 一律 `拒绝访问 (os error 5)`。

`tauri dev` 默认构建到 `<workspace>\src-tauri\target\debug\`，**必然落在工作区内**，于是：

- 日志写不出（`%APPDATA%\DSH Desktop\logs\main.log` 被拒；代码是 `if let Ok(...)`，**静默无报错**）；
- WebView2 用户数据目录 `%LOCALAPPDATA%\io.dsh.desktop\EBWebView` 写不进去 → WebView2 起不来
  → **窗口壳在、内容全空白、`msedgewebview2` 子进程数为 0**。

症状极具迷惑性：进程活着、窗口标题正常、`Responding=True`、19431 也能应答，但"什么都没有"且日志一行不加。
**跟代码/皮肤/前端无关，别去查样式。**

### 正确起法（2026-10-09 实测可行）

```powershell
$env:CARGO_TARGET_DIR="D:\dsh-target"   # 任意工作区外目录
npx --yes @tauri-apps/cli@latest dev
```

产物落到 `D:\dsh-target\debug\dsh-desktop.exe`（工作区外）→ 一切正常：日志正常增长、
`msedgewebview2` 多出约 8 个进程、内容正常渲染。

- 等效做法：构建后把 exe 拷到工作区外运行（如 `D:\dsh-dev\`）。
- **无效做法**：只改 CWD（exe 在工作区外 + CWD 指向工作区依然正常 → 只看镜像位置）；
  `WEBVIEW2_USER_DATA_FOLDER` 指到工作区外（照样被拒）。
- 判据：起来后 `(Get-Process msedgewebview2).Count` 应比基线多约 8，且 main.log 有新增行。

## 日常工作方式

- 工作方式：
  - debug 编译（首次约 1~2 分钟拉 CLI + 全量编译；之后增量编译更快）；
  - `Watching D:\HTML\DSH_Desktop\src-tauri for changes...` 监视 Rust 源码，**保存 `.rs` 自动重编译 + 重启应用**；
  - 前端静态文件（`src-tauri/frontend/*.html`）dev 模式**从磁盘直接加载**，改完**刷新窗口页即生效**（无需编译）。
- 产物：默认 `src-tauri/target/debug/dsh-desktop.exe`；**设了 `CARGO_TARGET_DIR` 后是 `%CARGO_TARGET_DIR%\debug\dsh-desktop.exe`**（推荐，见上）。debug 版体量/性能与 release 不同，仅开发用。
- 验证接口与 release 一样：`http://127.0.0.1:19431/status`；后端仍会接管外部 dsh 实例（`probe_port` 修复后 401 认证的 dsh 也能正确识别）。
- 前置：registry 已配 npmmirror（工作区 `.npmrc`）；tauri CLI 走 npx 按需拉取，无需全局安装。
- 进程管理：tauri dev 是长驻进程（后台 job），**保持运行即开发环境活跃**；停掉（Ctrl+C/退出）则 dev 环境停了，重跑上面命令即可。
