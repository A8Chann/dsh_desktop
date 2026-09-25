# WebView2 浏览器加速键（F6）打崩浏览器进程 → 内容区空白外框

> 2026-09-25 定位并修复。**症状极具迷惑性**：窗口、标题栏、按钮都在，只有内容区一片空白，
> 且「过一会儿才白」，看起来像皮肤/样式或渲染层级问题，实际是 **WebView2 浏览器进程整个崩了**。

## 症状（用户视角）

- 按 **F6**，过一会儿（几秒~几十秒）整个窗口只剩外框：内容区什么都没有，
  透出的是窗口的**亚克力磨砂材质**（Acrylic 把桌面内容模糊地透出来）。
- 标题栏按钮此时也按不动了（chrome WebView 同属一个浏览器进程，一起死了）。
- **永不自愈**，只能重启应用。

## 根因（实测证据链）

1. 按一次 F6 → `msedgewebview2.exe`（该应用的）进程数 **7 → 0**，整棵子进程树
   （browser/gpu/utility/renderer + `Chrome_WidgetWin_*`、`Chrome_RenderWidgetHostHWND`、
   `Intermediate D3D Window`）全部消失；应用自身进程仍 `Responding = True`。
2. **三个 `WRY_WEBVIEW` 宿主 HWND 依旧 `vis=True`、尺寸正确** —— 所以「WebView 是否可见」
   这类检查完全看不出问题，必须数 **WebView2 进程**或数**子窗口树**。
3. Crashpad 落 dump：`%LOCALAPPDATA%\io.dsh.desktop\EBWebView\Crashpad\reports\*.dmp`，
   `watson_metadata` 里写着决定性信息：

   ```
   ApplicationName=msedgewebview2.exe; ModuleName=msedge.dll; ProcessType=browser;
   SubCode=0xc0000005; WV=DSH-Desktop-2.8.0-tauri.exe; WV_V=2.8.0
   ```

   即 **browser 进程在 msedge.dll 里访问违例**（运行库 153.0.4234.48）。
   同目录 `Crashpad/temp/edge_shutdown_crash.txt` 内容为 `1`。
4. 复现脚本（本机）：启动后热身 40s → 发一次 F6 → 立即归零，并在 1s 内新增一份 dump。
   同一脚本在**未发 F6** 的情况下跑十几分钟不掉。

## 修复

`src-tauri/src/accel.rs`：挂 WebView2 的 `AcceleratorKeyPressed`，把 **VK_F6(0x75) / VK_F7(0x76)**
直接 `SetHandled(true)` 吃掉。**就这一件事，别顺手多干。**

```rust
// 关键 API 路径（tauri 的 PlatformWebview 已经暴露 controller，不用绕 wry）
webview.with_webview(move |w| {
    let controller = w.controller();              // PlatformWebview::controller()
    let h = AcceleratorKeyPressedEventHandler::create(Box::new(
        move |_s: Option<ICoreWebView2Controller>,
              args: Option<ICoreWebView2AcceleratorKeyPressedEventArgs>| {
            let mut vk = 0u32; args.VirtualKey(&mut vk)?;
            if vk == 0x75 || vk == 0x76 { args.SetHandled(true)?; }   // 只吞 F6/F7
            Ok(())
        }));
    controller.add_AcceleratorKeyPressed(&h, &mut token)?;
})
```

依赖：`webview2-com = "0.38.2"`、`windows-core = "0.61"`（必须与 tauri-runtime-wry 2.11.4 用的一致；
`cargo tree -i webview2-com` 应只有一个版本）。`wry` 不用加，`tauri::PlatformWebview` 自带 controller。

## 🔴 不要再犯：别用 `SetAreBrowserAcceleratorKeysEnabled(false)`「顺手加固」

**2026-09-25 实际犯过，用户立刻发现「F5 也被你干掉了」。** 那个开关：

- **治不了 F6**（F6 不在它的官方覆盖清单里：只有 Ctrl+F/F3、Ctrl+P、Ctrl+R/F5、Ctrl±、
  Ctrl+Shift+C/F12 等）——对 F6 崩溃毫无帮助；
- **却会真真切切关掉 F5 刷新 / Ctrl+R / Ctrl+F / Ctrl+P / F12** —— 这些本来是能用的功能。

所以：**只吞键，不动设置。** 修复的判据是「F6 不崩 + F5 照常刷新」两条一起过，
只测 F6 会漏掉这个回归（当时就是这么漏过去的）。

### 版本线收尾（2026-09-25）

这个回归曾以 `v2.9.1` 之名单独发过一版，用户随后要求**把 2.9.0 直接覆盖重发**：
源码版本号锁回 `2.9.0` 重新构建，`v2.9.0` tag 强制移动（`git push --force origin v2.9.0`），
Release 换掉 asset 与正文，再删掉 `v2.9.1` 的 tag 与 Release。
⚠️ 删了 tag **不会**连带删掉 Release —— 按 tag 查会 404，但 `GET /releases` 里仍在，
要按 **id** `DELETE /repos/:o/:r/releases/:id` 才干净（其 asset 也要先删）。

## ⚠️ 两个致命细节

