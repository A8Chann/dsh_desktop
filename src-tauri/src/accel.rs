//! F6 的「吞键」补丁（Windows 专属）。
//!
//! 背景（2026-09-25 实测定位）：本机 WebView2 运行库 153.0.4234.48 上，**按 F6 会让
//! WebView2 的浏览器进程崩溃**——Crashpad 里落的是
//! `ApplicationName=msedgewebview2.exe; ModuleName=msedge.dll; ProcessType=browser;
//! SubCode=0xc0000005`。浏览器进程一死，三个子 WebView 只剩空壳 HWND
//! （WRY_WEBVIEW 依然 visible、尺寸正确），内容区就变成「只剩外框、里面是亚克力磨砂」的空白，
//! 且永不恢复——只能重启应用。
//!
//! 复现记录（本机实测）：16:54:53 启动 → 16:56:25 发一次 F6 → `msedgewebview2` 进程数
//! 7 → 0，同时新增一份 16:56:26 的 browser 崩溃 dump。
//!
//! 做法：挂 `AcceleratorKeyPressed`，把 **F6 / F7** 直接 `SetHandled(true)` 吃掉，
//! 不让它们进 Chromium 的加速键处理路径。该事件对所有加速键都会触发，所以能精确兜住这两个键。
//!
//! ⚠️ **不要**用 `SetAreBrowserAcceleratorKeysEnabled(false)` 图省事：F6 不在它的官方覆盖
//! 清单里（治不了 F6），却会顺手关掉 F5 刷新 / Ctrl+R / Ctrl+F / Ctrl+P / F12 等**正常功能**
//! ——2026-09-25 已踩过并回退。浏览器加速键保持默认开启。
//!
//! ⚠️ 调用时机：**必须**在事件循环跑起来之后、且不要在 setup() 里同步调用。
//! `with_webview` 走的是 `run_on_main_thread`；在 setup()（主线程）里同步调用会死锁，
//! 实测表现为「窗口和三个子 WebView 宿主都建好了，但页面不渲染、日志也不再写」。
//! 所以这里统一从后台线程延迟 1.2s 后直接调用（不绕主线程）。

#[cfg(windows)]
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2AcceleratorKeyPressedEventArgs, ICoreWebView2BrowserProcessExitedEventArgs,
    ICoreWebView2Controller, ICoreWebView2Environment, ICoreWebView2Environment5,
    COREWEBVIEW2_BROWSER_PROCESS_EXIT_KIND,
};
#[cfg(windows)]
use webview2_com::{AcceleratorKeyPressedEventHandler, BrowserProcessExitedEventHandler};
#[cfg(windows)]
use windows_core::Interface as _;

/// 看门狗只需装一次（三个 WebView 共用同一个浏览器进程）。
#[cfg(windows)]
static WATCHDOG_ONCE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// 给指定 WebView 装 F6/F7 吞键防护。异步：后台线程延迟执行。
#[cfg(windows)]
pub fn guard_webview(webview: &tauri::Webview) {
    let app = webview.app_handle().clone();
    let label = webview.label().to_string();
    use tauri::Manager as _;
    let Some(state) = app.try_state::<std::sync::Arc<crate::controls::AppState>>() else {
        return;
    };
    let state = state.inner().clone();
    std::thread::spawn(move || {
        // 等窗口/WebView 完全就绪，避免和启动期的建窗时序打架
        std::thread::sleep(std::time::Duration::from_millis(1200));
        let log = state.log.clone();
        let Some(w) = app.get_webview(&label) else {
            log.info(&format!("[accel] {label}: 取 WebView 失败，跳过"));
            return;
        };
        install(&w, &state);
    });
}

