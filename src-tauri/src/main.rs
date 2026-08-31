//! DSH Desktop（Tauri 版）入口：装配窗口/托盘/菜单、后端管理、控制通道。
//! 架构（v1.9+）：一个 frameless 壳窗口 = 顶部独立 titlebar chrome WebView + 内容区
//! DSH/DeepSeek 两个独立内容 WebView；菜单/退出/下载为独立弹窗 WebViewWindow。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backend;
mod controls;
mod downloads;
mod environments;
mod settings;
mod util;
#[cfg(windows)]
mod win_toast;

use backend::Backend;
use controls::AppState;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{LogicalPosition, LogicalSize, Manager, RunEvent, WebviewUrl};

fn main() {
    // panic hook：崩溃前把 panic 位置写入 %APPDATA%\DSH Desktop\logs\panics.log，
    // 用于定位偶发 fail-fast（0xc0000409，如 WebView2 下载链路）的准确 panic 点。
    {
        use std::io::Write as _;
        std::panic::set_hook(Box::new(|info| {
            let mut msg = format!("PANIC: {}\n", info);
            // 尽力输出调用栈（release 下含模块偏移，配合符号表可定位）
            let bt = std::backtrace::Backtrace::force_capture();
            msg.push_str(&format!("BACKTRACE:\n{}", bt));
            let _ = std::io::Write::write_all(&mut std::io::stderr(), msg.as_bytes());
            if let Ok(appdata) = std::env::var("APPDATA") {
                let p = std::path::PathBuf::from(&appdata)
                    .join("DSH Desktop")
                    .join("logs")
                    .join("panics.log");
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&p)
                {
                    let _ = f.write_all(msg.as_bytes());
                }
            }
        }));
    }
    let app = controls::register_scheme(
        tauri::Builder::default()
            // 单实例：必须最先注册。重复启动（双击两次图标等）时聚焦已有窗口后退出，
            // 否则第二个实例的 19431 控制服务绑定失败，其标题栏按钮会全部操作到第一个窗口上。
            .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
                show_main_window(app);
            }))
            .plugin(tauri_plugin_notification::init())
            .plugin(tauri_plugin_opener::init()),
    )
    .invoke_handler(tauri::generate_handler![controls::dsh_action, controls::get_backend_status])
    // 拦截窗口关闭：弹「退出 / 缩小到托盘」选择框（独立弹窗）；已选退出则放行
    .on_window_event(|window, event| match event {
        tauri::WindowEvent::CloseRequested { api, .. } => {
            handle_close_requested(window, api);
        }
        // 壳窗口移动/缩放：重排内容子 WebView + 重算外壳层覆盖范围（跟手）
        tauri::WindowEvent::Moved { .. } | tauri::WindowEvent::Resized { .. } => {
            controls::on_shell_resize(window.app_handle());
        }
        _ => {}
    })
    .setup(|app| {
        // 启动时把当前 dsh（手动路径/全局）登记进环境管理，保证面板有记录
        let mut loaded = settings::load_settings();
        crate::environments::ensure_seed_versions(&mut loaded);
        crate::settings::save_settings(&loaded);
        let settings = Arc::new(Mutex::new(loaded));
        let log = Arc::new(util::Logger::new(settings::logs_dir().join("main.log")));
        log.info(format!("==== DSH Desktop (Tauri) 启动：{} ====", env!("CARGO_PKG_VERSION")).as_str());
        log.info(format!("settings: {:?}", settings.lock().unwrap()).as_str());

        let backend = Arc::new(Backend::new(app.handle().clone(), settings.clone(), log.clone()));
        let state = Arc::new(AppState {
            settings,
            backend: backend.clone(),
            log: log.clone(),
            force_exit: AtomicBool::new(false),
            deepseek_shown: AtomicBool::new(false),
            popup_menu_visible: AtomicBool::new(false),
            popup_close_visible: AtomicBool::new(false),
            popup_downloads_visible: AtomicBool::new(false),
            popup_settings_visible: AtomicBool::new(false),
            popup_env_visible: AtomicBool::new(false),
            env_task: Mutex::new(None),
            env_last_error: Mutex::new(None),
            last_popup_shown_ms: AtomicU64::new(0),
            deepseek_loaded: AtomicBool::new(false),
            theme_dsh: Mutex::new(None),
            theme_deepseek: Mutex::new(None),
            window_revealed: AtomicBool::new(false),
            token: controls::generate_token(),
        });
        app.manage(state.clone());
        // 自管下载器（拦截 WebView2 下载后由 Rust 线程下载，支持进度/暂停/取消）
        // notify：下载状态每次变化立即推给外壳层（徽标/面板即时刷新，不依赖轮询）
        // on_done：单个任务完成时发系统通知（后台下载完了用户不必盯着徽标）
        {
            let app2 = app.handle().clone();
            let notify = Arc::new(move || {
                if let Some(c) = app2.get_webview("chrome") {
                    let _ = c.eval("window.__dshdDlChanged && window.__dshdDlChanged();");
                }
            });
            let log2 = log.clone();
            let on_done = Arc::new(move |name: &str| {
                log2.info(&format!("[download] 完成通知: {}", name));
                #[cfg(windows)]
                {
                    if let Err(e) = win_toast::show_download_done_toast(name) {
                        log2.info(&format!("下载完成 toast 发送失败: {e}"));
                    }
                }
            });
            app.manage(downloads::Downloads::new(log.clone(), notify, on_done));
        }

        // 开始菜单快捷方式（AUMID + 应用图标）：一次性创建/刷新。
        // 非打包应用 toast 图标的官方途径；必须在主线程启动期做（COM），
        // 不能放到 toast 的 watcher 线程（堆损坏崩溃 0xc0000374，实测）。
        #[cfg(windows)]
        {
            if let Some(state) = app.try_state::<Arc<AppState>>() {
                if let Err(e) = win_toast::ensure_start_menu_shortcut() {
                    state.log.info(&format!("开始菜单快捷方式创建失败: {e}"));
                }
            }
        }

        // 壳窗口：标题栏 chrome WebView + DSH 内容 WebView；DeepSeek 内容 WebView 首次切换时懒建
        create_main_window(app)?;

        // 托盘 + 菜单
        setup_tray(app)?;

        // 后端管理
        backend.start();

        // 就绪后导航 DSH 内容 WebView；导航后推送一次状态给 chrome 标题栏
        start_navigator(app.handle().clone(), state.clone());

        // 本地 HTTP 控制服务（状态/窗口控制/图标/前端日志打点/主题）
        controls::start_http_server(app.handle().clone(), state.clone());

        // 主题由 DSH 页内主题桥事件驱动上报（见 dsh WebView 的 theme_bridge_js），无 Rust 轮询

        // 控制通道（agent 协作）、插件变更监控与安装结果自动汇报
        controls::start_control_watcher(state.clone());
        controls::start_plugin_watcher(state.clone());
        controls::start_auto_report(state.clone());

        Ok(())
    })
    .build(tauri::generate_context!())
    .expect("error while building DSH Desktop");

    // 退出前清理：结束自有 dsh 后端进程树，避免遗留后台进程
    app.run(|app_handle, event| {
        if let RunEvent::Exit = event {
            if let Some(state) = app_handle.try_state::<Arc<AppState>>() {
                state.log.info("==== DSH Desktop 退出：终止自有后端进程 ====");
                state.backend.kill_owned();
            }
        }
    });
}

