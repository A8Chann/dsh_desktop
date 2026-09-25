//! WebView2 浏览器加速键的「吞键」补丁（Windows 专属）。
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
//! F6 不在 WebView2 文档列出的 `AreBrowserAcceleratorKeysEnabled=false` 覆盖清单里，
//! 所以两件事都做：
//!   1) 关掉浏览器加速键（Ctrl+F/Ctrl+P/F5/F12…，本应用自带刷新按钮，不需要它们）；
//!   2) 用 `AcceleratorKeyPressed` 事件把 F6/F7 直接 `SetHandled(true)` 吃掉，
//!      不让它进 Chromium 的加速键处理路径。
//!
//! 该事件对所有加速键都会触发（与 1) 的开关无关），因此能兜住 F6。
//!
//! ⚠️ 调用时机：**必须**在事件循环跑起来之后、且不要在 setup() 里同步调用。
//! `with_webview` 走的是 `run_on_main_thread`；在 setup()（主线程）里同步调用会死锁，
//! 实测表现为「窗口和三个子 WebView 宿主都建好了，但页面不渲染、日志也不再写」。
//! 所以这里统一从后台线程延迟 1.2s 后直接调用（不绕主线程）。

#[cfg(windows)]
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2AcceleratorKeyPressedEventArgs, ICoreWebView2Controller, ICoreWebView2Settings3,
};
#[cfg(windows)]
use webview2_com::AcceleratorKeyPressedEventHandler;
#[cfg(windows)]
use windows_core::Interface as _;

/// 给指定 WebView 装上「浏览器加速键防护」（含吞掉 F6）。异步：后台线程延迟执行。
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

    let res = webview.with_webview(move |w| {
        let controller = w.controller();
        let core = match unsafe { controller.CoreWebView2() } {
            Ok(c) => c,
            Err(e) => {
                log.info(&format!("[accel] {label}: 取 CoreWebView2 失败 {e}"));
                return;
            }
        };

        // 1) 关掉浏览器加速键（F5/Ctrl+F/Ctrl+P/F12 等；F6 不在该清单内，见文件头）
        match unsafe { core.Settings() }.and_then(|s| s.cast::<ICoreWebView2Settings3>()) {
            Ok(s3) => unsafe {
                if let Err(e) = s3.SetAreBrowserAcceleratorKeysEnabled(false) {
                    log.info(&format!("[accel] {label}: 关闭浏览器加速键失败 {e}"));
                }
            },
            Err(e) => {
                log.info(&format!("[accel] {label}: 取 Settings3 失败 {e}"));
            }
        }

        // 2) 吞掉 F6/F7（F6 = 会打崩浏览器进程的那个键；F7 = 插入符浏览，一律不需要）
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
        log.info(&format!("[accel] {label}: 加速键防护已安装 token={token}"));
    });

    if let Err(e) = res {
        log_out.info(&format!("[accel] with_webview 调用失败: {e}"));
    }
}