#[cfg(windows)]
fn install(webview: &tauri::Webview, state: &std::sync::Arc<crate::controls::AppState>) {
    let label = webview.label().to_string();
    let log_out = state.log.clone();
    let log = state.log.clone();
    // 提前取出来克隆：with_webview 的闭包要求 'static，不能借用 state
    let session_log_path = state.session_log_path.clone();

    let res = webview.with_webview(move |w| {
        let controller = w.controller();

        // 只吞 F6/F7；浏览器加速键（F5 刷新 / Ctrl+R / F12 等）保持默认开启
        let log_h = log.clone();
        let label_h = label.clone();
        let handler = AcceleratorKeyPressedEventHandler::create(Box::new(
            move |_sender: Option<ICoreWebView2Controller>,
                  args: Option<ICoreWebView2AcceleratorKeyPressedEventArgs>|
                  -> windows_core::Result<()> {
                if let Some(args) = args {
                    let mut vk: u32 = 0;
                    unsafe {
                        let _ = args.VirtualKey(&mut vk);
                    }
                    if vk == 0x75 || vk == 0x76 {
                        // VK_F6 = 0x75, VK_F7 = 0x76
                        unsafe {
                            let _ = args.SetHandled(true);
                        }
                        log_h.info(&format!("[accel] {label_h}: 已吞掉 VK=0x{vk:02X}"));
                    }
                }
                Ok(())
            },
        ));
        let mut token: i64 = 0;
        unsafe {
            if let Err(e) = controller.add_AcceleratorKeyPressed(&handler, &mut token) {
                log.info(&format!("[accel] {label}: 注册 AcceleratorKeyPressed 失败 {e}"));
            }
        }
        log.info(&format!("[accel] {label}: F6/F7 吞键已安装 token={token}"));

        // ── WebView2 浏览器进程猝死看门狗（只装一次）─────────────────────────────
        // 浏览器进程一死，三个 WebView 就只剩空壳窗口（内容区变空白外框），而**应用进程还活着、
        // 还占着单实例 mutex** —— 用户再双击启动时，新进程只跑到单实例回调就退出，
        // 表现就是「双击没反应 / 启动即空白」，而且 `main.log` 一行都不写，极难排查。
        // 所以这里主动收尸：写一行死因进日志，然后让应用自己退出，把锁让出来，
        // 用户下一次启动就能正常起来。
        if !WATCHDOG_ONCE.swap(true, std::sync::atomic::Ordering::SeqCst) {
            let (exit_tx, exit_rx) = std::sync::mpsc::channel::<String>();
            let handler = BrowserProcessExitedEventHandler::create(Box::new(
                move |_sender: Option<ICoreWebView2Environment>,
                      args: Option<ICoreWebView2BrowserProcessExitedEventArgs>|
                      -> windows_core::Result<()> {
                    let mut kind = COREWEBVIEW2_BROWSER_PROCESS_EXIT_KIND(0);
                    let mut pid: u32 = 0;
                    if let Some(a) = &args {
                        unsafe {
                            let _ = a.BrowserProcessExitKind(&mut kind);
                            let _ = a.BrowserProcessId(&mut pid);
                        }
                    }
                    let _ = exit_tx.send(format!("kind={} browserPid={pid}", kind.0));
                    Ok(())
                },
            ));
            let mut token2: i64 = 0;
            let env = w.environment();
            match env.cast::<ICoreWebView2Environment5>() {
                Ok(env5) => unsafe {
                    match env5.add_BrowserProcessExited(&handler, &mut token2) {
                        Ok(()) => log.info(&format!(
                            "[accel] {label}: 浏览器进程看门狗已安装 token={token2}"
                        )),
                        Err(e) => log.info(&format!("[accel] {label}: 安装看门狗失败 {e}")),
                    }
                },
                Err(e) => log.info(&format!(
                    "[accel] {label}: 取 Environment5 失败（运行库过旧？）{e}"
                )),
            }

            // 在后台线程等“浏览器进程已退出”的通知，然后收尸退出
            let log_w = log.clone();
            let log_path = session_log_path.clone();
            std::thread::spawn(move || {
                let Ok(detail) = exit_rx.recv() else {
                    return;
                };
                let line = format!(
                    "[accel] !! WebView2 浏览器进程已退出（{detail}）：界面已失效，应用将自动退出以免占住单实例锁"
                );
                // 此刻 WebView2 已死，Logger 的写入不保证落盘，直接用路径追加（只写这一处，避免重复）
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log_path)
                {
                    use std::io::Write as _;
                    let secs = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    let _ = writeln!(f, "[{}] INFO {}", crate::util::format_utc(secs), line);
                }
                let _ = &log_w;
                // 留一点时间让日志刷盘，然后硬退出（正常退出路径在 WebView2 死后不可靠）
                std::thread::sleep(std::time::Duration::from_millis(300));
                std::process::exit(0);
            });
        }
    });

    if let Err(e) = res {
        log_out.info(&format!("[accel] with_webview 调用失败: {e}"));
    }
}
