# DSH Desktop（Tauri 桌面壳）开发指南

DeepSeek Harness 的 Windows 桌面端：Tauri v2 + WebView2，内嵌 dsh web GUI，自动拉起后端、自绘标题栏、托盘、本地 HTTP 控制服务。以下是从历次开发/排障中沉淀的通用逻辑与约定。

## 构建与发布

- 构建：`cd src-tauri && cargo build --release`（产物 `src-tauri/target/release/dsh-desktop.exe`）。
- 构建需 `danger-full-access`：cargo 要访问工作区外的 `~/.cargo` 缓存、rustc/link 工具链与 Temp 目录；产物路径本身在工作区内。
- 发布：把 release exe 复制为 `dist/DSH-Desktop-<version>-tauri.exe`。
- **不要把 exe 复制到用户桌面**——桌面部署由用户自己完成，助手只更新 dist。
- **版本约定（用户指定，2026-08-29 更新）**：
  - **每次打包（构建新 exe）第三位 +1**：`2.0.2 → 2.0.3 → 2.0.4 → …`（未发版迭代无限递增）；
  - **正式发版（打 tag/Release）除非另有说明：次版本 +1、第三位归 0**（如 2.0.3 迭代发版为 **2.1.0**；发版后下一轮迭代再继续第三位 +1）；
  - 版本号需同步：`package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、`src-tauri/frontend/chrome.html`（badge + 「封装」两处）。

## 本地 HTTP 控制服务（127.0.0.1:19431，仅本机、无鉴权）

WebView2 页面无法直接调 Tauri API（生产模式），统一走 HTTP 控制服务（`controls.rs::start_http_server`）：

- `GET /status` → 后端状态 JSON（state/url/port/pid/owned/error/nextRetrySec）
- `GET /action?name=<action>` → 窗口/后端操作，支持：
  - `min` / `max` / `drag` — 窗口控制（必须经 `run_on_main_thread`）
  - `close` — **弹出「退出/缩小到托盘」选择框，不关窗**
  - `min-tray` — 隐藏窗口到托盘
  - `quit` — 先 `kill_owned()` 杀后端进程树，再退出（见下）
  - `restart` — 重启后端（就绪后自动刷新页面，逻辑统一在 `Backend::restart` 内）
  - `reload` / `browser` / `ping`

## 窗口关闭与「退出/缩小到托盘」弹窗（踩坑总结）

1. **✕ 按钮不能走 `win.close()`**：程序化关闭**不触发** `CloseRequested` 事件，Rust 侧拦截会被整体绕过（结果：窗口直接关、应用退出、后端成孤儿进程）。✕ 在注入 JS 里直接调 `window.__dshdShowCloseDialog()`（自绘弹窗）。
2. **Alt+F4 / 任务栏关闭**：`main.rs` 的 `.on_window_event` 拦截 `CloseRequested` → `api.prevent_close()` + `controls::show_close_dialog()`；`AppState.force_exit=true` 时放行（弹窗/菜单/托盘已明确选择退出）。
3. **弹窗 CSS 必须用弹窗自身独立 id 选择器**：弹窗挂在 `document.body` 下（不是标题栏 `#BAR_ID` 的后代），如果写成 `'#BAR_ID .dch-overlay'` 会**样式全部失配**，弹窗退化成页面底部一行裸 HTML 文本。
4. **`hidden` 属性会被 CSS 覆盖**：overlay 设了 `display:grid`（作者样式优先于 UA 的 `[hidden]{display:none}`），必须显式补 `'#id[hidden]{display:none!important}'`，否则弹窗永远显示无法隐藏。
5. **退出清理不能依赖 `RunEvent::Exit` 回调**：`AppHandle::exit()` 内部 `request_exit` 在部分线程/时机下会失败并**直接 `std::process::exit`，Exit 事件根本不触发**（实测两次 quit 均无清理日志）。正确做法：所有退出路径先同步 `Backend::kill_owned()`，再 `app.exit(0)`；`RunEvent::Exit` 回调仅作兜底。
6. **`kill_owned()` 规则**：先 `stop.store(true)` 阻止管理线程重拉，再 taskkill `/T /F` 杀自有后端进程树；**外部接管实例（`owned=false`）不杀**；状态里没记到 pid 时按端口反查（`find_pid_by_port`）兜底。

