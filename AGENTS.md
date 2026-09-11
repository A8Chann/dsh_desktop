# DSH Desktop（Tauri 桌面壳）开发指南

DeepSeek Harness 的 Windows 桌面端：Tauri v2 + WebView2，内嵌 dsh web GUI，自动拉起后端、自绘标题栏、托盘、本地 HTTP 控制服务。以下是从历次开发/排障中沉淀的通用逻辑与约定。

## 开发环境（tauri dev 快速迭代，用户首选）

- **日常开发/迭代一律用 `npx --yes @tauri-apps/cli@latest dev`（项目根运行），不要每次 `cargo build --release`**——后者是发布打包流程，只在发版时用（用户 2026-09-01 明确：改一次东西就打一次包"怎么行"）。
- 工作方式：
  - debug 编译（首次约 1~2 分钟拉 CLI + 全量编译；之后增量编译更快）；
  - `Watching D:\HTML\DSH_Desktop\src-tauri for changes...` 监视 Rust 源码，**保存 `.rs` 自动重编译 + 重启应用**；
  - 前端静态文件（`src-tauri/frontend/*.html`）dev 模式**从磁盘直接加载**，改完**刷新窗口页即生效**（无需编译）。
- 产物：`src-tauri/target/debug/dsh-desktop.exe`（debug 版，体量/性能与 release 不同，仅开发用）。
- 验证接口与 release 一样：`http://127.0.0.1:19431/status`；后端仍会接管外部 dsh 实例（`probe_port` 修复后 401 认证的 dsh 也能正确识别）。
- 前置：registry 已配 npmmirror（工作区 `.npmrc`）；tauri CLI 走 npx 按需拉取，无需全局安装。
- 进程管理：tauri dev 是长驻进程（后台 job），**保持运行即开发环境活跃**；停掉（Ctrl+C/退出）则 dev 环境停了，重跑上面命令即可。

## 构建与发布

- 构建（仅发版打包）：`cd src-tauri && cargo build --release`（产物 `src-tauri/target/release/dsh-desktop.exe`）。
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
- **部署（历史）**：`pwsh scripts\apply-memos-plugin-fix.ps1`（复制到 `~/.dsh/profiles/web/vendor/` + package.json 改 `link:` 依赖 + `pnpm install`）。补丁会跟随 `link:` 依赖在重建 node_modules 后保留；`dsh plugin add/remove` 不破坏它，但**再次 `dsh plugin add @memtensor/memos-cloud-dsh-plugin`（registry 版本）会替换 link: 依赖**，需重跑脚本。
- **✅ 2026-09-10 已回归上游：上游 `0.1.1`（npm 2026-09-07 发布）改用新 API `settings.register(ns, schema, {base, validate})`，不再导入 `installSettingsSection`/`settingsNamespace`**。web profile 依赖已改为 `"@memtensor/memos-cloud-dsh-plugin": "^0.1.1"`（registry 包），`vendor/memos-cloud-dsh-plugin` + `scripts/apply-memos-plugin-fix.ps1` + `assets/memos-cloud-dsh-plugin-fixed` **仅作历史兜底，勿再执行**（执行会把依赖改回 `link:`）。命名空间仍是 `memos-cloud`，`~/.dsh/settings.yaml` 的 `memos-cloud:` 配置（apiKeyEnv/userId）原样沿用，无需迁移。
- **注意**：本地验证 web profile 完整启动若与正在运行的本机会话冲突，会报 `task-board ledger is already owned by process <pid>`（task-board 单实例锁），用 `--patch` 临时 overlay 禁用 `web-ui-task-board` 条目即可验证（`--patch` 必须放在 `--no-open` 等透传参数之前）。

## web profile 插件版本与升级（2026-09-10，dsh 0.1.5-rc.1）

- **升级命令**（用受管版本的 dsh，别用全局 0.1.2）：`node "%APPDATA%\DSH Desktop\versions\<id>\node_modules\@deepseek-ai\dsh\lib\bin.js" plugin --profile web <pnpm 参数>`，例如 `... plugin --profile web add "@linxin666/dsh-web-all@0.3.20"`、`... plugin --profile web up --latest`。`dsh plugin` 只是 pnpm 转发器，并在成功后按**已安装状态**重建 `dsh.profile.bundles`。
- **⚠️ pnpm 11 默认 `minimumReleaseAge` 24h（供应链冷静期）**：插件的「兼容新 dsh」版本往往和 dsh 同一天发布，会被静默降级到能通过冷静期的旧版本（日志里是 `[... was updated to X, not Y, to match the version preferred by your manifests]`）。要在 `~/.dsh/profiles/web/pnpm-workspace.yaml` 的 `minimumReleaseAgeExclude` 里按 `包名@版本` 逐条放行（该 profile 一直这么用），否则 `--latest` 拿不到真正的新版本。
- **`--profile web add` 里不要写 `^`**：dsh 在 Windows 用 `shell: true` 转发给 cmd.exe，`^` 是 cmd 转义符会被吃掉。给精确版本号（如 `@0.3.20`）即可，pnpm 自己会按 manifest 风格写入范围。
- **兼容性判断三件套**：① 插件 package.json 的 `dsh.engines.dsh`（如 `>=0.1.5-rc.1`）与 `dsh.compatibility.dshReleases`；② 宿主是否还导出插件 import 的 `@deepseek-ai/*` 符号；③ 客户端 `require()` 的模块是否在「平台种子表」里（0.1.5-rc.1 的种子 = react / react-dom / `@deepseek-ai/cordis` / `dsh-client-store` / `dsh-client-ui-slots` / `dsh-client-ui-primitives` / `dsh-client-ui-dockkit`，来自 `dsh-web-frontend/dist` 的 staticModules）。
- **验证办法（无浏览器也能验）**：另起端口启动 `node <bin.js> --profile web --no-open --port 3099`；`curl -sL -c cookies "http://127.0.0.1:3099/?token=<打印的 token>"` 拉首页 → 触发客户端模块组装（失败会在宿主日志里报 `client ... failed to compose` / `client bundles not found`）；首页里的 `/plugins/??<id>/client.js&rev=...` 组合 URL 用 cookie 请求应得 200 且体积正常。宿主侧看启动日志有没有 `failed to import loader entry`。

## Command Code（commandcode）provider 与思考强度（2026-09-11）