/// 窗口关闭请求（Alt+F4 / 任务栏关闭 / 系统关闭）：已明确退出则放行；
/// 否则拦截并弹出「退出 / 缩小到托盘」选择框（独立弹窗窗口）。
fn handle_close_requested(window: &tauri::Window, api: &tauri::CloseRequestApi) {
    let app = window.app_handle();
    // 关闭请求可能在状态未就绪时到达（启动早期）：降级为直接放行，绝不 panic
    let Some(state) = app.try_state::<Arc<AppState>>() else {
        return;
    };
    if state.force_exit.load(Ordering::SeqCst) {
        return; // 已选择退出 → 放行
    }
    api.prevent_close();
    controls::show_close_dialog(&app.clone());
}

/// 壳窗口：一个 frameless Window + 标题栏 chrome 子 WebView（36px 条）+
/// DSH 内容子 WebView（标题栏下方）。DeepSeek 内容子 WebView 由 controls 懒建。
fn create_main_window(app: &tauri::App) -> tauri::Result<()> {
    let window = tauri::WindowBuilder::new(app, "main")
        .title("DSH Desktop")
        .inner_size(1440.0, 900.0)
        .min_inner_size(960.0, 640.0)
        .center()
        .decorations(false)
        .shadow(true)
        .transparent(true) // 标题栏区域透出窗口材质（内容区被子 WebView 盖住不受影响）
        .visible(false) // 先隐藏：透明窗口在子 WebView 渲染前会闪一下桌面，就绪后再 show
        .build()?;

    // 窗口材质（Acrylic 亚克力 / Mica 云母 / 无）：首帧先按设置铺一层，
    // 内容页主题桥上报后由 push_theme → apply_window_effect 按真实主题刷新。
    {
        use tauri::window::{Effect, EffectsBuilder};
        let material = app
            .try_state::<Arc<AppState>>()
            .map(|s| s.settings.lock().unwrap().window_material.to_lowercase())
            .unwrap_or_else(|| "acrylic".to_string());
        let effect = match material.as_str() {
            "none" => None,
            "mica" => Some(Effect::MicaDark),
            _ => Some(Effect::Acrylic),
        };
        let ok = match effect {
            Some(e) => window
                .set_effects(EffectsBuilder::new().effect(e).build())
                .is_ok(),
            None => false,
        };
        if !ok {
            let _ = window.set_background_color(Some(tauri::window::Color(11, 18, 32, 255)));
        }
    }

    // 子 WebView 初始尺寸按窗口真实尺寸算：写死 1440x864 会在小屏 / DPI 缩放下
    // 首帧错位，要等第一次 Moved/Resized 才被 relayout 纠正。
    let scale = window.scale_factor().unwrap_or(1.0);
    let phys = window.inner_size().unwrap_or(tauri::PhysicalSize::new(1440, 900));
    let win_w = phys.width as f64 / scale;
    let win_h = phys.height as f64 / scale;
    let content_h = (win_h - 36.0).max(1.0);

    // 控制服务令牌：只注入自家页面，外部网页拿不到 → 无法调用受保护接口
    let token = app
        .try_state::<Arc<AppState>>()
        .map(|s| s.token.clone())
        .unwrap_or_default();
    let token_script = format!("window.__DSHD_K = '{}';", token);

    // DSH 内容页（标题栏下方整块内容区）：loading.html → 后端就绪后导航到 dsh web
    // 注入：主题桥（事件驱动上报主题）+ 点击转发（点 DSH 内容 = “点外部”，关弹窗）
    let dsh = tauri::WebviewBuilder::new("dsh", WebviewUrl::App("loading.html".into()))
        .initialization_script(token_script.clone())
        .initialization_script(controls::theme_bridge_js("dsh"))
        .initialization_script(controls::click_forwarder_js(&token))
        .initialization_script(controls::switch_loading_js())
        .on_download(controls::intercept_download);
    window.add_child(dsh, LogicalPosition::new(0.0, 36.0), LogicalSize::new(win_w, content_h))?;

    // DeepSeek 内容页：启动即创建（占位页、隐藏），首次切换才导航到 chat.deepseek.com
    // 必须在外壳层之前创建（子 WebView 后创建者在上层，外壳层要保持在最上）
    // UA 覆盖为桌面 Chrome：WebView2 默认 UA 带 "Edg/" 会被 DeepSeek 风控识别为非常规环境
    // 注入：主题桥（src=deepseek，标题栏跟随 Chat 页配色）+ 点击转发（点内容关弹层，与 dsh 页一致）
    let deepseek = tauri::WebviewBuilder::new("deepseek", WebviewUrl::App("empty.html".into()))
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36",
        )
        .initialization_script(controls::theme_bridge_js("deepseek"))
        .initialization_script(controls::click_forwarder_js(&token))
        .on_download(controls::intercept_download);
    let ds_webview = window.add_child(deepseek, LogicalPosition::new(0.0, 36.0), LogicalSize::new(win_w, content_h))?;
    let _ = ds_webview.hide();

    // 外壳层：最后创建（保持在最上层）。透明 WebView：空闲仅 36px 条；
    // 弹层打开时由 Rust 扩展其覆盖范围（透明，内容透过可见，弹层画在这层）
    let chrome = tauri::WebviewBuilder::new("chrome", WebviewUrl::App("chrome.html".into()))
        .initialization_script(token_script)
        .transparent(true);
    window.add_child(chrome, LogicalPosition::new(0.0, 0.0), LogicalSize::new(win_w, 36.0))?;

    // 兜底：即使 chrome 页加载失败（不会发 ping），也要保证窗口最终可见
    {
        let app2 = app.handle().clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(1500));
            let app3 = app2.clone();
            let _ = app2.run_on_main_thread(move || {
                reveal_main_window(&app3);
            });
        });
    }
    Ok(())
}