## 后端管理（backend.rs）

- 单管理线程 `run_loop` 循环：`spawn_own`（拉起 `node dsh bin.js web --no-open`）/ `adopt_external`（端口被 dsh 占用则接管，轮询 1s）/ `fail`（退避重试 1s→30s 封顶）。
- **重启必须「先杀后 join」**：管理线程阻塞在子进程 stdout 的 `read_line` 上，子进程不退出则 `read_line` 永不返回，直接 `join()` 会**永久死锁**（重启失效的根因）。`restart()` = `kill_owned()` → `join()` → `start()` → 后台轮询到 running 后 `location.reload()`。
- 退避 sleep 按 **1 秒切片**并每片检查 `stop`，保证重启/退出能及时打断（最长 30s 的整段 sleep 会让 join 卡住）。
- 状态机：idle / starting / running / external / error / restarting / stopped，经 `backend-status` 事件 + `win.eval` 推送到注入标题栏。

## 下载支持（自管下载器 downloads.rs）

- **不要用 WebView2 原生下载**：tauri 的 `DownloadEvent` 只有 Requested/Finished，**没有进度、没有暂停/取消控制**；且实测 WebView2 原生下载链路会崩溃（0xc0000409，崩溃偏移固定，无 [download] 日志）。
- 方案：`on_download` 拦截请求（解析文件名后**返回 `false` 阻止 WebView2**），交给 `downloads.rs` 用 **ureq 流式下载**到系统下载目录，维护进度/暂停/继续/取消（每块检查 AtomicBool 标志），并经 HTTP 控制服务暴露：
  - `GET /downloads` → 任务列表 JSON（id/name/url/file/state/bytes/total/error）
  - `action download-cancel?id=` / `download-pause?id=&on=0|1` / `download-open?id=`（explorer /select）/ `download-delete?id=`（删文件+移记录）/ `download-retry?id=` / `download-browser?id=`（转交系统浏览器并自动取消自管下载，避免重复）
- 注入 JS 提供「下载管理」**右上角下拉卡片**（类 Edge 下载浮层；⋯ 菜单入口 / `__dshdShowDownloads()`）：列表 + 进度条 + 按钮矩阵（下载中=暂停/用浏览器下载/取消；已暂停=继续/取消；完成=打开文件夹/删除；失败/取消=重试/用浏览器下载/删除），800ms 轮询，点击外部/ESC 关闭；样式沿用弹窗经验：独立 id CSS + `[hidden]{display:none!important}`（下拉用 `position:fixed;top:44px;right:12px`，无遮罩）。
- **全部 `app.state::<T>()` 一律用 `try_state`**：回调（on_download / on_window_event / tray / RunEvent::Exit / HTTP）可能在 `manage()` 之前触发，`state()` 会 panic 且在 C 回调上下文无法 unwind → fail-fast 0xc0000409（实测崩溃根因）。未就绪时降级处理。
- ureq 2.12 API 注意：取响应头用 `resp.header("content-length")`（返回 `Option<&str>`，无 `headers()` 方法），借用需在闭包内转 String。
- 锚点日志：「[download] start/intercepted/cancel/pause/finished」。

## 日志与排障

- 日志：`%APPDATA%\DSH Desktop\logs\main.log`（UTF-8；PowerShell 控制台按 GBK 显示会乱码，读文件用 `-Encoding UTF8`）。
- 关键锚点：「==== 重启后端」「==== 终止自有后端进程树 pid=」「[http] action:」「==== DSH Desktop 退出」「[close] 页面不可用」。
- 排障先看日志时间线，能直接区分「action 没走到 / 回调没执行 / pid 没找到 / 杀进程失败」。

## Windows toast 通知图标（插件变更提示，踩坑总结）

