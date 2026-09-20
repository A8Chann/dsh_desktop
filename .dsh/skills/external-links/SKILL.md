# 外链打开（交给系统默认浏览器）

## 症状

- dsh Chat 页里**模型回答中的网页链接点了没反应**；插件市场/侧边栏等用 `window.open` 的地方同理。
- DeepSeek 聊天页（chat.deepseek.com 子 WebView）里的链接同样没反应。
- 右键「在新窗口打开」/ Ctrl+点击 也没用。

## 根因：wry 把 WebView2 的 NewWindowRequested 静默吞掉

- wry（0.55.x，`src/webview2/mod.rs:696-783`）在 Windows 上**无条件**注册 `NewWindowRequested` 处理器；
  没配置 `new_window_req_handler` 时只执行 `args.SetHandled(true)`——既不新建窗口也不放行 → **所有新窗口请求被吞**。
- `target="_blank"` 锚点与 `window.open(...)` 都走这条事件（dsh 前端外链就是 `window.open(url, "_blank", "noopener")`）。
- Tauri v2 从 2.2 起暴露 `WebviewBuilder::on_new_window`（本地 2.11.5 见 `src/webview/mod.rs:585`），
  返回 `tauri::webview::NewWindowResponse::{Allow, Create, Deny}`。

## 做法（`src-tauri/src/main.rs` + `controls.rs`）

- `controls::external_link_handler(app.handle().clone())` 挂在**两个内容 WebView**（`dsh` 与 `deepseek`）上：
  - `http`/`https` → `tauri_plugin_opener::OpenerExt::opener(&app).open_url(url, None::<&str>)`（系统默认浏览器）+ 记 `[link]` 日志，返回 **Deny**；
  - 其它 scheme（`about:blank`、`file:` 等）→ 一律 **Deny**，维持改动前行为，不引入新窗口。
- **不要**顺手加 `on_navigation` 拦截：页面内同窗口导航（登录 / OAuth 回流）不能碰。
- opener 插件已在 `Cargo.toml` 里（托盘「在系统浏览器打开」用它），Rust 侧调用**不需要 ACL/权限**。

## 关键坑：dsh-better-sidebar 会自己接管 http 外链（先于 wry）

- `dsh-better-sidebar` 在 document 上注册 **capture 阶段** click 监听（`registerLinkInterception`，
  `lib/client.js` 里搜 `shouldInterceptLink` / `takeoverEnabled`）：命中就 `preventDefault()` 再把 URL 塞进
  它自己的「浏览器」侧边栏标签（sandbox iframe）→ **wry 的 NewWindowRequested 根本不会触发**。
- 默认 prefs（`SIDEBAR_PREFS_DEFAULTS`）：`browserInterceptLinks: true`、`browserInterceptHttp: true`、
  **`browserInterceptHttps: false`** —— 默认只接管 **http**，不接管 https。模型回答/网页链接基本都是 https，
  所以默认走我们这条链路；若用户手动打开 https 接管，则 https 也会被抢进侧边栏。
- 被接管后 iframe 常因 `X-Frame-Options` / `frame-ancestors` 空白或显示「禁止嵌入」面板；
  面板里的「在浏览器中打开」按钮走 `window.open(embedBlocked, "_blank")`——本轮修复后它会真的打开浏览器。
- **Ctrl/Cmd/Shift/Alt + 点击**会绕过接管（插件刻意留的后门）→ 同样落到我们的处理器。
- 想彻底关掉接管：侧边栏设置里的 `browserInterceptLinks`（「拦截外链在侧边栏打开」）。

## 验证套路（2026-09-20 实测通过）

1. **改 identifier 才能起第二个实例**：`tauri.conf.json` 的 `identifier` 临时改成 `io.dsh.desktop.linktest`
   （单实例锁按 identifier 命名；不改的话第二个进程只会激活已有窗口然后退出）。**验证完必须改回**。
2. **临时加远程调试**：给 dsh WebView 加
   `.additional_browser_args("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --remote-debugging-port=9223")`。
   ⚠️ 环境变量 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` **无效**：wry 自己通过 API 传了 additional_browser_args，env 被忽略。
3. **后台 job 跑 debug exe**：前台命令结束时 harness 会把子进程树一起杀掉（实测进程秒退，只剩 msedgewebview2 残留）。
4. `http://127.0.0.1:9223/json/list` 里**只暴露 dsh 内容页**一个 page target（chrome.html / 隐藏的 deepseek 页不在列表里）。
5. 起两个本地监听收集证据：`http` 服务（记录 GET）与**裸 `net` 服务**（记录 TLS ClientHello，用来验证 https 路径）。
6. 用 **CDP `Input.dispatchMouseEvent`（mousePressed+mouseReleased）真点**：
   `Runtime.evaluate` 里 `a.click()` 是 untrusted，会被 popup 拦截（假阴性）；
   合成锚点也要先 `document.elementFromPoint` 确认没被遮挡。
7. **验证 https 路径必须用 `https://` 链接**：用 `http://` 会被 better-sidebar 接管，得到假阴性（本次踩过）。
8. 判定：`main.log` 出现 `[link] 外部链接交由系统默认浏览器打开: <url>` **且**本地监听收到连接（TLS ClientHello）。
9. 第二个实例对在跑的正式版**无害**：日志会打 `[backend] 接管外部实例 pid=… （退出时不杀）`，19431 绑定失败也无所谓；
   收尾记得 `Stop-Process -Name dsh-desktop`，并删掉 `%LOCALAPPDATA%\io.dsh.desktop.linktest`。

## 相关文件

- `src-tauri/src/main.rs`：两个内容 WebView 的 `on_new_window` 挂载点。
- `src-tauri/src/controls.rs`：`external_link_handler`（紧邻 `intercept_download`）。
