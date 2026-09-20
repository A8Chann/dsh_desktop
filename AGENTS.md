# DSH Desktop（Tauri 桌面壳）开发指南

DeepSeek Harness 的 Windows 桌面端：Tauri v2 + WebView2，内嵌 dsh web GUI，自动拉起后端、自绘标题栏、托盘、本地 HTTP 控制服务。

> **本文件是「热上下文」，每轮都会注入，必须保持精简（目标 < 12 KB）。**
> 详细经验、实测数值、历史方案与踩坑记录**一律写进项目目录 `.dsh/skills/<主题>/SKILL.md`（即 `D:\HTML\DSH_Desktop\.dsh\skills\...`，不是 `~/.dsh/skills`，也不是别处）**，
> 由技能目录按需加载；**不要再往本文件追加长段落**。
> 新增经验时：先看下面「技能索引」有没有对应主题，有就更新那个 SKILL.md。

## 环境

- 开发目录 `D:\HTML\DSH_Desktop`；受管 dsh 版本在 `%APPDATA%\DSH Desktop\versions\<id>\`。
- 日志 `%APPDATA%\DSH Desktop\logs\main.log`（UTF-8，PowerShell 控制台按 GBK 读会乱码）。
- 本地控制服务 `http://127.0.0.1:19431`（`/status`、`/action?name=…`、`/downloads`、`/env`、`/theme`）。
- profile 在 `~/.dsh/profiles/web`；配置 `~/.dsh/settings.yaml`；凭据 `~/.dsh/.credentials.yaml`。

## 核心底线（违反会直接出事）

1. **用中文回复正文**（工具日志/命令输出不受此约束）。
2. **日常迭代一律 `npx --yes @tauri-apps/cli@latest dev`**，不要每次 `cargo build --release`（那只是发版流程）。
3. **版本号约定**：每次打包第三位 +1；正式发版次版本 +1、第三位归 0。⚠️ 内部构建号不能直接当 Release tag。需同步 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、`src-tauri/frontend/chrome.html`（两处）。
4. **只更新 `dist/`，不要把 exe 复制到用户桌面**。
5. **Rust 里 `app.state::<T>()` 一律用 `try_state`**（回调可能早于 `manage()` 触发 → 0xc0000409 崩溃）。
6. **前端一律事件驱动，不得加 `setInterval` 轮询**（启动页、环境管理面板均如此）。
7. **升级/新增插件用受管版本的 dsh**，不要用全局 dsh；Windows 下 `plugin add` 的版本号**不写 `^`**。
8. **改宿主侧代码/插件模块后必须重启后端**；只改客户端产物（`client.js`）F5 即可。
9. **不要在实例运行时手改内存回写型配置**（如 `~/.dsh/storages/*/ledger.json`），会被 flush 覆盖，只能走 UI。
10. **改皮肤/样式类改动，验证前必须先刷新内容页**（F5 或 `Page.reload {ignoreCache:true}`）。
11. **含中文的文件不要用 PowerShell `Get-Content`/`Set-Content` 往返**（编码会坏），用 write/edit 工具。
12. **CSS-module 哈希选择器（`[class*="<hash>_x"]`）一律不要裸写**：加 `[data-slot="main.conversation"]` 作用域，或优先用语义属性（`data-slot` / `data-dsh-*` / `data-composer-*`）。

## 技能索引（按需加载，不要在本文件里展开）

> 本表所有技能文件都存于**本项目** `.dsh/skills/<主题>/SKILL.md`（即 `D:\HTML\DSH_Desktop\.dsh\skills\...`）；找技能只来项目目录这里找，不要搜 `~/.dsh/skills` 或其它位置。

| 场景 | skill |
|---|---|
| tauri dev 工作流、debug 产物 | `tauri-dev` |
| 打包发版、版本号同步 | `release-versioning` |
| 19431 控制服务与 action | `http-control-service` |
| 后端进程管理、重启死锁 | `backend-management` |
| 环境管理（版本 / Profile） | `environments` |
| 下载器与下载浮层 | `downloads` |
| 启动流程、单实例、透明窗口 | `startup-flow` |
| 日志排障、字段命名 | `logging-troubleshooting` |
| 窗口关闭 / 托盘退出清理 | `window-close-exit` |
| Windows toast 图标 | `windows-toast-icon` |
| 主题桥、标题栏变黑 | `theme-bridge` |
| 毛玻璃材质、切页主题 | `window-material-theme-sync` |
| 插件安装/升级、冷静期、兼容性 | `web-plugin-upgrade` |
| 第三方插件本地打补丁 | `plugin-local-patch` |
| memos-cloud 插件兼容 | `memos-cloud-plugin` |
| Command Code provider（思考强度；tool 结果图片触发顺序「insufficient tool messages」400 补丁） | `commandcode-provider` |
| cost-meter 的 CommandCode 面板 | `cost-meter-commandcode` |
| cost-meter 图框与 Go 版式对齐 | `cost-meter-plan-box` |
| 会话迁移 bug | `session-migration` |
| 皮肤：patches.css 维护与上游合并 | `patches-css-maintenance` |
| 皮肤：能否改 skin.json / hooks | `skin-layer-boundary` |
| 皮肤：气泡变量联动 | `bubble-skin-vars` |
| 皮肤：气泡模糊滑块补丁 | `skin-center-knob-patch` |
| 皮肤：输入栏下方三行、dock 样式 | `composer-dock-styling` |
| 皮肤：导航栏 / 侧边栏底色 | `nav-and-sidebars` |
| 皮肤：哈希选择器误伤 | `hash-selector-pitfalls` |
| 皮肤：宽度自适应 | `width-fit-content` |
| 无头 Edge + CDP 验证 | `headless-verification` |
| 侧边栏「点不动」误判 | `sidebar-list-misdiagnosis` |
| 梁神模式预设移除 | `liangshen-preset` |
| 外链点了没反应 / 用默认浏览器打开 | `external-links` |

## 本仓库的冷存档（不属于热上下文）

- `assets/skins/blue-fantasy/` —— 皮肤成品 + 上游基准 + provenance 快照。
- `assets/upstream-pr/` —— 待提交给皮肤中心上游的 PR 材料。
- `scripts/` —— 补丁与验证工具：`cost-meter-plan-box.mjs`、`skin-center-bubble-blur.mjs`
  （**已废弃**，功能随上游 0.3.22 发布，脚本自带版本闸门）、`skin-patches.ps1`、
  `session-log.mjs`（多帧 zstd 会话日志：`frames`/`dump`/`raw`/`scan`）、
  `verify-toolimg-patch.mjs`、`cdp/`（无头验证三件套）。
- 这些用 `AGENTS.md.bak-*` 之类**不要**再堆在根目录。

## 会话协作约定

- 记忆召回与写回由 MemOS Cloud 插件自动完成，**不要**手动调用 `mcp__memos-mcp__*`（仅主动管理记忆时按需使用）。
- 用用户的语言（中文）回复正文。
