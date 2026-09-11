---
name: startup-flow
description: >
  应用启动流程：单实例保护、透明窗口隐藏创建、启动页状态上报、子 WebView 尺寸。
---

# 启动流程

- **必须有单实例保护**：`tauri-plugin-single-instance` 且**最先注册**。否则重复启动时第二个实例的 19431 控制服务绑定失败（`os error 10048`），而页面里的 `act()` 全部硬编码发往 19431 → **第二个窗口的按钮会操作第一个窗口**（实测复现）。第二次启动的回调里 `show_main_window` 聚焦已有窗口即可。
- **耗时步骤必须持续上报状态**：首次启动无 dsh 时 `default_install_dsh` 同步阻塞 1~2 分钟（测速 + `npm install -g`），期间若不 publish，状态停在 `idle`，启动页只有一行静态文字，像卡死。现改为传 `report(phase, detail, fetched)` 回调，流式读 npm stderr 的 `http fetch` 行统计已获取包数。`InstallState` 之前是**只有定义、从无构造**的死代码。
- **`BackendStatus` 是 snake_case 序列化**（没加 `rename_all`）：前端读 `next_retry_sec`，写成 `nextRetrySec` 会恒为 undefined（loading.html 的重试倒计时曾因此从不显示）。
- **透明窗口必须隐藏创建**：`transparent(true)` + 立即可见 → 子 WebView 渲染前会闪一下桌面/材质。改为 `.visible(false)`，由外壳层 `act('ping')` 触发 `reveal_main_window()`（幂等），并加 1.5s 兜底防止 chrome 页加载失败导致窗口永不显示；显示前顺带 `on_shell_resize` 校准布局。
- **子 WebView 初始尺寸不能写死**：原先硬编码 `1440x864`，小屏 / DPI 缩放下首帧错位，要等第一次 `Moved`/`Resized` 才被 `relayout_children` 纠正。改为按 `inner_size() / scale_factor()` 实算。
- **启动页也不轮询**：子 WebView 的 Tauri IPC 在生产模式未必可用（capability 的 `windows` 不覆盖子 webview），loading.html 只做**一次** `GET /status` 初始化拉取；此后由 Rust `emit`/`publish` 直接 eval `window.__dshdStatus` 到 `get_webview("dsh")` 推送（与 chrome 一致）。不要恢复 `setInterval` 轮询。
