//! DSH Desktop（Tauri 版）入口：装配窗口/托盘/菜单、后端管理、控制通道。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backend;
mod controls;
mod settings;
mod util;

use backend::Backend;
use controls::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, RunEvent, WebviewUrl, WebviewWindowBuilder};

fn main() {
    let app = controls::register_scheme(
        tauri::Builder::default()
            .plugin(tauri_plugin_notification::init())
            .plugin(tauri_plugin_opener::init()),
    )
    .invoke_handler(tauri::generate_handler![controls::dsh_action, controls::get_backend_status])
    // 拦截窗口关闭：弹「退出 / 缩小到托盘」选择框；已选退出则放行
    .on_window_event(|window, event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            handle_close_requested(window, api);
        }
    })
    .setup(|app| {
        let settings = Arc::new(Mutex::new(settings::load_settings()));
        let log = Arc::new(util::Logger::new(settings::logs_dir().join("main.log")));
        log.info(format!("==== DSH Desktop (Tauri) 启动：{} ====", env!("CARGO_PKG_VERSION")).as_str());
        log.info(format!("settings: {:?}", settings.lock().unwrap()).as_str());

        let backend = Arc::new(Backend::new(app.handle().clone(), settings.clone(), log.clone()));
        let state = Arc::new(AppState {
            settings,
            backend: backend.clone(),
            log: log.clone(),
            force_exit: AtomicBool::new(false),
        });
        app.manage(state.clone());

        // 主窗口（loading.html → 后端就绪后 navigate 到 dsh web，注入自绘标题栏）
        create_main_window(app)?;

        // 托盘 + 菜单
        setup_tray(app)?;

        // 后端管理
        backend.start();

        // 就绪后导航到 dsh web；导航后推送一次状态给注入标题栏
        start_navigator(app.handle().clone(), state.clone());

        // 本地 HTTP 控制服务（状态/窗口控制/图标/前端日志打点）
        controls::start_http_server(app.handle().clone(), state.clone());

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
            let state = app_handle.state::<AppState>();
            state.log.info("==== DSH Desktop 退出：终止自有后端进程 ====");
            state.backend.kill_owned();
        }
    });
}

/// 窗口关闭请求（Alt+F4 / 任务栏关闭 / 系统关闭）：已明确退出则放行；
/// 否则拦截并弹出「退出 / 缩小到托盘」选择框（自绘 ✕ 等程序化关闭路径
/// 不经过这里，已在注入脚本/控制服务中直接弹窗）。
fn handle_close_requested(window: &tauri::Window, api: &tauri::CloseRequestApi) {
    let app = window.app_handle();
    let state = app.state::<AppState>();
    if state.force_exit.load(Ordering::SeqCst) {
        return; // 已选择退出 → 放行
    }
    api.prevent_close();
    controls::show_close_dialog(&app.clone());
}

fn create_main_window(app: &tauri::App) -> tauri::Result<()> {
    WebviewWindowBuilder::new(app, "main", WebviewUrl::App("loading.html".into()))
        .title("DSH Desktop")
        .inner_size(1440.0, 900.0)
        .min_inner_size(960.0, 640.0)
        .center()
        // frameless：去掉原生标题栏，由注入脚本自绘标题栏（直接加载 dsh web）
        .decorations(false)
        .shadow(true)
        .initialization_script(controls::inject_js())
        .build()?;
    Ok(())
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
            "restart" => app.state::<AppState>().backend.restart("tray"),
            "browser" => {
                let state = app.state::<AppState>();
                if let Some(u) = state.backend.current_status().url {
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
                let st = app.state::<AppState>();
                st.force_exit.store(true, Ordering::SeqCst);
                st.backend.kill_owned();
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
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
    }
}

/// 后端就绪（running/external）→ 主窗 navigate 到 dsh web；导航后推送状态给注入标题栏。
fn start_navigator(app: tauri::AppHandle, state: Arc<AppState>) {
    std::thread::spawn(move || {
        let mut last_url: Option<String> = None;
        loop {
            let status = state.backend.current_status();
            match (status.state.as_str(), status.url.clone()) {
                ("running" | "external", Some(url)) => {
                    let window = app.get_webview_window("main");
                    if let Some(win) = window {
                        if last_url.as_deref() != Some(url.as_str()) {
                            if let Ok(u) = url.parse::<tauri::Url>() {
                                let _ = win.navigate(u);
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

/// navigate 后延时多次 eval 推送当前状态（幂等；注入脚本会据此更新标题栏/菜单）。
fn push_status_after_navigate(app: tauri::AppHandle, state: Arc<AppState>) {
    std::thread::spawn(move || {
        for delay_ms in [600u64, 1600, 3200, 6000] {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            if let Some(win) = app.get_webview_window("main") {
                let st = state.backend.current_status();
                if let Ok(json) = serde_json::to_string(&st) {
                    let _ = win.eval(&format!(
                        "window.__dshdStatus && window.__dshdStatus({});",
                        json
                    ));
                }
            }
        }
    });
}