- **症状**：用 `@mars-sea/dsh-commandcode-provider` 接 `deepseek/deepseek-v4.1-flash` 时，模型选择器**没有思考强度选项**；而官方 Command Code 该模型有 `low`/`high`/`max` 三档。
- **根因**：插件把「可选思考强度」写死在快照表 `KNOWN_EFFORTS`（镜像官方 CLI 的 `ZA` 模型表），`resolveModel` 只有命中该表才返回 `reasoning.efforts`。本机装的是 **0.10.2**，其快照停在更早的 CLI 版本，表里没有 `deepseek/deepseek-v4.1-flash`。官方 command-code 在 **1.53.0**（2026-09-10）才加入该模型（`deepseek/deepseek-v4.1-flash`，Go 档、Vision、1M 上下文、efforts `low`/`high`/`max`），插件 **0.10.3** 同步。
- **修法**：升级插件（`0.10.3` 起即可，本机升到 `0.10.5`）——
  `node "%APPDATA%\DSH Desktop\versions\v18d385b479a67f08\node_modules\@deepseek-ai\dsh\lib\bin.js" plugin --profile web add "@mars-sea/dsh-commandcode-provider@0.10.5"`
  ⚠️ 该版本在 24h 冷静期内，需先在 `~/.dsh/profiles/web/pnpm-workspace.yaml` 的 `minimumReleaseAgeExclude` 放行 `@mars-sea/dsh-commandcode-provider@0.10.3/0.10.4/0.10.5`。**改完必须重启后端**（宿主模块在进程内存里）。
- **免 UI 的单元级验证**（不用起第二个实例、不碰用户运行中的会话）：直接 import 插件并打桩调用 `resolveModel`——
  ```js
  const { CommandCodeAdapter } = await import('file:///…/@mars-sea/dsh-commandcode-provider/lib/index.js')
  const a = Object.create(CommandCodeAdapter.prototype)
  a.catalog = [{ id: 'deepseek/deepseek-v4.1-flash', name: 'x', contextWindow: 1e6, maxTokens: 131072 }]
  a.deps = { options: () => ({}) }
  console.log((await a.resolveModel('commandcode', 'deepseek/deepseek-v4.1-flash')).reasoning)
  // → { efforts: [{id:'low'…},{id:'high'…},{id:'max'…}] }
  ```
  打桩 `a.catalog` 是关键：否则 `resolveModel` 会回落 `loadCatalog()` 去发网络请求。
- **注意事项**：`dsh plugin add` 输出里的 `Packages: +1 -12` 是 pnpm 清理陈旧存储，**不是删插件**；核对 `node_modules` 各包版本 + `package.json` 的 `dependencies`/`dsh.profile.bundles` 即可确认完整。

## dsh-cost-meter 的 CommandCode 额度面板（2026-09-11）

- **面板早已内置**：`dsh-cost-meter@1.7.19` 起 `commandcode` 就是第 7 家 Coding Plan（`lib/coding-plans.js` 的 `CODING_PLAN_PROVIDERS` / `CODING_PLAN_ENDPOINTS` / `parseCommandCodeCredits`，`store.js` 的默认值与 `SECRET_REF_MAP`，`client.js` 的注册表与短标签 `CC`）。**无需开发**，默认 `enabled:false, display:'settings'` 所以侧边栏看不到。
- **启用方式（推荐走 UI）**：设置 → 费用 → Coding Plan → **CommandCode** → 勾「启用额度查询」+ 显示位置选「主页面侧边栏 / 两者」。Key 自动从 DSH 凭据库的 `COMMANDCODE_API_KEY` 发现（`SECRET_REF_MAP` 里 `'codingPlans.commandcode' → 'COMMANDCODE_API_KEY'`），不必手填。
- **⚠️ 不要在实例运行时改它的配置文件**：全部配置在 `$DSH_HOME/storages/cost-meter/ledger.json`（**不在** `settings.yaml`，也没有 `dsh-cost-meter` 设置段）。宿主在 `Store.flush()` 时把**内存里的 config** 原子写回该文件（`store.js:2103`），而本会话每产生一次用量就会触发 flush → **手改的 `enabled` 会在 1~2 分钟内被覆盖回 `false`**（实测 02:56 改、02:58 就被抹掉）。要改文件只能趁进程没在跑。
- **端点与解析实测**（2026-09-11，真实 Key）：`GET https://api.commandcode.ai/alpha/billing/credits`（Bearer + `x-command-code-version` + `x-cli-environment: production`）→ `{credits:{monthlyCredits}, windowLimits:{fiveHour:{used,cap,resetAt}, weekly:{…}}}`；`parseCommandCodeCredits` 输出 `fiveHour.percent 1.7 / weekly.percent 0.7 / monthly.text 余额 $69.76`，与 OpenCode Go 卡片同构（5h / 周 + 月度余额）。
- **验证套路**（无需 UI）：直接跑插件自己的解析器对真实响应做断言——
  ```js
  const cp = await import('file:///…/dsh-cost-meter/lib/coding-plans.js')
  cp.parseCommandCodeCredits(payload, { now: Date.now() })
  ```
- **注意**：`CODING_PLAN_PARSERS` **未导出**，解析器是按名字逐个导出的（`parseCommandCodeCredits` 等）；`CODING_PLAN_ENDPOINTS`/`CODING_PLAN_PROVIDERS`/`CODING_PLAN_PROVIDER_IDS` 才在导出表里。

## ⭐ Coding Plan 图框与 Go 图框「不同款」＝ upstream 名不副实（2026-09-11，已本地补丁）

- **症状**：README 写「侧边栏卡片**与 Go 额度同款**」，但 CommandCode 卡片 256×106、Go 卡片 256×90，**版式根本不同**。
- **根因**：侧边栏聚合处按厂商分派**两个不同组件**——`x.map(A=>A==="minimax"?e(fn,…):e(xn,{id:A,…}))`，只有 MiniMax 走专用组件，**其余 9 家（含 commandcode）都走通用组件 `xn`**，而 `xn` ≠ Go 的 `Vt`：
  | | Go `Vt` | Coding Plan `xn` |
  |---|---|---|
  | 标题 | `cm-bbox-head`：标题与主窗口%同一行 | `cm-mm-title`：标题**独占一行** |
  | 进度条 | `cm-bbox-bar` **全宽** | `.cm-mm-row` 内短条 |
  | 次要窗口 | `cm-bbox-line`：`周 28% · 月 99%` | 每窗口各自一行「标签+条+%」 |
  | 重置时间 | `cm-bbox-line` | **完全没有** |
  隐藏差异：`xn` 外层多一个 `cm-mm` 类，`.cm-mm{gap:4px}` 覆盖 `.cm-bbox{gap:6px}` → 行距差 2px。
