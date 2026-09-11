---
name: window-close-exit
description: >
  窗口关闭与「退出/缩小到托盘」弹窗的踩坑规则，含 CloseRequested 拦截与退出清理顺序。
whenToUse: >
  窗口关闭、Alt+F4、托盘退出、关闭弹窗行为
---

# 窗口关闭与「退出/缩小到托盘」弹窗

## ✕ 按钮不能走 `win.close()`

程序化关闭**不触发** `CloseRequested` 事件，Rust 侧拦截会被整体绕过（结果：窗口直接关、应用退出、后端成孤儿进程）。

✕ 按钮在注入 JS 里直接调 `window.__dshdShowCloseDialog()`（自绘弹窗）。

## Alt+F4 / 任务栏关闭

`main.rs` 的 `.on_window_event` 拦截 `CloseRequested`：

- `api.prevent_close()` + `controls::show_close_dialog()`；
- `AppState.force_exit=true` 时放行（弹窗/菜单/托盘已明确选择退出）。

## 弹窗 CSS 选择器

弹窗挂在 `document.body` 下（**不是**标题栏 `#BAR_ID` 的后代）。

- 写成 `'#BAR_ID .dch-overlay'` 会样式全部失配，弹窗退化成页面底部一行裸 HTML 文本；
- 必须用弹窗自身独立 id 选择器。

## `hidden` 属性会被 CSS 覆盖

overlay 设了 `display:grid`（作者样式优先于 UA 的 `[hidden]{display:none}`）。

必须显式补：`'#id[hidden]{display:none!important}'`，否则弹窗永远显示无法隐藏。

## 退出清理不能依赖 `RunEvent::Exit`

`AppHandle::exit()` 内部 `request_exit` 在部分线程/时机下会失败并**直接 `std::process::exit`，Exit 事件根本不触发**（实测两次 quit 均无清理日志）。

正确做法：所有退出路径先同步 `Backend::kill_owned()`，再 `app.exit(0)`；`RunEvent::Exit` 回调仅作兜底。

## `kill_owned()` 规则

1. 先 `stop.store(true)` 阻止管理线程重拉；
2. taskkill `/T /F` 杀自有后端进程树；
3. 外部接管实例（`owned=false`）**不杀**；
4. 状态里没记到 pid 时按端口反查（`find_pid_by_port`）兜底。
