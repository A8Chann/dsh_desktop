---
name: http-control-service
description: >
  本地 19431 控制服务：/status、/action 动作表、close/quit/restart 语义、令牌限制。
---

# 本地 HTTP 控制服务（127.0.0.1:19431，仅本机、无鉴权）

WebView2 页面无法直接调 Tauri API（生产模式），统一走 HTTP 控制服务（`controls.rs::start_http_server`）：

- `GET /status` → 后端状态 JSON（state/url/port/pid/owned/error/nextRetrySec）
- `GET /action?name=<action>` → 窗口/后端操作，支持：
  - `min` / `max` / `drag` — 窗口控制（必须经 `run_on_main_thread`）
  - `close` — **弹出「退出/缩小到托盘」选择框，不关窗**
  - `min-tray` — 隐藏窗口到托盘
  - `quit` — 先 `kill_owned()` 杀后端进程树，再退出
  - `restart` — 重启后端（就绪后自动刷新页面，逻辑统一在 `Backend::restart` 内）
  - `reload` / `browser` / `ping`

## 参数命名冲突（重要）

- `/action?name=...` 的**其它参数不能叫 `name`**：HTTP 查询解析后 `HashMap` 里 `name` 会被动作名占用，再传 `name=profile名` 会覆盖动作名，导致动作被当成未知 action 直接失效。环境管理的 profile 参数统一用 `p`（`env-profile-use&p=web` 等）；版本用 `id`/`spec`/`label`，没有冲突。
