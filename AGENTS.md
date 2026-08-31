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
  - **⚠️ 发布前必须核对：内部构建号（如 2.3.10）不能直接作为正式 Release tag；正确正式版本号 = 次版本 +1、第三位 0（如 2.4.0）。2026-08-31 教训：2.3.10 被误发，已按新规则改发 2.4.0。**
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

## 环境管理（environments.rs，2026-08-31 新增）

- 入口：⋯ 菜单「环境管理」→ 外壳层 `popup-env` 弹层（版本 + Profile 双列）。Rust 侧通过 `GET /env`（只读）与 `/action?name=env-*`（需令牌）交互；长耗时安装/更新走后台任务，`AppState.env_task` 存进度，`window.__dshdEnvChanged` 通知面板刷新。
- **环境管理不使用前端轮询**：面板打开时拉一次 `/env`，之后全部由事件驱动——安装进度/结果用 `__dshdEnvChanged`，切换完成用 `backend-status`（Rust `emit`/`publish` 会同步 eval 到 chrome WebView）。不要加 `setInterval` 轮询。
- **切换环境的全屏 Loading 是注入到 DSH 内容 WebView**（`controls.rs::switch_loading_js` / `switch_loading`），只覆盖内容 WebView 区域、不盖标题栏；面板内另一条「切换中」横幅，后端 `running/external/error/stopped` 状态事件到达后自动收起，《不要》用整窗外壳层弹层做切换遮罩。
- **版本**：每个受管版本安装到 `%APPDATA%\DSH Desktop\versions\<id>\`（`npm install --prefix <dir> @deepseek-ai/dsh@<spec>`），切换只改 `settings.dsh_bin`，**不影响当前正在运行的实例**（下次重启/切换才生效）；全局安装与手动路径只展示/使用，不能由桌面端删除或更新。
- **Profile**：以 `$DSH_HOME/profiles/<name>` 目录为准扫描（读 package.json 的 `dsh.profile.bundles`/dependencies）。新建=创建 `@deepseek-ai/dsh-base` + `@deepseek-ai/dsh-web-app` 的空模板（立即可启动）；复制可带 node_modules（完整现场改造/修复），复制时修正 manifest 的 `name`。
- **⚠️ `/action?name=...` 的其它参数不能叫 `name`**：HTTP 查询解析后 `HashMap` 里 `name` 会被动作名占用，再传 `name=profile名` 会覆盖动作名，导致动作被当成未知 action 直接失效。环境管理的 profile 参数统一用 `p`（`env-profile-use&p=web` 等）；版本用 `id`/`spec`/`label`，没有冲突。
- **当前使用中的版本/Profile 不再显示 使用/重命名/删除 按钮**，Rust 侧同样拒绝（防误操作）。
- **启动命令必须是 `node <bin.js> --profile <name> --no-open --port <port>`**：`web` 只是 `--profile web` 的**别名**，不能与 `--profile <name>` 混用（`dsh --profile x web ...` 会被 Commander 拒绝）；自定义 profile 直接传 web 自身参数即可。`controls.rs::run_install_plugin` 使用 `dsh plugin --profile <name> ...` 的形式（子命令前无 parent `--profile`），同样按当前 settings.profile，不再硬编码 web。
- 首次启动若没有任何版本，后端会改为安装一个「桌面端管理」版本（不再 npm -g）；启动时 `environments::ensure_seed_versions` 会把当前 dsh（全局/手动）登记进列表。
- 插件变更监控随 profile 切换自动重新 watch（每循环读取 settings.profile，变化即重建 watcher）。

## 下载支持（自管下载器 downloads.rs）

- **不要用 WebView2 原生下载**：tauri 的 `DownloadEvent` 只有 Requested/Finished，**没有进度、没有暂停/取消控制**；且实测 WebView2 原生下载链路会崩溃（0xc0000409，崩溃偏移固定，无 [download] 日志）。
- 方案：`on_download` 拦截请求（解析文件名后**返回 `false` 阻止 WebView2**），交给 `downloads.rs` 用 **ureq 流式下载**到系统下载目录，维护进度/暂停/继续/取消（每块检查 AtomicBool 标志），并经 HTTP 控制服务暴露：
  - `GET /downloads` → 任务列表 JSON（id/name/url/file/state/bytes/total/error）
  - `action download-cancel?id=` / `download-pause?id=&on=0|1` / `download-open?id=`（explorer /select）/ `download-delete?id=`（删文件+移记录）/ `download-retry?id=` / `download-browser?id=`（转交系统浏览器并自动取消自管下载，避免重复）
- 注入 JS 提供「下载管理」**右上角下拉卡片**（类 Edge 下载浮层；⋯ 菜单入口 / `__dshdShowDownloads()`）：列表 + 进度条 + 按钮矩阵（下载中=暂停/用浏览器下载/取消；已暂停=继续/取消；完成=打开文件夹/删除；失败/取消=重试/用浏览器下载/删除），**事件驱动刷新（`__dshdDlChanged`；下载线程每 500ms fire 一次进度）**，点击外部/ESC 关闭；样式沿用弹窗经验：独立 id CSS + `[hidden]{display:none!important}`（下拉用 `position:fixed;top:44px;right:12px`，无遮罩）。
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

## 标题栏毛玻璃（Acrylic / Mica）与切页主题自适应（2026-08-29）

- **毛玻璃只能由 DWM 出**：标题栏（chrome）与内容页（dsh/deepseek）是**各自独立的 WebView**，CSS `backdrop-filter` 只能模糊同一 WebView 内的下层内容，**跨 WebView 一律无效**（旧代码写了 blur 但从来没生效）。真毛玻璃 = 窗口 `.transparent(true)` + `set_effects(...)`，由 DWM 在窗口背后合成。
- **Acrylic 与 Mica 的本质区别（选错会被当成 bug）**：
  - `Effect::Acrylic`（DWM backdrop=3）：**实时模糊窗口后面的一切**，包括其它应用窗口——用户说的「透出上面的应用程序」只能靠它。
  - `Effect::Mica*`（backdrop=2）：**只采样桌面壁纸**做柔和底纹，下层窗口完全不参与合成；壁纸不变则标题栏颜色恒定。
  - 现已做成设置项 `windowMaterial`（acrylic / mica / none，默认 acrylic），`apply_window_effect()` 统一按设置 + 主题亮度应用，改完立即生效无需重启。
- **清除材质必须传 `None`**：`set_effects(EffectsBuilder::new().build())` 传的是「空效果列表」，DWM 已设的 backdrop 会**原样保留**（实测切 none 后 backdrop 仍是 2）。正确写法：`set_effects(None::<tauri::utils::config::WindowEffectsConfig>)`，且因窗口是 `transparent(true)`，清除后必须补 `set_background_color` 否则露底。
- 材质按主题亮度选变体：Mica 分 `MicaDark` / `MicaLight`（`0.299R+0.587G+0.114B < 140` 判暗）；Acrylic 无暗亮变体。`set_effects` 失败（Win10 等不支持）时回退 `set_background_color`。
- **Rust 语法坑**：`0.299 * r as f64 + … < 140.0` 会被解析成 `f64<…>` 泛型参数（报 "invalid const generic expression"，一处错误连带 6 个报错）。cast 后比较必须加括号：`0.114 * (b as f64) < 140.0`。
- 标题栏 CSS 用 `color-mix(in srgb, var(--dshd-bg) var(--dshd-tint), transparent)` 把主题色叠在材质上——完全透明会太素、不透明则盖掉材质。浓度由**设置面板的「标题栏透明度」滑块**控制（`settings.json` 的 `titlebarTint`，0-100，默认 18），拖动即时改 CSS 变量做预览，保存才落盘，取消则还原上次保存值。
- **切页时开关状态要由 Rust 回推**：标题栏滑柄原先只在用户点开关本体时本地翻转，从 ⋯ 菜单/托盘等入口切换时不会动（`deepseek_shown` 是 Rust 侧真相源）。`toggle-deepseek` 处理完后 `eval` 调 `window.__dshdSwitchState(on)` 同步滑柄与菜单文案。
- **切页主题自适应**：`AppState` 用**两个主题槽**（`theme_dsh` / `theme_deepseek`），主题桥注入时带 `src` 参数上报到对应槽；`/set-theme` 只在「上报页 == 当前显示页」时才 `push_theme`（隐藏页的上报只入缓存，不打扰当前标题栏）；`toggle-deepseek` 切换后立即用 `current_theme()` 推目标页缓存，无缓存则等桥上报。
- **前景色不能取外部页的 `body` color**：chat.deepseek.com 的 body computed color 实测是 `rgb(128,0,128)`（链接紫），套到标题栏会让文字全变紫。改为**按 bg 亮度推导** fg（暗底 `rgb(230,236,255)` / 亮底 `rgb(20,28,48)`）——标题栏是应用自身 UI，不需要跟页面文字色一致，只需可读且协调。

## 启动流程（2.2.1 修复要点）

- **必须有单实例保护**：`tauri-plugin-single-instance` 且**最先注册**。否则重复启动时第二个实例的 19431 控制服务绑定失败（`os error 10048`），而页面里的 `act()` 全部硬编码发往 19431 → **第二个窗口的按钮会操作第一个窗口**（实测复现）。第二次启动的回调里 `show_main_window` 聚焦已有窗口即可。
- **耗时步骤必须持续上报状态**：首次启动无 dsh 时 `default_install_dsh` 同步阻塞 1~2 分钟（测速 + `npm install -g`），期间若不 publish，状态停在 `idle`，启动页只有一行静态文字，像卡死。现改为传 `report(phase, detail, fetched)` 回调，流式读 npm stderr 的 `http fetch` 行统计已获取包数。`InstallState` 之前是**只有定义、从无构造**的死代码。
- **`BackendStatus` 是 snake_case 序列化**（没加 `rename_all`）：前端读 `next_retry_sec`，写成 `nextRetrySec` 会恒为 undefined（loading.html 的重试倒计时曾因此从不显示）。
- **透明窗口必须隐藏创建**：`transparent(true)` + 立即可见 → 子 WebView 渲染前会闪一下桌面/材质。改为 `.visible(false)`，由外壳层 `act('ping')` 触发 `reveal_main_window()`（幂等），并加 1.5s 兜底防止 chrome 页加载失败导致窗口永不显示；显示前顺带 `on_shell_resize` 校准布局。
- **子 WebView 初始尺寸不能写死**：原先硬编码 `1440x864`，小屏 / DPI 缩放下首帧错位，要等第一次 `Moved`/`Resized` 才被 `relayout_children` 纠正。改为按 `inner_size() / scale_factor()` 实算。
- **启动页也不轮询**：子 WebView 的 Tauri IPC 在生产模式未必可用（capability 的 `windows` 不覆盖子 webview），loading.html 只做**一次** `GET /status` 初始化拉取；此后由 Rust `emit`/`publish` 直接 eval `window.__dshdStatus` 到 `get_webview("dsh")` 推送（与 chrome 一致）。不要恢复 `setInterval` 轮询。

## memos-cloud-dsh-plugin 兼容修复（2026-08 起，本地补丁）

- **背景**：web profile 里的 `@memtensor/memos-cloud-dsh-plugin@0.1.0` 依赖 `@deepseek-ai/dsh-settings` 旧导出（`installSettingsSection`/`settingsNamespace`），dsh 0.1.2-alpha.2 已移除 → **`dsh web` 启动即挂**（`does not provide an export named 'installSettingsSection'`）。用户用 `--profile resolve`（不含该插件）作为临时方案。
- **上游无修复**：npm 最新仍是 0.1.0（2026-08-16）；GitHub `MemTensor/MemOS-Cloud-Dsh-Plugin` 最后提交同日（0.1.0-beta.1），`MemOS-Cloud-OpenClaw-Plugin` 已删除 `packages/dsh` 子包。故在 `assets/memos-cloud-dsh-plugin-fixed/` 固化补丁，详见其 `PATCH-NOTES.md`。
- **补丁内容**：`installSettingsSection(ctx,...)` → `ctx.inject(["settings"], (sctx) => sctx.settings.installSection(ctx, ...))`；`settingsNamespace(ns)` → 直接字符串。其余 peer 导入（launchEnvironmentOf/isAppendSurfaceEvent/credentialRef/createUserMessage）当前版本仍存在，未动。
- **部署**：`pwsh scripts\apply-memos-plugin-fix.ps1`（复制到 `~/.dsh/profiles/web/vendor/` + package.json 改 `link:` 依赖 + `pnpm install`）。补丁会跟随 `link:` 依赖在重建 node_modules 后保留；`dsh plugin add/remove` 不破坏它，但**再次 `dsh plugin add @memtensor/memos-cloud-dsh-plugin`（registry 版本）会替换 link: 依赖**，需重跑脚本。
- **注意**：本地验证 web profile 完整启动若与正在运行的本机会话冲突，会报 `task-board ledger is already owned by process <pid>`（task-board 单实例锁），用 `--patch` 临时 overlay 禁用 `web-ui-task-board` 条目即可验证（`--patch` 必须放在 `--no-open` 等透传参数之前）。

## 会话协作约定

- 记忆召回与写回由 MemOS Cloud 插件自动完成，**不要**手动调用 `mcp__memos-mcp__*`（仅主动管理记忆时按需使用）。
- 用用户的语言（中文）回复正文；工具日志/命令输出不受此约束。