/// 首次显示主窗口（幂等）：由 chrome 页的 ping 触发，或启动 1.5s 后兜底触发。
pub fn reveal_main_window(app: &tauri::AppHandle) {
    let Some(state) = app.try_state::<Arc<AppState>>() else {
        return;
    };
    if state.window_revealed.swap(true, Ordering::SeqCst) {
        return; // 已显示过
    }
    controls::on_shell_resize(app); // 按真实尺寸校准一次子 WebView 布局
    if let Some(win) = app.get_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let show_i = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let restart_i = MenuItem::with_id(app, "restart", "重启后端", true, None::<&str>)?;
    let browser_i = MenuItem::with_id(app, "browser", "在系统浏览器打开", true, None::<&str>)?;
    let logs_i = MenuItem::with_id(app, "logs", "打开日志目录", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &show_i,
            &PredefinedMenuItem::separator(app)?,
            &restart_i,
            &browser_i,
            &logs_i,
            &PredefinedMenuItem::separator(app)?,
            &quit_i,
        ],
    )?;
    // 托盘图标用 32x32 小图标（点对点清晰；大图缩到托盘反而模糊）
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))
        .unwrap_or_else(|_| tauri::image::Image::new(include_bytes!("../icons/icon.ico") as &[u8], 256, 256));
    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("DSH Desktop")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "restart" => {
                if let Some(s) = app.try_state::<Arc<AppState>>() {
                    s.backend.restart("tray");
                }
            }
            "browser" => {
                let u = app
                    .try_state::<Arc<AppState>>()
                    .map(|s| s.backend.current_status().url)
                    .flatten();
                if let Some(u) = u {
                    let _ = tauri_plugin_opener::OpenerExt::opener(app).open_url(u, None::<&str>);
                }
            }
            "logs" => {
                let dir = settings::logs_dir();
                let _ = tauri_plugin_opener::OpenerExt::opener(app)
                    .open_path(dir.to_string_lossy().to_string(), None::<&str>);
            }
            "quit" => {
                // 先同步杀自有后端进程树再退出（Exit 事件回调不可靠，见 run_action quit 注释）
                if let Some(st) = app.try_state::<Arc<AppState>>() {
                    st.force_exit.store(true, Ordering::SeqCst);
                    st.log.info("[tray] 用户选择退出：开始清理后端");
                    st.backend.kill_owned();
                    st.log.info("[tray] 后端清理完成，请求退出应用");
                } else {
                    eprintln!("[tray] quit: 状态未就绪");
                }
                app.exit(0);
            }
            _ => {}
        })
        // 双击（或左键单击）托盘图标：显示主窗口
        .on_tray_icon_event(|tray, event| {
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
            let show = matches!(
                event,
                TrayIconEvent::DoubleClick { button: MouseButton::Left, .. }
                    | TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    }
            );
            if show {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

/// 显示并聚焦主窗口。
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(win) = app.get_window("main") {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
    }
}

/// 后端就绪（running/external）→ DSH 内容 WebView 导航到 dsh web；导航后推送状态给 chrome 标题栏。
fn start_navigator(app: tauri::AppHandle, state: Arc<AppState>) {
    std::thread::spawn(move || {
        let mut last_url: Option<String> = None;
        loop {
            let status = state.backend.current_status();
            match (status.state.as_str(), status.url.clone()) {
                ("running" | "external", Some(url)) => {
                    if let Some(wv) = app.get_webview("dsh") {
                        if last_url.as_deref() != Some(url.as_str()) {
                            if let Ok(u) = url.parse::<tauri::Url>() {
                                let _ = wv.navigate(u);
                                last_url = Some(url.clone());
                                push_status_after_navigate(app.clone(), state.clone());
                            }
                        }
                    }
                }
                _ => {}
            }
            std::thread::sleep(std::time::Duration::from_millis(700));
        }
    });
}

/// navigate 后在几个时间点把当前状态推给 chrome 标题栏（幂等；覆盖页面加载慢的情况）。
fn push_status_after_navigate(app: tauri::AppHandle, state: Arc<AppState>) {
    std::thread::spawn(move || {
        let start = std::time::Instant::now();
        // 绝对时间点（相对 navigate 时刻），不是逐个累加的间隔
        for at_ms in [600u64, 1600, 3200, 6000] {
            let elapsed = start.elapsed().as_millis() as u64;
            if at_ms > elapsed {
                std::thread::sleep(std::time::Duration::from_millis(at_ms - elapsed));
            }
            if let Some(wv) = app.get_webview("chrome") {
                let st = state.backend.current_status();
                if let Ok(json) = serde_json::to_string(&st) {
                    let _ = wv.eval(&format!(
                        "window.__dshdStatus && window.__dshdStatus({});",
                        json
                    ));
                }
            }
        }
    });
}