- **上游没修**：**1.7.20（npm 最新）与 1.7.19 的 `xn` 逐字节同构** → 升级插件解决不了。`client.js` 是 esbuild 产物、`scripts/` 未随包发布 → 打本地补丁。
- **补丁**：`scripts\cost-meter-plan-box.mjs`（`--check`/`--revert`，备份 `lib/client.js.orig-plan-box`），改 2 处：① 正文换成 Go 四段式；② 外层 className 去掉 `cm-mm`（对齐 `gap`）。
- **⚠️ 锚点必须唯一**：`"cm-bbox cm-mm clickable"` 出现 **3 次**（`vn`/`xn`/`Nn` 三个图框），② 的锚点须带 `+(M==="ok"?"":" "+M)`（`M` 只在 `xn` 作用域内）才唯一。**写这类补丁先数 `split(needle).length-1`。**
- **验证**：`Page.reload {ignoreCache:true}` 后两卡片逐项一致（standard `256×90` = `256×90`、compact `256×78` = `256×78`、simple `35` vs `36` 为文字宽度舍入）；DOM 子节点同为 `cm-bbox-head/bar/line/line` 四段；控制台零 error/exception；`compact`/`simple`/`rail` 均不破版。
- **已知残留（有意未改）**：rail（`wide===false`）仍不一致——Go 只显示主窗口一个%（`0%`），Coding Plan 显示前两个（`3% 1%`），是上游多窗口设计（`w=P.slice(0,2)`）。
- **生效**：只需重载内容页（F5），**不用重启后端**。⚠️ `dsh-cost-meter` 升级会覆盖 `client.js` → 重跑脚本（幂等）。
- **CDP 验证工具**（本次沉淀到 `scripts\cdp\`：`eval.mjs <expr文件>` / `shot.mjs <out.png> x y w h` / `reload.mjs`）。⚠️ **表达式一律写进 `.js` 文件**：内联表达式会被 PowerShell 引号处理搞坏（`SyntaxError: Invalid or unexpected token`）；`shot` 的 clip 用 4 个数字参数，别传 JSON。

## 皮肤补丁 patches.css 与「输入栏按钮样式被搞坏」（2026-09-10）

- **⭐ 配色基准（用户明确要求）**：给这套皮肤加/改任何颜色时，**一律以「左侧边栏」`[data-pane="sidebar"]` 的 computed `backgroundColor` 为基准取色**（当前 `rgba(242,245,250,.75)`），顶部导航栏、右侧栏等都从它取值，**不要自己拍纯白或凭感觉配**；浓度也在它基础上调。深色主题同法重测。

- **⭐ patches.css 的存档与上游更新（2026-09-10 建立）**：
  - 这个文件是**市场安装件**：`~/.dsh/skins/blue-fantasy/` 来自 dsh-market，安装时留下的 `dsh-market.provenance.json` 记着**每个文件的 sha256**；从市场**更新/重装皮肤会把 `patches.css` 整文件覆盖**。而上游改它时**不 bump `skin.json` 的 version**（至今 `0.2.0`），市场因此**不会**提示「有更新」——只能靠 sha256 比对发现。
  - 仓库存档：`assets/skins/blue-fantasy/`（成品 `patches.css`、上游基准 `upstream/patches.base.css`、上游坐标与沿革 `upstream/notes.json`、安装时 provenance 快照），配套脚本 `scripts/skin-patches.ps1`：`status` / `save` / `restore` / `check` / `merge [-Apply]`。另有一份时间戳快照在 `~/.dsh/_skin-backups/blue-fantasy/`。
  - 上游 = `github.com/zhu1090093659/dsh-web` 的 **`dev` 分支** `packages/skins/skin-center/skins/blue-fantasy/`；**市场文件与该分支逐字节相同**，等价可取 `https://dsh-market.com/assets/skins/blue-fantasy/patches.css`；清单 `https://dsh-market.com/manifest/skins.json`（作者 `powerdog996（DreamSkin 社区）· dsh-web 适配`）。
  - 2026-09-10 已合入一次上游更新：commit `66d83fa`「perf(skin-center): remove root backdrop-filter in blue-fantasy skin (#1358)」——`.aionui-root` 从 `backdrop-filter: blur(12px)` 那条规则里摘掉、并给 `[data-aionui-explorer-col], [data-aionui-preview-col]` 加 `contain: paint`（**就这 2 行**）。合并后实测：成品完整包含上游全部 207 条非空行（缺失 0），其余 ~485 行是本地追加。
  - 判定「本地文件是不是上游 + 本地追加」的快速方法（不改文件）：把上游文件逐行 trim 后与本地逐行比对，数「上游有、本地没有」的行——为 0 即完全包含。
  - ⚠️ **本机只有 Windows PowerShell 5.1（没有 `pwsh`）**：含中文的 `.ps1` 必须存成**带 UTF-8 BOM**，而 write/edit 工具写出的是**无 BOM** → 每次用工具改完 `scripts\skin-patches.ps1` 都要按字节补回 `EF BB BF`（`[System.IO.File]::WriteAllBytes`，**不要**用 PS 做文本往返）。脚本内部也刻意**不把 git 的 stdout 管道回 PowerShell**（`git merge-file` 就地合并），避开中文注释在 stdio 解码环节被搞坏。

- **⭐ 改动边界：只碰 `patches.css`，不要动 `skin.json` / `hooks.mjs`（2026-09-10 实测确认）**：
  - skin-center 的皮肤格式是分层的：**L1** token 重映射（`skin.css`）、**L2** 语义属性选择器（`skin.css`）、**L3** `patches.css` = **官方明说的「任意选择器、脆弱性作者自负」层** —— 我们那 485 行追加正是 L3 的正当用法，**不需要 fork 上游**。皮肤中心只提供"自定义主题"（仅审计过的 token，不接受任意 CSS），**没有**皮肤之外的 L3 叠加层，所以任意 CSS 只能写在某个皮肤的 `patches.css` 里。
  - **为什么不能碰另外两个文件**：官方市场安装靠 `dsh-market.provenance.json` 做**字节校验**，钉的是 `skin.json` + hooks 入口；README 原文「任何被修改、改名、手工投放或篡改的目录都会继续拒绝 hooks facet，声明式部分仍正常加载」。→ 改 `patches.css` **不影响**（实测 `GET /api/skin-center/v2/catalog`：blue-fantasy 只有一条 `shadows the built-in "blue-fantasy" skin` 提示，**零 provenance/hooks 拒绝告警**）；但一旦为了让版本号好看去改 `skin.json`，hooks 立刻被拒。
  - 顺带：**blue-fantasy 是 skin-center 包内内置皮肤**，用户目录同 id 会遮蔽内置（那条 warning 就是这意思）；其 `hooks.mjs` 只有 25 行、**只注入 favicon**（`assets/whale-icon.png`），背景壁纸是**声明式** `contributes.backgroundMedia`、由皮肤中心渲染 —— 所以即使自建皮肤 id 把 hooks 弄丢，代价也仅是一个 favicon。
  - 因此路线选择：**想就地维护** → 现状（市场皮肤 + L3 + 仓库存档 + `skin-patches.ps1`）；**想彻底解耦** → 复制成自己的皮肤 id（改 `skin.json` 的 id/name；代价：与原皮肤列表并存、丢 hooks=favicon、上游对 `skin.css`/资源的改进要手动搬）；**想分享/发布** → 一次性 PR 到 `github.com/zhu1090093659/dsh-web` 的 `skins/`（市场条目由该仓库生成），**不必长期 fork**。

- **⭐ 气泡基材已与皮肤中心设置联动（2026-09-10）**：
  - skin-center 的「背景」卡把值写在 `body` 的**行内 CSS 变量**上（`applyBubbleOpacity()` / `applyInputCardBlur()`，**与当前是哪张皮肤无关**，只要背景卡 `enabled` 就写；代码在 `@linxin666/dsh-client-ui-skin-center/lib/client.js` 的 `BUBBLE_ALPHA_VAR`）：
    - `--dsh-skin-bubble-alpha` = 气泡不透明度 / 100（默认 50 → `0.5`）
    - `--dsh-input-card-blur` = 输入卡片模糊（默认 10px）
    - 另有 `--dsw-skin-scrim` = backgroundOpacity/100（壁纸遮罩）
  - 补丁里**只在我们追加的区域**（「正文可读性」注释之后；上游那 262 行一字不动，否则 upstream diff 会变脏）把 **32 处**气泡底色改成
    `rgb(R G B / calc(var(--dsh-skin-bubble-alpha, .5) * K))`，K 取「**默认 50% 时正好复现原观感**」：
    `.5 → ×1`、`.55 → ×1.1`、`.75 → ×1.5`、`.4 → ×.8`、`.45 → ×.9`（深色同系数；超 1 由浏览器夹紧）。
    **37 处**模糊改成 `blur(var(--dsh-skin-bubble-blur, 10px|12px)) saturate(1.3)` —— 该变量皮肤中心**目前不存在**，先用兜底（= 现有观感），
    留给将来的「气泡模糊程度」滑块接管，届时**本文件无需再动**。
  - 实测（无头 Edge，手动改 `--dsh-skin-bubble-alpha`）：`0.8` → 胶囊 `.88`、导航栏 `1.2→1`、markdown 行 `.8`；`0.2` → `.22 / .3 / .2`；`0` → 全透明。
  - ⚠️ **坑：这个值可能是 0**。本次实测用户持久化配置 `~/.dsh/skin-center-active.json` 的 `background.bubbleOpacity` 就是 **0** → 联动一上线所有气泡立刻全透明，看起来像"皮肤坏了"。处置：把 `bubbleOpacity` 设回 50，或让用户在「设置 → 皮肤中心 → 背景」拖滑块。
  - ⚠️ 页面以**加载时**读到的值为准：改完 JSON 必须**重新加载**页面，否则页面后续任何写入会把内存里的旧值（0）写回去。
  - 官方推荐姿势可参考 whale-mom：`--dsw-specific-bubble: rgb(246 250 254 / var(--dsh-skin-bubble-alpha))` —— 我们只是把它用到 L3。

- **⭐「气泡模糊程度」滑块 = 本地补丁（2026-09-10 落地并实测通过）**：
  - 皮肤中心原本只有 5 个背景旋钮（背景遮挡 / 空对话背景模糊 / 有对话背景模糊 / **输入卡片模糊** / 气泡不透明度），**没有气泡模糊**。蓝幻想的 37 处 `blur(...)` 已改成读 `var(--dsh-skin-bubble-blur, 10px|12px)`，所以**只要有人写这个变量，滑块立刻生效**。
  - 补丁脚本：`scripts\skin-center-bubble-blur.mjs`（`node scripts\skin-center-bubble-blur.mjs` 应用；`--check` 看状态；`--revert` 从同目录 `.orig-bubble-blur` 备份还原）。**共改 3 个文件**：
    - 客户端 = **`@linxin666/dsh-web-all/lib/client.js`**（真正下发的）+ `@linxin666/dsh-client-ui-skin-center/lib/client.js`（独立安装时用）
    - 宿主 = `@linxin666/dsh-client-ui-skin-center/lib/index.js`（defaults / RANGES / zod schema → 持久化）
  - **⚠️ 怎么判定哪份代码在下发**（本次的关键前置）：拉首页，把里面 64 个 `/plugins/??…client.js` chunk 逐个抓下来找 `skin-center-bubble-opacity` —— 命中的是 **聚合包** `@linxin666/dsh-web-all/client.js`（2.5 MB，内联了 skin-center 的客户端代码）；宿主半边才是独立包（聚合包 `lib/shells/shell.js` 只有 `export { apply, inject }`，真插件由 `cordis.patch.yml` 的 `config.plugin: '@linxin666/dsh-client-ui-skin-center'` 指定）。**所以两个 client.js 都要打，只打独立包等于没打。**
  - **🔥 最大的坑：组件拿到的 `background` 是「门面对象」**，逐条列方法（`bubbleOpacity: () => background.bubbleOpacity(), setBubbleOpacity: (v) => …`），**不是控制器实例**。只给类加 getter/setter 而漏了门面 → `useSyncExternalStore(subscribe, undefined)` → **整个「设置 → 皮肤」面板崩掉**：控制台 `TypeError: n is not a function` + `slot entry crashed in 'settings.section'`。补门面两行后恢复。
    → 通用教训：给这类 controller 加字段，**类 + 门面必须同时改**；改完一定要看控制台有没有 `slot entry crashed`。
  - 实证（无头 Edge，F5 → 设置 → **皮肤**（这个分区在设置里叫「皮肤」，不是"皮肤中心"）→ 背景卡）：`#skin-center-bubble-blur` 存在（`input[type=range]`，0–20，默认 10，aria「10px」）；拖到 18 → body `--dsh-skin-bubble-blur: 18px` → 胶囊 `blur(18px) saturate(1.3)`；把不透明度拖到 70% → `--dsh-skin-bubble-alpha: 0.7`、胶囊底 `rgba(242,245,250,0.77)`（= 0.7 × 1.1，联动系数正确）。截图 `.shot/blur-knob.png`。
  - ⚠️ **持久化要重启后端**：宿主半边是进程内的模块，改完必须重启后端才会把 `bubbleBlur` 写进 `~/.dsh/skin-center-active.json`（实测未重启时该键被宿主丢弃 → 回落到默认 10）。客户端半边只需重新加载页面。
  - ⚠️ **插件升级会覆盖这 3 个文件**（`dsh-web-all` 或 `skin-center` 任一升级）→ 重跑脚本即可（幂等；备份 `.orig-bubble-blur` 在各自目录）。

- **输入栏下方三行（2026-09-10 定稿）**：`Go 5h`(cm-qstrip) / 统计行(`[data-composer-stats]`) / 本会话(`cm-root`) 的底**全部来自皮肤「外壳渲染层」的 accessory 规则**（`… [data-slot="conversation.input.dock"] > * { background: var(--dsh-composer-accessory-bg, var(--dsw-specific-tip,…)) !important; backdrop-filter: … !important }`），实测解析成 **不透明的 `rgb(243,245,251)` + `blur(10px)`**（补丁自己写的那份 `.5` 白底早被这条 `!important` 压掉了，改它是没用的）。
  - **「Go 5h」(cm-qstrip) —— 清掉外壳那层底**：查证 cost-meter 自己的 CSS 里 `.cm-qstrip` **只有布局、没有任何 background**（`display:flex;flex-wrap:wrap;justify-content:center;width:100%;max-width:var(--dsh-chat-content-width,720px);padding:0 calc(var(--dsh-composer-side-clearance,0px) + 16px)`），**里面每个 `cm-qchip` 小胶囊才各自有底**（`var(--dsw-alias-bg-layer-2)`）。用户说的「本来就有背景」指的就是这些小胶囊 → 用高特异性 `!important` 把外壳底清成 `transparent`（连 `backdrop-filter`/`box-shadow` 一起清），并**撤掉皮肤之前给的 fit-content/内边距**，交还插件原生布局（否则那一条会被压成 135px 的小条，与官方不一致）。
  - **统计行 + 本会话行 —— 毛玻璃**：用户先要求「优化成透明的」，做成**全透明后又反馈「需要背景模糊」**，故最终是**半透明底 + blur**：`background: rgba(242,245,250,.55) !important; backdrop-filter: blur(10px) saturate(1.3) !important; box-shadow: none !important`（底色调皮肤基色 `rgb(242,245,250)`，按「配色基准」规则取自左侧栏）。深色主题 `rgba(16,22,42,.55) !important`。
  - 覆盖外壳层必须用**同款选择器 + 皮肤前缀 + `!important`**：前缀加进去后特异性 (0,4,1) 压过外壳层的 (0,3,1)；这是 patches.css 里**唯一必须用 `!important`** 的地方。选择器要同时覆盖 `conversation.input.dock` 与 `conversation.composer.dock` 两种 dock 变体。
  - 实测（F5 后）：cm-qstrip 条 `bg=透明/bf=none`（737 宽，原生布局，只余小胶囊自身底色）；统计行 `bg=rgba(242,245,250,.55) bf=blur(10px) saturate(1.3)`；本会话行同。
  - **`cm-qchip`（「Go 5h」里的小胶囊）：插件自己的 hover 会让它"变透明"** ——
    `.cm-qchip{background:var(--dsw-alias-bg-layer-2)}`（常态 77.6% 浅灰实底）vs
    `.cm-qchip:hover{background:var(--dsw-alias-interactive-bg-hover)}`（hover 只有 8% 靛蓝）。
    实测 `0.776 → 0.08`，视觉上就是鼠标一移上去整颗胶囊淡掉。**用户 2026-09-10 反馈的就是这个**。
    中途两版（都被用户推翻，别再来）：① 只把 hover 换成 `rgba(74,95,168,.18)`（仍"变淡"）；
    ② 加 `blur(10px)`（**方向对了**——半透明 hover 不模糊就是"清晰透出壁纸"，像穿帮；
    见放大图 `z-chip-hover`：胶囊内部的动画人物被糊开、外面线条依旧锐利）。
    **✅ 定稿（用户 2026-09-10：「这玩意儿跟别的类似的按钮的颜色和 hover 都不一样，
    照着下面的那个轮数的样式改改」）—— 直接对齐同区域的「轮数」行，不发明新颜色。**
    把配件区挨个测一遍（CDP 逐个 hover）得到的基准：
    | 元素 | 常态 bg | hover bg | 备注 |
    |---|---|---|---|
    | 轮数行 `bOPqQW_root` = `[data-composer-stats]` | `rgba(242,245,250,.55)` + blur(10px) saturate(1.3) | **不变** | radius 12px |
    | 轮数药丸 `bOPqQW_pill`（行内两颗可点胶囊） | 透明 | `rgba(74,95,168,.08)` | 插件标准 hover |
    | 本会话行 `cm-root` | `rgba(242,245,250,.55)` + blur | 不变 | 皮肤补丁给的 |
    | 余额/今日 `cm-foot` | 透明 | `rgba(74,95,168,.08)` | 插件标准 hover |
    | ← 唯独 `cm-qchip` 用插件默认 `--dsw-alias-bg-layer-2`（**0.776 偏灰实底**） | | hover 跳到 8% 靛蓝**丢掉底色** | 所以是全场唯一的"灰盒子"且 hover 会变淡 |
    定稿写法：常态 = 轮数行同款 `.55` + blur；hover = **保留这层底**，再叠一层插件标准的 8% 靛蓝
    （用 `background-image: linear-gradient(...)` 叠加，而不是把 `background` 换掉）：
    ```css
    [class*="cm-qchip"] { background: rgba(242,245,250,.55); backdrop-filter: blur(10px) saturate(1.3) }
    [class*="cm-qchip"]:hover { background-color: rgba(242,245,250,.55);
      background-image: linear-gradient(rgba(74,95,168,.08), rgba(74,95,168,.08)) }
    ```
    深色主题同构：底色 `rgba(16,22,42,.55)`、叠加层 `rgba(160,185,235,.08)`（**必须补深色常态那条**，
    否则深色下会被浅色 `.55` 盖住）。
    实测（F5 后）：常态 `bg=rgba(242,245,250,.55) bf=blur(10px) saturate(1.3)` **与轮数行/本会话行逐字节相同**；
    hover = `.55` + `LG(74,95,168,.08)`，净效果 == 轮数药丸 hover（`.55` 底 + 8% 靛蓝）。
    放大对照图：`.shot/qchip-align-rest.png` / `qchip-align-hover.png` / `qchip-align-ref-stats.png`。
    ⚠️ 圆角仍是插件原来的 **6px**（用户只提颜色与 hover；轮数行是 12px 胶囊圆角）——要"更一致"可改 12px。
    ⚠️ 顺带发现：hover 这颗胶囊 500ms 后插件会弹 tooltip（`Go: 0% · 重置:… 点击立即刷新`），
    截图里出现的黄条是它，不是样式问题。
  - **`[class*="yAWgPa_chip"]` = `dsh-client-ui-conversation` 的 `ReferenceChip`（输入框里 @引用 的小胶囊）**：
    ① 补丁「补漏」批次曾把它误当成"会话 chip"改过（白底 `.5` + `blur` + `8px` 圆角）→ 已删；
    ② 2026-09-10 我又短暂加过一版常态/hover 底色 → **同样已撤回**（当时把"hover 变透明"误判成它，
    实际是上面的 `cm-qchip`）。
    它现在**完全沿用插件原样式**：`.yAWgPa_chip{background:var(--dsw-alias-interactive-bg-hover);
    border-radius:6px;color:var(--dsw-alias-state-business-primary);height:22px;padding:0 6px;display:inline-flex}`
    —— 常态就是 8% 靛蓝（`#4a5fa814`），**且插件没有任何 `:hover`**（实测 `:hover` 命中但底色不变）。
    ⚠️ 教训：**"某个元素 hover 变透明"这种反馈，一定要先在页面上把候选元素逐个 hover 量一遍**，
    别凭"最像"就动手——本次就是答错对象、白改一轮。若要让它更显眼，**别用左侧栏那个中性基色**
    （压在白色输入卡片上≈白、比 8% 靛蓝更看不见），要用它自己的蓝色系加强。

- **踩坑**：补丁里用 `[class*="<hash>_row"]` 这种 **CSS-module 哈希类名**匹配「正文行」时极易误伤同前缀的其它模块。`uV2eYG_` 是 **composer（输入框）模块**的哈希前缀，`uV2eYG_row` 就是「输入框下方那排按钮（+ / 附件 / 权限 / 模型 / 发送）」的容器行，**不是正文行**；给它加 `background`+`backdrop-filter`+`width:fit-content`+`margin:auto` 后，该行实测塌缩到 **16px 宽**、按钮全部溢出到白块之外 —— 用户看到的就是「按钮样式被搞坏」。
- **修法**：排除 composer 后代即可，正文行规则不变：
  `[class*="uV2eYG_row"]:not([data-composer-card] *):not([data-composer-seat] *)`（深色版同改）。排查同类问题的办法：先数一遍每个 `[class*="..."]` 选择器实际命中几个元素、分别在哪个区域（`[data-composer-card]` / `[data-slot="conversation.session"]` / `[data-pane="sidebar"]`）——本次那批「补漏」哈希里只有它命中，且 100% 在 composer 内。
- **第五类踩坑：哈希撞上「设置面板」的行（2026-09-10 用户截图反馈，已修）**。「补漏」块里裸写的 `[class*="lats3W_row"]`，在当前构建里是**设置面板**「对话显示 / 紧凑」那一行的 CSS-module 哈希 → 该行被套上白底 + 8px 圆角 + `width:fit-content` + `margin:auto`，行宽从 **564px 塌成 282px**、描述文字被白块盖住（同面板其它行都正常，一眼能看出坏的是那一行）。
  - **定位手法（可复用）**：无头 Edge 打开 设置 → 通用设置，找到目标行的叶子元素，向上 5 层 dump `tag/class/rect/computed(background/padding/border-radius/width/box-shadow)`，**与邻近的正常行逐项对比**（本次差异：282 vs 564、白底 vs 透明、radius 8 vs 0）；再对候选 `[class*="哈希"]` 做全文档命中统计并按区域归类（`会话正文 / 输入区 / 设置面板 / 左侧栏 / 右侧栏`）——`lats3W_row` 命中 2 个，100% 在设置面板，铁证。
  - **两条修法（都已落地）**：① **删掉该哈希条目**；② **整块加作用域 `[data-slot="main.conversation"]`** —— 实测**设置面板在它之外、会话正文/输入区（含 dock）都在它之内**，所以今后任何哈希再撞到设置面板/侧栏/市场等 UI 都不会受伤。
  - 回归验证：设置行恢复 **564px / 透明 / radius 0**（与同面板各行的宽度数组 `[564×6]` 一致）；会话侧 `callRow` 仍 `rgba(255,255,255,.5)+blur(10px) saturate(1.3)`、`markdown>p`、`userStack bubble`、`[data-turn-tail] actions`、配件胶囊/统计行/`cm-root` 全部照旧。
  - 教训：**跨构建的 CSS-module 哈希选择器一律不要裸写** —— 要么加 `[data-slot="main.conversation"]` 作用域，要么优先用语义属性（`data-slot` / `data-dsh-*` / `data-composer-*`）。
- **无浏览器也能精确验证**（比肉眼截图快）：起一个无头 Edge 指向同一个 dsh 端口
  `msedge --headless=new --remote-debugging-port=9222 --user-data-dir=%TEMP%\edgecdp "http://127.0.0.1:3080/?token=<token>"`，
  再用 Node 的**全局 `WebSocket`** 走 CDP：`Runtime.evaluate` 遍历 `document.styleSheets` + `el.matches(rule.selectorText)` 找出真正命中的规则，`Page.captureScreenshot` 出图对照；改完 `Page.reload {ignoreCache:true}` 再验一遍。用完只杀命令行含该 `--user-data-dir` 的进程，别误杀用户自己的 Edge。
- 皮肤 CSS 响应头是 `cache-control: no-store`，但页面**只在加载时拉一次** → 改完必须让内容页重载（标题栏聚焦后按 F5，或重启后端）。控制服务 `/action?name=reload` 需要**进程内令牌 `k`**（只以 `window.__DSHD_K` 注入页面），外部脚本拿不到，别指望用 curl reload。
- **第二类白条：皮肤「外壳渲染层」的 accessory 规则**（2026-09-10 续）。skin-center 另有一张 `<style data-dsh-shell-rendering>`（**无 href，样式表索引拿不到名字，只能靠 `ownerNode.attributes` 识别**），其中一条把 composer 停靠区的**每一个直接子元素**都当 accessory 上背景：
  `html[data-dsh-skin] [data-phase="active"] [data-slot="conversation.input.dock"] > *`（以及 `…composer.dock > *`）→ `background/border-radius/backdrop-filter` **全带 `!important`**。
  子元素自身 CSS 若写 `width:100%`（如 `dsh-client-ui-chat/StatsPills.module.css` 的统计行 `.xxx_root[data-composer-stats]`），就会被拉成**通栏白条**（实测 737px），而 `cm-qstrip`/`cm-root` 因为自带 `width:fit-content` 才是贴合的小条。
  **修法**（写在 `patches.css`，只动宽度、不碰 `!important` 的背景）：给该子元素加
  `width: fit-content; max-width: 100%; margin-left/right: auto; padding-left/right: 10px;`，实测 737px → 390px 且保留原有背景/模糊/圆角，深浅色主题自动适配。
  ⚠️ 内边距要跟同区域的兄弟配件对齐：`cm-qstrip`（Go 5h）/`cm-root`（本会话 ¥…）在 cm-* 补丁里都是 **10px**，写 16px 会让这条比下面那行每边多凸出 6px（用户一眼就看出来了）。量法：对 `[data-slot="conversation.input.dock"|"conversation.composer.dock"] > *` 逐个取 `getBoundingClientRect().width` 与 `paddingLeft/Right` 横向对比。
  排查套路：列出 dock 的所有直接子元素（class / `data-*` / 宽度 / 是否 `fullWidth`）逐个对比，谁通栏就收谁；`[data-dsh-composer-*]` 这类语义属性比哈希类名稳定，优先用它。

- **第三类：同一行被自己叠了多层背景（2026-09-10 续）。** `Pwsh · …` / `工具调用 · …` 这类行是**三层嵌套、尺寸几乎重合**，而「补漏」批次给三层都加了 `background + backdrop-filter + border-radius`：
  `[class*="callRow"]`（最外层 wrapper，含 1px 8px 内边距）→ `[class*="o3BgMG_root"]`（工具行本体）→ `[data-disclosure-row]`（可展开标题行 `_row_xxx o3BgMG_row`）。半透明底叠 3 次 → 越叠越白/越糊，用户看到就是「两层背景」。
  **修法**：保留最外层 `callRow`，重置其内层——
  `[data-slot="conversation.session"] [class*="callRow"] [class*="o3BgMG_root"], … [class*="o3BgMG_summary"], … [data-disclosure-row] { background:none; backdrop-filter:none; border-radius:0; box-shadow:none; padding:0; margin:0; width:auto; max-width:none }`，
  **必须再写一遍 `body[data-ds-dark-theme]` 前缀的同款选择器**，否则深色那条 `body[data-ds-dark-theme] … [data-disclosure-row]` 特异性更高、重置不掉。验证：沿祖先链数 `backgroundColor !== 'rgba(0,0,0,0)'` 的元素个数，修复前 3 → 修复后 1。

- **第四类：子串误伤（`[class*="X"]` 命中 `XSomething`）**。「深度求索中… 4分12秒」的计时 chip 类名是 `EvIC1a_turnStatusClock`，**包含** `turnStatus` 子串 → 连它一起吃到了 `[class*="turnStatus"]::before` 的白底，计时那段变成两层。修法：`[class*="turnStatus"]:not([class*="turnStatusClock"])`（浅色 + 深色两条都要改）。**凡是用 `[class*="…"]` 的补丁都要问一句：这个子串还会命中谁？**
- **补漏：「本次产出」行**（deliverables，`nArs4W_producedRow`）之前没有任何补丁覆盖 → 补玻璃底 + **自适应宽度**。
  **⚠️ 宽度不能用裸 `width: fit-content` 单干**：它的父级是 `display: contents` 的插槽宿主（宽度算出来是 0），fit-content + `max-width:100%` 会把里面的文件 chip 挤成 3 行（实测 434px/3 行）。正解是让自身变成**行内级 shrink-to-fit 盒**：
  `display: inline-flex; align-self: flex-start; width: fit-content; max-width: 100%` —— 不依赖父宽、也不怕父级是 flex（`align-self` 防 stretch）。用**同构合成 DOM**（block 737px 父 > display:contents 宿主 > 行）在真实样式表下验证：背景宽 281px / 内容 243px / **1 行不换行**。
- **顶部导航栏与右侧侧边栏的底色 + 导航栏内部各项的自适应底（2026-09-10 定稿）**。
  - **颜色取自「左侧导航栏」**：`[data-pane="sidebar"]` 的 computed `backgroundColor` = **`rgba(242,245,250,.75)`**（实测值）。顶部导航栏与右侧栏面板统一用这个色 + `.75`：
    `[data-slot="main.conversation"] header, [data-slot="rightbar.session"] > *, [class*="rightbarCol"] [class*="panel"] { background: rgba(242,245,250,.75) }`（顶部另有 1px 分隔线），深色主题 `rgba(16,22,42,.75)`。
  - **导航栏内部各项（会话名 / 模式选择 / 对话 / 轨迹 / 上下文）：最终决定「不加任何背景」**。历程（三轮反转，记下来免得再绕）：① 按要求加贴合内容宽度的模糊底 → ② 用户反馈「文字底下的模糊很怪」→ 去掉 `backdrop-filter` 只留底色 → ③ 用户明确「那几个字不要背景了」→ **整条规则删除**（背景/圆角/内边距/宽度都不加），恢复官方原始间距与下划线指示器。实测：会话名 102x28、模式 71x22、tab 26/26/39，全部 `bg=rgba(0,0,0,0)`、`bf=none`、`padding 0`。
    若以后又要小底，选择器用语义属性：会话名 = `header nav[aria-label="会话层级"]`、tab = `header [role="tablist"] button[role="tab"]`，模式选择没有稳定钩子只能用 `[class*="headerActions"]`。
  - **右侧栏是官方的 `@deepseek-ai/dsh-client-ui-sidebar-right`**（`rightbarCol` / `rightbar.session` / `P3OORG_panel` 都在它里面；`dsh-better-sidebar` 只是通过 `dsh.client.inject` 往官方右侧栏里塞「文件/终端/浏览器」内容，**右侧栏本身不是它的**）。渲染层级是
    `[class*="rightbarCol"]`（列）→ `[data-slot="rightbar"]`（**display:contents，0×0**）→ `[data-slot="rightbar.session"]`（**同样 0×0**）→ `[class*="panel"]`（真正画出来的面板，`position:absolute`）。
    **给前两个宿主上底等于没上**（第一版就是踩在这里，用户反馈「侧边栏背景没生效」），必须写到 `.panel` 那一层。
  - 判断某个面板属于谁：直接在 `node_modules` 里 `Select-String -Pattern "<哈希前缀>|<slot名>"`——本次 `P3OORG_panel`/`rightbar.session` 只命中官方包，better-sidebar 里一次都没有。

- **子 agent 自动打开位置：dsh-better-sidebar 0.19 起被写死成底部面板（2026-09-10 已本地回退）**。
  - 现象：调用子 agent 时弹**底部面板**而不是右侧栏。
  - 根因：`~/.dsh/profiles/web/node_modules/dsh-better-sidebar/lib/client.js` 里自动打开逻辑改成 `openTab({ type: "subagent", title: t("subagent"), target: "bottom" })`（3 处：两个 effect + subagentJump）。**0.18.0 时是无 target**（`openTab` 内部 `if (surface !== void 0 && seed.target !== "bottom")` 才走右侧栏 surface 分支，并带 `revealIfOpened` 自动展开），并且旧版还会先 `store.reduce(... togglePanel ...)` 打开面板。今天随 `@linxin666/dsh-web-all@0.3.20` 把 better-sidebar 从 0.18.0 升到 **0.19.0-alpha.1** 才变了行为。
  - 回退：把那 3 处的 `, target: "bottom"` 删掉（**保留** terminal / jobs 的 `target: "bottom"` 不动），`node --check` 验证语法；备份留在 `lib/client.js.orig-subagent-bottom`。**插件升级会被覆盖，需重打**。
  - 不想打补丁的替代方案：better-sidebar 设置里关掉「自动打开子 agent」（`autoOpenSubagent`，默认 true），改成手动点右侧栏的 subagent 标签页。
- **流程约定（用户 2026-09-10 明确）**：皮肤改动只在页面加载时生效，**每次改完 patches.css，验证截图前必须先 F5 刷新内容页**（无头验证就是脚本里的 `Page.reload {ignoreCache:true}`），否则截出来的还是旧样式。
  通用教训：给 `[class*="…"]` 加视觉属性前，先看这条选择器会不会同时命中**同一子树里的多层**；要「只留一层」就得连内层一起重置。

## 「读取不了之前的对话」＝ dsh 0.1.5-rc.1 会话迁移闸门 bug（2026-09-10，已本地补丁）

- **现象**：点侧边栏任意旧会话 → 右侧报 `历史加载失败：failed to observe session "…": cannot safely transform unclassified message source; source v0 artifact remains unchanged (raw log: …\session.jsonl.zstd)`。
- **根因**：`@deepseek-ai/dsh-session-format-v2-to-v3` 的 `SOURCE_KINDS` 白名单（15 项）不含**核心自己写出的** kind：`provider` / `fallback`（`@deepseek-ai/dsh-session-title`，会话标题 LLM 请求/兜底）、`instruction-hint`（agent preset 的 tool-bootstrap 注入，如 liangshen 预设的 anchored-tool-bootstrap）。v0 会话一旦含这些来源，整段迁移就被拒绝（是 upstream 的 kind 分类漂移，不是会话损坏）。
- **本地补丁**（会被 dsh 升级覆盖，升级后需重打）：`%APPDATA%\DSH Desktop\versions\<id>\node_modules\@deepseek-ai\dsh-session-format-v2-to-v3\lib\index.js` 的 `SOURCE_KINDS` 末尾补 `"provider", "fallback", "instruction-hint"`（已带注释块）。**改完必须重启后端**才生效（迁移代码在宿主进程内存里）。
- **验证办法**：另起一个实例（`node <bin.js> --profile minimal --no-open --port 3099`，别动线上那个）→ 无头 Edge 连上去点旧会话，看会话正文是否出现（出现即修复）。
- **读会话日志**：`~/.dsh/sessions/<workspace>/<session-id>/session.jsonl.zstd` 是**多帧 zstd 追加**，Node 的 `zstdDecompressSync` 与流式解压都只吃第一帧；要按 `28 B5 2F FD` magic 切帧后逐帧解压才能拿到全部记录。

## 「梁神模式」预设的移除（2026-09-10）

- 它来自 agent 预设目录 `~/.dsh/.agent-presets/liangshen/`（`preset.yml` 的 `name: 梁神模式`，由 `@linxin666/dsh-liangshen` 的宿主半边在启动时同步）。**该插件行在 `@linxin666/dsh-web-all` 聚合层里默认就是 `disabled: true`（id `web-ui-liangshen`）**，所以移走预设目录后不会被重建。
- 移除方式：把 `~/.dsh/.agent-presets/liangshen` 移到扫描目录之外（本次移到 `~/.dsh/_disabled-agent-presets/liangshen` 留底），刷新页面即从预设选择器消失。**已被该预设创建的会话仍可正常打开**（会话头里的 `agentPreset: liangshen` 失去对应预设只会掉徽章，不报错）。

## 侧边栏「展开其余 N 个会话」点不动？先别急着当 bug（2026-09-10 误判记录）

- 会话列表容器 `[class*="listArea"]`/`[class*="list"]` 是**可滚动**的（`overflow-y: auto`，实测 client 340 / scroll 940）。列表最后那项「展开其余 N 个会话」经常在可视区之外，`getBoundingClientRect()` 仍会给出**可视区外**的坐标，往那个坐标派发鼠标事件自然打到底部用量面板（`cm-foot`）上——看起来像「被盖住 / 点不动」，其实只是**没滚动到位**。
- 复核手段：`document.elementFromPoint(btn 中心)` 若返回别的元素，先比较 `btn.getBoundingClientRect().top` 与滚动容器 `getBoundingClientRect().bottom`，确认是不是「屏幕外」；对照实验：用 `--profile minimal` 起同版本实例（没有 cost-meter 底栏、列表更短），同一个按钮若可点，即证明与插件无关。

## 会话协作约定

- 记忆召回与写回由 MemOS Cloud 插件自动完成，**不要**手动调用 `mcp__memos-mcp__*`（仅主动管理记忆时按需使用）。
- 用用户的语言（中文）回复正文；工具日志/命令输出不受此约束。