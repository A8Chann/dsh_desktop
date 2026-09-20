---
name: http-control-service
description: >
  本地 19431 控制服务：/status、/action 动作表、close/quit/restart 语义、令牌限制。
---

# 本地 HTTP 控制服务（127.0.0.1:19431，仅本机、无鉴权）

WebView2 页面无法直接调 Tauri API（生产模式），统一走 HTTP 控制服务（`controls.rs::start_http_server`）：

- `GET /status` → 后端状态 JSON（state/url/port/pid/owned/error/nextRetrySec）
- `GET /plugin-hint` → `{"pending":bool}`：检测到插件变更、等待重启的提示态（只读，无需令牌）
- `GET /action?name=<action>` → 窗口/后端操作，支持：
  - `min` / `max` / `drag` — 窗口控制（必须经 `run_on_main_thread`）
  - `close` — **弹出「退出/缩小到托盘」选择框，不关窗**
  - `min-tray` — 隐藏窗口到托盘
  - `quit` — 先 `kill_owned()` 杀后端进程树，再退出
  - `restart` — 重启后端（就绪后自动刷新页面，逻辑统一在 `Backend::restart` 内）
  - `reload` / `browser` / `ping`

## 插件变更 → 标题栏药丸「点击重启更新插件」

- **置位**：`controls::start_plugin_watcher` 检测到 `profiles/<profile>/package.json` 或 `pnpm-lock.yaml` 变更
  （6s 安静期后）→ `set_plugin_change_pending(app, state, true)`（同时仍发系统 toast）
  → eval `window.__dshdPluginHint(true)` 给 chrome 页。
- **显示**：chrome.html 的状态药丸（`#status-pill`）变蓝，并在**真实状态文案之后追加**「· 点击重启更新插件」
  （`baseText + ' · 点击重启更新插件'`，不替换原状态文本——用户明确要求保留）、`role=button` + 可键盘 Enter；
  菜单里的状态行仍显示真实后端状态（`baseCls/baseText` 与提示态分离）。
- ⚠️ **有效提示态 = 已收到后端状态 且 检测到插件变更**（`statusSeen && pluginHint`，chrome.html 里叫 `hintActive`）：
  - 还没收到任何状态（状态桥未上报）时**不追加提示、不可点**，药丸保持占位「连接中…」——
    否则会出现自相矛盾的「后端未就绪 · 点击重启更新插件」（用户实测反馈）；
  - **click/keydown 处理器也必须判 `hintActive`**，只判 `pluginHint` 不够：状态未到时药丸虽然看着不可点，
    但 mousedown 的 `drag` 分支仍会放行 click 事件 → 实测真的会发出 `restart`。
- ⚠️ 提示态下 `#bar` 的 mousedown 必须**提前 return**（`e.target.closest('#status-pill.clickable')`）：
  否则会发 `act('drag')`，Windows 的 `start_dragging` 进入模态拖动循环**吃掉 click**（实测）。
- **点击**：药丸 click → `act('restart')`（与菜单「重启 Web 服务」同一动作；`Backend::restart` 就绪后自动刷新内容页）。
- **清除**：所有重启入口的唯一漏斗 `Backend::restart` 开头清位 → `window.__dshdPluginHint(false)`。
- 状态位在 `AppState.plugin_change_pending`，所以 chrome 页晚加载/重载后能靠 `GET /plugin-hint` 拉回。

## 参数命名冲突（重要）

- `/action?name=...` 的**其它参数不能叫 `name`**：HTTP 查询解析后 `HashMap` 里 `name` 会被动作名占用，再传 `name=profile名` 会覆盖动作名，导致动作被当成未知 action 直接失效。环境管理的 profile 参数统一用 `p`（`env-profile-use&p=web` 等）；版本用 `id`/`spec`/`label`，没有冲突。