- **tauri-plugin-notification 在 Windows 上无法设置 toast 小图标**：builder 的 `.icon()` 只会写 notify-rust 的 `icon` 字段，而 notify-rust 的 Windows 构建（`build_toast`）**从不读取该字段**；toast 左上角小图标由 AppUserModelID（AUMID）对应实体的图标决定（`CreateToastNotifierWithId(aumid)`）。
- **非打包应用 toast 图标的官方途径 = 开始菜单快捷方式**（带 `System.AppUserModel.ID`，图标指向 exe）：实测 `appLogoOverride`（`file:///` 与 `http://` 源）、注册表 `IconUri` **全部不被 toast 平台采用**，一律回退通用「文件」图标。
- **Windows 首次用该 AUMID 发 toast 时会自动创建一个「空壳」快捷方式**（指向随机 `%TEMP%\xxx\` 目录、IconLocation 为空）→ 这就是「默认文件图标」的真正来源；必须用真实 exe 覆盖它（TargetPath = 当前 exe、IconLocation = exe,0、AUMID 属性）。
- **⚠️ 进程内 Rust COM 创建 .lnk（IShellLinkW + IPropertyStore + 手写 PROPVARIANT）→ 堆损坏崩溃 0xc0000374（ntdll.dll，Windows 事件日志 Application Error 1000）**：第一次可能“成功”（快捷方式写出），但内存已坏，之后**每次 toast 触发或下次启动必崩**（「程序莫名其妙关」/「启动打不开」同根因）。**必须用隔离进程**：启动时 spawn `powershell.exe`（`-WindowStyle Hidden` + `CREATE_NO_WINDOW`）执行经典 C# 片段（`win_toast.rs::ensure_start_menu_shortcut`，已在 win_toast.rs 内固化）。
- C# 片段要点：IShellLinkW 设 Target/WorkingDirectory/IconLocation；IPropertyStore 写 AUMID（PKEY `{9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3}` pid=5，VT_LPWSTR，`Marshal.StringToHGlobalUni` + `FreeHGlobal`）→ Commit → IPersistFile::Save。**不要先 IPersistFile::Load**（只读打开 → SetValue/Commit 报 STG_E_ACCESSDENIED）；**C# 必须用单引号 here-string `@'…'@`**（`@"…"@` 会做 PowerShell 变量插值把代码改坏）。
- 保留项：注册表 AUMID（DisplayName + IconUri，无空格路径）+ `Toast::icon()` appLogoOverride 正斜杠路径作双保险；非 Windows 平台仍走 tauri-plugin-notification（`backend.rs::on_plugin_change` 的 `cfg` 分支）。

## 主题桥采样与「标题栏变黑」（踩坑总结）

- 标题栏底色链路：内容页 `theme_bridge_js` 采样 html/body 的**真实渲染色** → `/set-theme` → Rust 存 `state.theme` 并 `push_theme`（chrome WebView 设 `--dshd-bg/--dshd-fg` + `set_background_color`）。标题栏本身透明，颜色 = 窗口底色。
- **`--dsw-alias-*` 只是皮肤（skin.css）里的主题变量；官方默认皮肤下 html/body 背景是透明的**（底色由 `#root` / `[data-dsh-frame]` 等面板绘制）。桥若只采样 html/body，会全部透明 → 落到硬编码回退 `#0b1220`（深蓝黑）→ 标题栏看起来「变纯黑」，且与官方浅色界面严重不匹配。
- 皮肤激活时皮肤给 html 显式背景色（蓝色幻想 `#e8ecf5`/`#101624`）→ 采样正常 → 标题栏贴合皮肤。所以「切回原始皮肤 → 标题栏变黑」的根因是**桥的回退值**，不是皮肤。
- **修复**：html/body 透明时继续向下采样 `[data-dsh-frame]` / `#root` / `[data-dsh-app]` 的真实渲染色；最终兜底也按 `data-ds-dark-theme` 区分（暗 `#0b1220` / 亮 `#ffffff`）。
- 注意「刷新后好了、不刷新坏」的迷惑性：刷新后走的是 boot 页/重新加载的初采样（可能恰好正常），不刷新的运行时切换才暴露真实采样结果——**要看 `/theme` 接口的最终值**（`http://127.0.0.1:19431/theme`），不要凭肉眼时序下结论。

## 会话协作约定

- 记忆召回与写回由 MemOS Cloud 插件自动完成，**不要**手动调用 `mcp__memos-mcp__*`（仅主动管理记忆时按需使用）。
- 用用户的语言（中文）回复正文；工具日志/命令输出不受此约束。