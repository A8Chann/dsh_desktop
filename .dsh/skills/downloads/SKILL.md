---
name: downloads
description: >
  下载功能：on_download 拦截、自管下载器、下载管理浮层、进度与暂停取消。
---

# 下载支持（自管下载器 downloads.rs）

- **不要用 WebView2 原生下载**：tauri 的 `DownloadEvent` 只有 Requested/Finished，**没有进度、没有暂停/取消控制**；且实测 WebView2 原生下载链路会崩溃（0xc0000409，崩溃偏移固定，无 [download] 日志）。
- 方案：`on_download` 拦截请求（解析文件名后**返回 `false` 阻止 WebView2**），交给 `downloads.rs` 用 **ureq 流式下载**到系统下载目录，维护进度/暂停/继续/取消（每块检查 AtomicBool 标志），并经 HTTP 控制服务暴露：
  - `GET /downloads` → 任务列表 JSON（id/name/url/file/state/bytes/total/error）
  - `action download-cancel?id=` / `download-pause?id=&on=0|1` / `download-open?id=`（explorer /select）/ `download-delete?id=`（删文件+移记录）/ `download-retry?id=` / `download-browser?id=`（转交系统浏览器并自动取消自管下载，避免重复）
- 注入 JS 提供「下载管理」**右上角下拉卡片**（类 Edge 下载浮层；⋯ 菜单入口 / `__dshdShowDownloads()`）：列表 + 进度条 + 按钮矩阵（下载中=暂停/用浏览器下载/取消；已暂停=继续/取消；完成=打开文件夹/删除；失败/取消=重试/用浏览器下载/删除），**事件驱动刷新（`__dshdDlChanged`；下载线程每 500ms fire 一次进度）**，点击外部/ESC 关闭；样式沿用弹窗经验：独立 id CSS + `[hidden]{display:none!important}`（下拉用 `position:fixed;top:44px;right:12px`，无遮罩）。
- **全部 `app.state::<T>()` 一律用 `try_state`**：回调（on_download / on_window_event / tray / RunEvent::Exit / HTTP）可能在 `manage()` 之前触发，`state()` 会 panic 且在 C 回调上下文无法 unwind → fail-fast 0xc0000409（实测崩溃根因）。未就绪时降级处理。
- ureq 2.12 API 注意：取响应头用 `resp.header("content-length")`（返回 `Option<&str>`，无 `headers()` 方法），借用需在闭包内转 String。
- 锚点日志：「[download] start/intercepted/cancel/pause/finished」。