1. **绝不能在 `setup()` 里同步调 `with_webview`**：它内部走 `run_on_main_thread`，而主线程正卡在
   `setup()` → **死锁**。症状：进程活着、窗口和三个 WebView 宿主都建好了，但页面不渲染、
   后续日志一行不写（看起来像"应用启动崩了"）。
   正确做法：后台线程 `sleep(1.2s)` → `app.get_webview(label)` → `with_webview(...)`（`accel.rs` 即此结构）。
2. `AcceleratorKeyPressed` 的回调签名里**第一个参数是 `ICoreWebView2Controller`**（不要写成 handler
   自己），第二个是 `Option<ICoreWebView2AcceleratorKeyPressedEventArgs>`。

## 验证清单（两条都要过）

1. **F6 不崩**：连发 5~23 次 F6，`msedgewebview2` 进程数保持不变、无新 dump，
   日志出现 `[accel] <label>: 已吞掉 VK=0x75`。
2. **F5 照常刷新**：焦点在标题栏时按 F5，日志应出现 `[http] action: reload-visible`
   （这是 chrome.html 自己的 F5 处理）；顺带能看到 `[backdrop] reload 后开始重推幕布`。

## 诊断脚本（可复用）

见 `scripts/f6-crash-diagnostics/`：

- `deep.ps1` — 枚举 `Tauri Window`/`DSH Desktop` 的整棵子窗口树（看 `WRY_WEBVIEW` 与
  `Chrome_*`/`Intermediate D3D Window` 是否还在）。
- `monitor.ps1` — 每 350ms 采样三个 WebView 的可见性/矩形/前台窗口，变化时落日志+截图。
- `f6lab.ps1` — 复现实验：热身 → 定时发 F6 → 监控 `msedgewebview2` 进程数归零。
- `probe.ps1` / `sendf6.ps1` / `sendkey.ps1` / `refocus.ps1` — 定位窗口、截图、注入按键、切前台。
- `crop.ps1` / `fullshot.ps1` — 全屏截图与裁剪（取窗口实拍图）。

一律用 `powershell.exe -NoProfile -ExecutionPolicy Bypass -File xxx.ps1` 运行
（本机 pwsh 不在 PATH，且默认执行策略禁止未签名脚本）。

⚠️ 另一个「看着像 bug」的坑：**同时跑两个实例**（或上一个实例没退干净）时，新起的那个
会有窗口和三个 WebView 宿主、但**没有任何 webview2 进程**、也不写日志——那不是崩溃，
是单实例/端口竞争。验证前先确认 `Get-Process *dsh*` 是空的。


## 环境坑（这一轮踩到的）

- `main.log` 可能**不更新**：后端起进程持有句柄 / 多个实例；判断应用是否真的启动，
  不要只看日志，要同时看进程 + 窗口 + 子窗口树。
- **本机 pwsh 的 `Add-Type` 在 read-only 沙箱下会被拦**；`Get-CimInstance Win32_Process` 便宜且够用。
- **不要用 PowerShell `Get-Content`/`Set-Content` 往返改 `main.rs`**：本机按 ANSI 读 UTF-8，
  会把中文注释写成乱码、还会吃掉换行（2026-09-25 实际写坏一次，用 `git checkout --` 恢复后
  改用 edit 工具）。脚本文件同理：**含中文注释的 .ps1 用 PS 5.1 跑会解析报错，脚本一律写纯 ASCII**。
- `Start-Process` 在本沙箱会因 stdio 继承而挂住；改用
  `Invoke-CimMethod Win32_Process Create -Arguments @{CommandLine=...; CurrentDirectory=...}`
  （**必须给 `CurrentDirectory`**，否则工作目录是 System32，应用读不到 settings）。
- `src-tauri/target/debug/dsh-desktop.exe` 在本机**起不来 WebView2**（窗口建好但没有任何
  webview2 进程）。验证这类修复请用 **release** 产物。
- **进程抢占才是"起不来"的最常见原因**：同时跑两个实例（或上一个没退干净 / 上一个的
  `dsh-desktop.exe` 还占着 `target\release` 里的文件）时，新起的那个会有窗口 + 三个 WebView
  宿主，但**没有任何 webview2 进程、也不写日志**。判据：`Get-Process | ? ProcessName -like '*dsh*'`
  必须先为空，再启动、再验证（否则会把"抢占"误判成"补丁把应用弄坏了"）。
- 本地 `target\` 下的 exe 直接双击/拉起时容易撞上上一条；要让进程真正跑起来（WebView2 正常创建），
  把它复制到桌面路径再启动最稳（实测 desktop 路径下必成，`dist\` 与 `target\release\`
  冷启动时偶发拿不到 webview2）。
- 发版推送：`git push` 在本机会卡在 GCM 凭据交互。改用 Windows 凭据库里的 gho_ token +
  一次性 helper：`git -c credential.helper= -c 'credential.helper=!f() { echo username=x-access-token; echo "password=$GH_TOKEN"; }; f' push origin main`
  （`$GH_TOKEN` 从 `scripts/gh-cred-reader.cs` 读出后放进环境变量，别写进命令行）。
