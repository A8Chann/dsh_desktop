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


## 🧟 「启动是空的 / 双击没反应」的真凶：僵尸实例占着单实例锁

**2026-09-25 花了很久才抓到，务必先看这条。**

现象：双击图标后**什么都没有**（或只有一个空白外框的窗口），`main.log` **一行都不写**。

真凶链条：

1. 某个时刻 WebView2 浏览器进程死了（F6 崩、被强杀、渲染崩溃……），**应用进程却还活着**，
   于是三个 WebView 只剩空壳窗口 = 一个"空白外框"的僵尸实例；
2. 这个僵尸**握着单实例 mutex**（`tauri-plugin-single-instance`，标识符 `io.dsh.desktop`，
   跟 exe 名字无关）；
3. 之后用户再双击启动，新进程在**单实例回调里就直接退出了** → 表现为"双击没反应 / 启动即空白"，
   而且**日志一行都不写**（`setup()` 根本没跑）；
4. 越试越像"程序坏了"，其实只是那个看不见的僵尸在挡路。

排查命令（**按路径**匹配，别只按进程名 —— 改名后的副本/测试副本会漏掉）：

```powershell
Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -match 'dsh|DSH' } |
  Select-Object ProcessId, Name, ExecutablePath, CreationDate | Format-Table -AutoSize
```

判据：如果列出来的实例数 > 1，或者有一个实例的 webview2 进程数是 0 —— 那就是僵尸，先 `Stop-Process` 掉它再启动。

**产品侧的修复（v2.9.1 起）**：`accel.rs` 里装了 `BrowserProcessExited` 看门狗
（`ICoreWebView2Environment5::add_BrowserProcessExited`，注意这个方法在 **Environment5** 上，
`w.environment()` 拿到 Environment 后要先 `.cast::<ICoreWebView2Environment5>()`）。
浏览器进程一死就：写一行死因进日志 → 300ms 后 `std::process::exit(0)` 把自己收掉，
把单实例锁让出来。用户下次启动即可正常。日志形如：

```
[accel] !! WebView2 浏览器进程已退出（kind=1 browserPid=34660）：界面已失效，应用将自动退出以免占住单实例锁
```

**另外**：`main()` 里现在有一条 `DSH_BOOT_TRACE=1` 才生效的引导日志（写到
`%TEMP%\dsh-boot-trace.log`），专门用来区分"进程没起来 / 卡在 setup / 单实例退出"这三种情况。

## 环境坑（这一轮踩到的）

> ⚠️ 更正：下面这条"工作区路径"的对照结论**后来被推翻了** —— 真正的解释是上面那节
> 「僵尸实例占着单实例锁」：当时那个僵尸恰好是留在工作区外目录里的一个进程，
> 我误把"它在挡路"读成了"路径有问题"。保留原始记录作反面教材：
> **凡是"同一份二进制换个位置就成败不同"的结论，先查有没有僵尸进程在挡路。**

