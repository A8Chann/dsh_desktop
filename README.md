# DSH Desktop 🖥️（Tauri 版）

DeepSeek Harness 的 Windows 桌面端。把 `dsh web` 从「cmd 敲命令 + 浏览器开标签页」变成双击即用的桌面应用：

- **Tauri（Rust + WebView2）**：单文件 exe 约 11MB（Electron 版 90MB），启动更快、内存更省；
- **自动拉起后端**：找不到 dsh 时**测速选最快 npm 源自动安装**（不用 npx），随后拉起 `dsh web` 并内嵌加载 GUI；
- **自绘标题栏**（参照 Deepseek-Harness-EAC 风格）：玻璃拟态、跟随 dsh 主题；
  - 左侧：图标 + 标题 + 后端状态徽标（比 EAC 多出的状态显示：运行中 · 端口 / 启动中 / 异常·重试）；
  - 右侧：⋯ 菜单（重启 Web 服务 / 重新加载 / 在浏览器打开 / 关于 / 退出）+ 最小化 / 最大化 / 关闭；
  - 按钮经本地 HTTP 控制服务（`127.0.0.1:19431`）可靠执行窗口操作；
- **托盘**：双击/左键单击显示主窗口，右键菜单含「显示主窗口 / 重启后端 / 在浏览器打开 / 打开日志目录 / 退出」；
- **重启后端**：dsh 后端重启完成后页面自动刷新；
- **插件变更**：仅系统通知提示手动重启（不再自动重启）；
- **旧实例智能接管**：端口已被 dsh web 占用则直接接管复用（状态徽标标注「外部实例」）；
- **崩溃自愈**：后端意外退出按退避策略（1s→30s）自动重启。

## 环境要求

- Windows 10/11 x64，[WebView2 运行时](https://developer.microsoft.com/microsoft-edge/webview2/)（系统自带或自动安装）
- 构建需要：Node.js（图标脚本）、Rust 工具链与 MSVC（见下）

## 构建

```bash
# 1) 图标（可选，仓库已带 assets/icon.png、src-tauri/icons/）
npm run icon      # 重新生成 assets/icon.png
npm run icon:ico  # 从 icon.png 生成全尺寸 assets/icon.ico

# 2) Tauri 构建（需 Rust stable + VS Build Tools C++ 工作负载）
cd src-tauri
cargo tauri build --no-bundle        # 产物：src-tauri/target/release/dsh-desktop.exe
```

发布产物：`dist/DSH-Desktop-<version>-tauri.exe`（单文件，双击即用）。

> 注意：`npm run icon:ico` 生成的 `src-tauri/icons/icon.ico` 需手动同步（复制 `assets/icon.ico` → `src-tauri/icons/icon.ico`）。

## 目录结构

```
src-tauri/                 Tauri（Rust）桌面壳
  src/
    main.rs                入口：窗口/托盘/菜单/后端导航与状态推送
    backend.rs             dsh 后端管理：拉起/接管外部实例/退避自愈/状态发布
    controls.rs            自绘标题栏注入脚本 + 本地 HTTP 控制服务 + 插件变更监控 + 控制通道 + 自动汇报
    settings.rs            设置读写（%APPDATA%\DSH Desktop\settings.json，与旧版兼容）
    util.rs                日志/netstat 端口探测/HTTP 探测
  frontend/                loading.html（启动页）
  icons/                   应用图标集
scripts/                   图标生成（纯 Node，零依赖）
assets/                    icon.png / icon.ico 源图标
```

## 使用

| 操作 | 方式 |
|---|---|
| 重启后端 | 标题栏 ⋯ 菜单「重启 Web 服务」（重启后自动刷新）/ 托盘 |
| 刷新前端 | ⋯ 菜单「重新加载」 |
| 窗口控制 | 自绘标题栏 ─ ▢ ✕ |
| 显示主窗口 | 托盘双击 / 托盘右键「显示主窗口」 |
| 在浏览器打开 | ⋯ 菜单 / 托盘 |

## 排障

- 日志：`%APPDATA%\DSH Desktop\logs\main.log`（含前端打点 `[web]`、HTTP 控制 `[http]` 等，可直接定位问题）
- 本地 HTTP 控制端口 `127.0.0.1:19431` 被占用：杀掉占用进程后重启应用
- 任务栏图标显示旧缓存：重启 Windows 资源管理器

## 许可

MIT