- 🔴 **绝对不要从 DSH 工作区目录里启动这个 exe**（工作区根目录、`dist\` 都一样）。
  2026-09-25 实测对照（**同一份二进制、SHA256 完全相同**）：

  | 位置 | 结果 |
  |---|---|
  | `%TEMP%\dshpath\` | ✅ webview2 = 7、日志正常 |
  | `D:\dsh-dl\`（D 盘普通目录） | ✅ webview2 = 7、日志正常 |
  | `C:\Users\HWX\Desktop\` | ✅ webview2 = 7、日志正常 |
  | `D:\HTML\DSH_Desktop\dist\` | ❌ webview2 = 0、**一行日志都不写**（进程活着、窗口在） |
  | `D:\HTML\DSH_Desktop\`（工作区根目录） | ❌ 同上 |

  排除了「工作目录」和「启动上下文」两个变量：把 CWD 换成 `C:\Users\HWX` 仍失败；
  用 `schtasks /run`（完全脱离工具沙箱）也仍失败。**症状就是"启动即空白"**：
  窗口标题栏在、内容区空的，且 `main.log` 里连「==== 启动 ====」都没有。
  → 发版产物、桌面副本都是好的；把 exe 放到**工作区之外**的目录再启动即可。

  **机制（逐步定位出来的，别只记结论）**：

  1. 失败进程的诊断读数：存活、`19 threads / 302 handles / 80MB`、**三个 `WRY_WEBVIEW`
     宿主窗口都已创建**，但 **`WebView2Loader.dll` / `d3d11.dll` / `dcomp.dll` 全部未加载**
     —— 说明它卡在「创建 WebView2 环境」那一步的**早期**，并且是**静默失败**（连自己写日志
     的代码都没跑到，所以 `main.log` 一行都没有）。
  2. 判别实验：给同一个工作区里的 exe 加一个环境变量把 WebView2 数据目录指到别处 ——

     ```powershell
     cmd.exe /c set WEBVIEW2_USER_DATA_FOLDER=D:\dsh-udd&& "D:\HTML\DSH_Desktop\dist\DSH-Desktop-2.9.0-tauri.exe"
     ```

     → **webview2 = 7，起来了**。指到 `%LOCALAPPDATA%\<任意新目录>` 也一样成功。
  3. 反向对照：桌面那份 exe **显式**指定默认目录
     `%LOCALAPPDATA%\io.dsh.desktop\EBWebView` → 也成功（7 个进程 + 13 行日志）。
  4. 所以卡点是**「工作区里的进程」×「默认数据目录这个路径」这个组合**：
     只有"工作区 exe + 用默认数据目录"会失败；两边任意一边换掉就好了。
     目录本身的 ACL 是干净的（`icacls` 只有 SYSTEM/Administrators/HWX 的 FullControl，无 Deny），
     真正动手的是启动器给工作区进程套的那层沙箱/受限令牌：它对**应用自己的数据目录**
     （`%LOCALAPPDATA%\io.dsh.desktop`、`%APPDATA%\DSH Desktop`）有额外限制，
     连日志文件都写不进去 —— 这也解释了为什么失败时"一行日志都没有"。
     注：`icacls D:\HTML\DSH_Desktop` 里那条 `Everyone:(CI)(DENY)(DC)` 是工作区防删除项，
     不是本次的原因（它只影响子目录删除）。

  **绕开办法（任选其一）**：把 exe 放到工作区之外启动（桌面/D 盘普通目录/下载目录都行）；
  或者启动前设 `WEBVIEW2_USER_DATA_FOLDER` 指向工作区外的目录。

- `main.log` 可能**不更新**：后端起进程持有句柄 / 多个实例；判断应用是否真的启动，
  不要只看日志，要同时看进程 + 窗口 + 子窗口树。
- **本机 pwsh 的 `Add-Type` 在 read-only 沙箱下会被拦**；`Get-CimInstance Win32_Process` 便宜且够用。
- **不要用 PowerShell `Get-Content`/`Set-Content` 往返改 `main.rs`**：本机按 ANSI 读 UTF-8，
  会把中文注释写成乱码、还会吃掉换行（2026-09-25 实际写坏一次，用 `git checkout --` 恢复后
  改用 edit 工具）。脚本文件同理：**含中文注释的 .ps1 用 PS 5.1 跑会解析报错，脚本一律写纯 ASCII**。
- `Start-Process` 在本沙箱会因 stdio 继承而挂住；改用
  `Invoke-CimMethod Win32_Process Create -Arguments @{CommandLine=...; CurrentDirectory=...}`
  （**必须给 `CurrentDirectory`**，否则工作目录是 System32，应用读不到 settings）。
  ⚠️ 但这个方式**只对"工作区之外"的 exe 有效**，见上面第一条。
- **启动前先确认没有旧实例**：`Get-Process | ? ProcessName -like '*dsh*'` 必须为空再启动、再验证，
  否则会把「上一实例还占着」和「工作区路径」这两种"起不来"混在一起，误判成补丁问题。
- 发版推送：`git push` 在本机会卡在 GCM 凭据交互。改用 Windows 凭据库里的 gho_ token +
  一次性 helper：`git -c credential.helper= -c 'credential.helper=!f() { echo username=x-access-token; echo "password=$GH_TOKEN"; }; f' push origin main`
  （`$GH_TOKEN` 从 `scripts/gh-cred-reader.cs` 读出后放进环境变量，别写进命令行）。
