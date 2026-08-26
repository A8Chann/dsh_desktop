//! Desktop controls: titlebar inject script, local HTTP control service,
//! plugin install control channel (agent cooperation) and auto-report.

use crate::backend::Backend;
use crate::settings::{control_dir, Settings};
use crate::util::Logger;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::borrow::Cow;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Manager};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
pub struct AppState {
    pub settings: Arc<Mutex<Settings>>,
    pub backend: Arc<Backend>,
    pub log: Arc<Logger>,
}

// ?????????????????????????????????????????????????????? ???????????????????????Deepseek-Harness-EAC????????????????????????????????????????????????????????

#[allow(dead_code)]
pub const INJECT_JS: &str = r##"
(function () {
  if (window.__dshdInjected) return;
  window.__dshdInjected = true;
  var BAR_ID = '__dsh_desktop_chrome__';
  var BAR_HEIGHT = 36;

  var GLYPHS = {
    menu: '<svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor"><circle cx="2.4" cy="6" r="1.15"/><circle cx="6" cy="6" r="1.15"/><circle cx="9.6" cy="6" r="1.15"/></svg>',
    min: '<svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"><path d="M2.5 6h7"/></svg>',
    max: '<svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.1"><rect x="2.6" y="2.6" width="6.8" height="6.8" rx="1.4"/></svg>',
    restore: '<svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.1"><path d="M4.2 4.2V2.6h5.2v5.2H7.8"/><rect x="2.6" y="4.2" width="5.2" height="5.2" rx="1.2"/></svg>',
    close: '<svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"><path d="M2.6 2.6l6.8 6.8M9.4 2.6l-6.8 6.8"/></svg>'
  };

  // ── EAC 的 CHROME_CSS（原样；--dsw-alias-* 为主题变量，自动贴合 dsh 皮肤）──
  var CHROME_CSS = [
    '#' + BAR_ID + '{position:fixed;top:0;left:0;right:0;height:' + BAR_HEIGHT + 'px;z-index:2147483000;',
    'display:flex;align-items:center;justify-content:space-between;padding:0 6px 0 10px;',
    '-webkit-app-region:drag;user-select:none;box-sizing:border-box;',
    'font-family:var(--dsw-font-family,"Segoe UI","Microsoft YaHei",system-ui,sans-serif);',
    'background:color-mix(in srgb,var(--dsw-alias-bg-base,#0b1220) 74%,transparent);',
    'backdrop-filter:blur(16px) saturate(1.5);-webkit-backdrop-filter:blur(16px) saturate(1.5);',
    'border-bottom:1px solid color-mix(in srgb,var(--dsw-alias-border-l1,rgba(255,255,255,.09)) 55%,transparent)}',
    '#' + BAR_ID + ' .dch-left{display:flex;align-items:center;gap:8px;min-width:0;-webkit-app-region:drag}',
    '#' + BAR_ID + ' .dch-icon{width:20px;height:20px;border-radius:6px;display:block;flex:none;-webkit-app-region:drag;background:transparent;box-shadow:none}',
    '#' + BAR_ID + ' .dch-title{font-size:12.5px;font-weight:600;letter-spacing:.2px;line-height:16px;color:var(--dsw-alias-label-primary,#e6ecff);white-space:nowrap;-webkit-app-region:drag}',
    '#' + BAR_ID + ' .dch-badge{font-size:10px;line-height:14px;padding:1px 6px;border-radius:999px;color:var(--dsw-alias-label-tertiary,#93a5d8);border:1px solid var(--dsw-alias-border-l1,rgba(255,255,255,.09));white-space:nowrap;-webkit-app-region:drag;font-family:var(--ds-font-family-code,Consolas,monospace)}',
    // 状态徽标（比 EAC 多：后端状态显示）
    '#' + BAR_ID + ' .dch-status{display:inline-flex;align-items:center;gap:5px;font-size:10px;line-height:14px;padding:1px 8px;border-radius:999px;color:var(--dsw-alias-label-tertiary,#93a5d8);border:1px solid var(--dsw-alias-border-l1,rgba(255,255,255,.09));white-space:nowrap;-webkit-app-region:drag;font-family:var(--ds-font-family-code,Consolas,monospace)}',
    '#' + BAR_ID + ' .dch-status .dshd-dot{width:6px;height:6px;border-radius:50%;flex:none;background:#94a3b8}',
    '#' + BAR_ID + ' .dch-status .dshd-dot.ok{background:#22c55e;box-shadow:0 0 5px rgba(34,197,94,.8)}',
    '#' + BAR_ID + ' .dch-status .dshd-dot.warn{background:#eab308;box-shadow:0 0 5px rgba(234,179,8,.8)}',
    '#' + BAR_ID + ' .dch-status .dshd-dot.err{background:#ef4444;box-shadow:0 0 5px rgba(239,68,68,.8)}',
    '#' + BAR_ID + ' .dch-right{display:flex;align-items:center;gap:2px;-webkit-app-region:no-drag}',
    '#' + BAR_ID + ' .dch-btn{width:30px;height:28px;display:grid;place-items:center;border:none;border-radius:8px;background:transparent;color:var(--dsw-alias-label-secondary,#b8c5ea);cursor:pointer;padding:0;-webkit-app-region:no-drag;outline:none;transition:background .12s,color .12s}',
    '#' + BAR_ID + ' .dch-btn:hover{background:var(--dsw-alias-interactive-bg-hover,rgba(255,255,255,.09));color:var(--dsw-alias-label-primary,#eef2ff)}',
    '#' + BAR_ID + ' .dch-btn:active{background:var(--dsw-alias-interactive-bg-hover-solid,rgba(255,255,255,.14))}',
    '#' + BAR_ID + ' .dch-close:hover{background:#e81123;color:#fff}',
    '#' + BAR_ID + ' .dch-menu{position:fixed;top:' + (BAR_HEIGHT + 8) + 'px;right:8px;width:272px;z-index:2147483001;-webkit-app-region:no-drag;box-sizing:border-box;padding:6px;',
    'background:var(--dsw-alias-bg-layer-2,color-mix(in srgb,var(--dsw-alias-bg-base,#0b1220) 92%,white));',
    'border:1px solid var(--dsw-alias-border-l1,rgba(255,255,255,.1));border-radius:14px;',
    'box-shadow:0 12px 40px rgba(0,0,0,.5),0 2px 8px rgba(0,0,0,.35);',
    'backdrop-filter:blur(20px) saturate(1.5);-webkit-backdrop-filter:blur(20px) saturate(1.5);',
    'color:var(--dsw-alias-label-primary,#e6ecff);font-family:var(--dsw-font-family,"Segoe UI","Microsoft YaHei",system-ui,sans-serif)}',
    '#' + BAR_ID + ' .dch-mh{padding:8px 10px 10px;border-bottom:1px solid var(--dsw-alias-border-l2,rgba(255,255,255,.08));margin-bottom:6px}',
    '#' + BAR_ID + ' .dch-mh-title{font-size:13px;font-weight:600;display:flex;align-items:center;gap:6px}',
    '#' + BAR_ID + ' .dch-mh-sub{font-size:11px;color:var(--dsw-alias-label-tertiary,#8b9ac4);margin-top:3px;line-height:16px;display:flex;gap:8px;flex-wrap:wrap}',
    '#' + BAR_ID + ' .dch-item{display:flex;align-items:center;gap:8px;width:100%;min-height:30px;padding:5px 10px;border:none;border-radius:8px;background:transparent;color:var(--dsw-alias-label-primary,#dbe4f8);font:inherit;font-size:12.5px;line-height:18px;text-align:left;cursor:pointer;-webkit-app-region:no-drag}',
    '#' + BAR_ID + ' .dch-item:hover{background:var(--dsw-alias-interactive-bg-hover,rgba(255,255,255,.08))}',
    '#' + BAR_ID + ' .dch-item .dch-kbd{margin-left:auto;font-size:10.5px;color:var(--dsw-alias-label-caption,#5f6f9c);font-family:var(--ds-font-family-code,Consolas,monospace)}',
    '#' + BAR_ID + ' .dch-item[data-danger="1"]{color:var(--dsw-alias-state-error-primary,#ff7a85)}',
    '#' + BAR_ID + ' .dch-sep{height:1px;background:var(--dsw-alias-border-l2,rgba(255,255,255,.08));margin:5px 6px}'
  ].join('');

  var menuEl = null, maxBtn = null, menuOpen = false;
  var HTTP = 'http://127.0.0.1:19431';
  // 外部页面按钮：标准 HTTP img 请求到本地控制服务（WebView2 必达 Rust）
  function act(a) { new Image().src = HTTP + '/action?name=' + encodeURIComponent(a); }
  function logWeb(m) { try { new Image().src = HTTP + '/log?msg=' + encodeURIComponent(m); } catch (e) {} }
  function setMaximized(isMax) {
    if (!maxBtn) return;
    maxBtn.innerHTML = isMax ? GLYPHS.restore : GLYPHS.max;
    maxBtn.title = isMax ? '还原' : '最大化';
  }
  function closeMenu() { menuOpen = false; if (menuEl) menuEl.hidden = true; }
  function renderMenu() {
    if (!menuEl) return;
    menuEl.innerHTML = [
      '<div class="dch-mh"><div class="dch-mh-title">DSH Desktop <span style="font-weight:400;color:var(--dsw-alias-label-tertiary)">封装 v1.3.1</span></div>',
      '<div class="dch-mh-sub"><span>后端状态：<span id="dshd-menu-status">连接中…</span></span></div></div>',
      '<button class="dch-item" data-act="restart"><span>重启 Web 服务</span><span class="dch-kbd">重启后端</span></button>',
      '<button class="dch-item" data-act="reload"><span>重新加载</span><span class="dch-kbd">刷新页面</span></button>',
      '<div class="dch-sep"></div>',
      '<button class="dch-item" data-act="browser">在浏览器中打开</button>',
      '<div class="dch-sep"></div>',
      '<button class="dch-item" data-act="about">关于 DSH Desktop</button>',
      '<button class="dch-item" data-danger="1" data-act="quit">退出</button>'
    ].join('');
    menuEl.querySelectorAll('.dch-item').forEach(function (item) {
      item.addEventListener('click', function () {
        var actName = item.getAttribute('data-act');
        closeMenu();
        if (actName === 'quit') { act('close'); return; }
        act(actName);
      });
    });
  }
  function openMenu() {
    if (!menuEl) return;
    renderMenu();
    menuOpen = true;
    menuEl.hidden = false;
    // 菜单重建后：先用缓存状态填充菜单状态行，再拉一次最新状态刷新
    if (window.__lastDshdStatus) window.__dshdStatus(window.__lastDshdStatus);
    fetch(HTTP + '/status').then(function (r) { return r.json(); })
      .then(function (s) { window.__dshdStatus(s); }).catch(function () {});
  }

  function injectChrome() {
    if (document.getElementById(BAR_ID)) return;
    var style = document.createElement('style');
    style.textContent = CHROME_CSS;
    document.head.appendChild(style);

    // 向页面声明自绘标题栏高度（EAC 契约；客户端插件据此刻意避让）
    document.documentElement.setAttribute('data-dsh-title-bar-height', String(BAR_HEIGHT));
    // 内容整体下移，避免遮挡
    var layout = document.createElement('style');
    layout.textContent = 'body{box-sizing:border-box!important;padding-top:' + BAR_HEIGHT + 'px!important}';
    document.head.appendChild(layout);

    var bar = document.createElement('div');
    bar.id = BAR_ID;
    bar.innerHTML =
      '<div class="dch-left">' +
      '<img class="dch-icon" alt="" draggable="false" src="__DSHD_ICON_URI__" />' +
      '<span class="dch-title">DSH Desktop</span>' +
      '<span class="dch-badge">v1.3.1</span>' +
      '<span class="dch-status"><span class="dshd-dot warn" id="dshd-dot"></span><span id="dshd-status-label">连接中…</span></span>' +
      '</div>' +
      '<div class="dch-right">' +
      '<button class="dch-btn" data-act="menu" title="菜单" aria-label="菜单">' + GLYPHS.menu + '</button>' +
      '<button class="dch-btn" data-act="min" title="最小化" aria-label="最小化">' + GLYPHS.min + '</button>' +
      '<button class="dch-btn" data-act="max" title="最大化" aria-label="最大化">' + GLYPHS.max + '</button>' +
      '<button class="dch-btn dch-close" data-act="close" title="关闭" aria-label="关闭">' + GLYPHS.close + '</button>' +
      '</div>' +
      '<div class="dch-menu" hidden></div>';
    document.body.appendChild(bar);

    maxBtn = bar.querySelector('[data-act="max"]');
    menuEl = bar.querySelector('.dch-menu');

    bar.querySelector('[data-act="min"]').addEventListener('click', function () { act('min'); });
    bar.querySelector('[data-act="max"]').addEventListener('click', function () { act('max'); });
    bar.querySelector('.dch-close').addEventListener('click', function () { act('close'); });
    bar.querySelector('[data-act="menu"]').addEventListener('click', function (e) { e.stopPropagation(); if (menuOpen) closeMenu(); else openMenu(); });
    // Tauri 不支持 -webkit-app-region：整栏拖动走 dshd://drag（Rust start_dragging）
    bar.addEventListener('mousedown', function (e) {
      if (e.target.closest('button') || e.target.closest('.dch-menu')) return;
      if (e.button !== 0) return;
      act('drag');
    });
    document.addEventListener('click', function (e) { if (menuOpen && !bar.contains(e.target)) closeMenu(); });
    document.addEventListener('keydown', function (e) { if (e.key === 'Escape') closeMenu(); });
    // 启动自检：探测 remote IPC 通道是否可用（Rust 侧会打日志）
    act('ping');
  }

  window.__dshdStatus = function (s) {
    window.__lastDshdStatus = s;
    var dot = document.getElementById('dshd-dot');
    var label = document.getElementById('dshd-status-label');
    var mlabel = document.getElementById('dshd-menu-status');
    var cls = 'warn', text = '后端未就绪';
    switch (s.state) {
      case 'running': case 'external':
        cls = 'ok';
        text = '运行中 · 端口 ' + (s.port || '?') + (s.state === 'external' ? '（外部实例）' : '');
        break;
      case 'starting':
        cls = 'warn'; text = s.install ? '自动安装中…' : '启动中…';
        break;
      case 'restarting': cls = 'warn'; text = '重启中…'; break;
      case 'error':
        cls = 'err'; text = s.next_retry_sec ? '异常 · ' + s.next_retry_sec + 's 后重试' : '异常';
        break;
      case 'stopped': cls = 'warn'; text = '已停止'; break;
    }
    if (!dot || !label) return;
    dot.className = 'dshd-dot ' + cls;
    label.textContent = text;
    if (mlabel) mlabel.textContent = text;
  };

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', injectChrome);
  else injectChrome();
  // 初始状态：fetch 本地控制服务（CORS）；状态变化由 Rust eval 实时推送
  logWeb('inject loaded, url=' + location.href);
  fetch(HTTP + '/status').then(function (r) { return r.json(); })
    .then(function (s) { window.__dshdStatus(s); }).catch(function () {});
  act('ping');
})();
"##;

/// ??????????????? data URI ?????????????????????????
#[allow(dead_code)]
pub fn inject_js() -> String {
    use base64::Engine as _;
    let png = include_bytes!("../icons/128x128@2x.png");
    let uri = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    );
    INJECT_JS.replace("__DSHD_ICON_URI__", &uri)
}

// ?????????????????????????????????????????????????????? dshd:// ??? ??????????????????????????????????????????????????????

pub fn handle_dshd(
    app: &AppHandle,
    path: &str,
) -> tauri::http::Response<Cow<'static, [u8]>> {
    use tauri::http::Response;
    let json = |body: String| {
        Response::builder()
            .status(200)
            .header("access-control-allow-origin", "*")
            .header("content-type", "application/json")
            .body(Cow::Owned(body.into_bytes()))
            .unwrap_or_else(|_| Response::new(Cow::Owned(b"{}".to_vec())))
    };
    match path {
        "/status" => {
            let state = app.state::<AppState>();
            let st = state.backend.current_status();
            match serde_json::to_string(&st) {
                Ok(s) => json(s),
                Err(_) => json("{}".to_string()),
            }
        }
        "/restart" => {
            let state = app.state::<AppState>();
            state.backend.restart("menu");
            json(r#"{"ok":true}"#.to_string())
        }
        "/reload" => {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.eval("location.reload()");
            }
            json(r#"{"ok":true}"#.to_string())
        }
        "/browser" => {
            let state = app.state::<AppState>();
            let u = state.backend.current_status().url;
            if let Some(u) = u {
                let _ = tauri_plugin_opener::OpenerExt::opener(app).open_url(u, None::<&str>);
            }
            json(r#"{"ok":true}"#.to_string())
        }
        // ???? ???????????????Tauri ??? API ????????????????
        "/min" => {
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(win) = app2.get_webview_window("main") {
                    let r = win.minimize();
                    let _ = r;
                }
            });
            json(r#"{"ok":true}"#.to_string())
        }
        "/max" => {
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(win) = app2.get_webview_window("main") {
                    if win.is_maximized().unwrap_or(false) {
                        let _ = win.unmaximize();
                    } else {
                        let _ = win.maximize();
                    }
                }
            });
            json(r#"{"ok":true}"#.to_string())
        }
        "/close" => {
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(win) = app2.get_webview_window("main") {
                    let _ = win.close();
                }
            });
            json(r#"{"ok":true}"#.to_string())
        }
        "/drag" => {
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(win) = app2.get_webview_window("main") {
                    let _ = win.start_dragging();
                }
            });
            json(r#"{"ok":true}"#.to_string())
        }
        "/icon" => {
            // ????????????????????? .dch-icon ???
            let bytes: &'static [u8] = include_bytes!("../icons/128x128@2x.png");
            Response::builder()
                .status(200)
                .header("access-control-allow-origin", "*")
                .header("content-type", "image/png")
                .header("cache-control", "no-store")
                .body(Cow::Borrowed(bytes))
                .unwrap_or_else(|_| Response::new(Cow::Borrowed(b"{}")))
        }
        _ => {
            let body: Cow<'static, [u8]> = Cow::Borrowed(&b"{}"[..]);
            Response::builder()
                .status(404)
                .header("access-control-allow-origin", "*")
                .body(body)
                .unwrap_or_else(|_| Response::new(Cow::Borrowed(&b"{}"[..])))
        }
    }
}

pub fn register_scheme(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder.register_uri_scheme_protocol("dshd", |ctx, req| {
        let path = req.uri().path().to_string();
        let app = ctx.app_handle().clone();
        handle_dshd(&app, path.as_str())
    })
}

/// ?????????????????? shell ??? invoke??? HTTP /action ?????????????
#[tauri::command]
pub fn dsh_action(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    action: String,
) -> bool {
    state.inner().log.info(&format!("[titlebar] action: {}", action));
    run_action(&app, state.inner(), &action)
}

/// ???????????????????shell ?????????????
#[tauri::command]
pub fn get_backend_status(state: tauri::State<'_, Arc<AppState>>) -> serde_json::Value {
    serde_json::to_value(state.inner().backend.current_status())
        .unwrap_or(serde_json::json!({}))
}

// ?????????????????????????????????????????????????????? ??? HTTP ?????? ??????????????????????????????????????????????????????
// ?????shell ????????status?????JSON????action?name=?????????????// /icon????????PNG????log?msg=??????????????? main.log????// ?????CORS ?????shell ??fetch/img ????????HTTP???????????
pub fn run_action(app: &tauri::AppHandle, state: &Arc<AppState>, action: &str) -> bool {
    match action {
        "min" => {
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(w) = app2.get_webview_window("main") {
                    let _ = w.minimize();
                }
            });
            true
        }
        "max" => {
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(w) = app2.get_webview_window("main") {
                    if w.is_maximized().unwrap_or(false) {
                        let _ = w.unmaximize();
                    } else {
                        let _ = w.maximize();
                    }
                }
            });
            true
        }
        "close" => {
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(w) = app2.get_webview_window("main") {
                    let _ = w.close();
                }
            });
            true
        }
        "drag" => {
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(w) = app2.get_webview_window("main") {
                    let _ = w.start_dragging();
                }
            });
            true
        }
        "restart" => {
            state.backend.restart("titlebar");
            // ?????????????????????????????????????? reload
            let app2 = app.clone();
            let st2 = state.clone();
            std::thread::spawn(move || {
                for _ in 0..50 {
                    std::thread::sleep(std::time::Duration::from_millis(300));
                    let s = st2.backend.current_status();
                    if (s.state == "running" || s.state == "external") && s.url.is_some() {
                        if let Some(w) = app2.get_webview_window("main") {
                            let _ = w.eval("location.reload()");
                        }
                        return;
                    }
                }
            });
            true
        }
        "reload" => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.eval("location.reload()");
            }
            true
        }
        "browser" => {
            if let Some(u) = state.backend.current_status().url {
                let _ = tauri_plugin_opener::OpenerExt::opener(app).open_url(u, None::<&str>);
            }
            true
        }
        "ping" => {
            state.log.info("[http] ping received");
            true
        }
        _ => false,
    }
}

pub fn start_http_server(app: tauri::AppHandle, state: Arc<AppState>) {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    std::thread::spawn(move || {
        let listener = match TcpListener::bind("127.0.0.1:19431") {
            Ok(l) => l,
            Err(e) => {
                state.log.error(&format!("[http] ??? 19431 ???: {}", e));
                return;
            }
        };
        state.log.info("[http] ??????????? http://127.0.0.1:19431");
        for stream in listener.incoming() {
            let Ok(mut s) = stream else { continue };
            let app = app.clone();
            let state = state.clone();
            std::thread::spawn(move || {
                let mut buf = [0u8; 4096];
                let n = match s.read(&mut buf) {
                    Ok(n) if n > 0 => n,
                    _ => return,
                };
                let req = String::from_utf8_lossy(&buf[..n]);
                let first = req.lines().next().unwrap_or("");
                let mut parts = first.split_whitespace();
                let method = parts.next().unwrap_or("");
                let raw_path = parts.next().unwrap_or("/");
                let path = raw_path.split('?').next().unwrap_or("/").to_string();
                let query: String = raw_path.split('?').nth(1).unwrap_or("").to_string();
                let q: std::collections::HashMap<String, String> = query
                    .split('&')
                    .filter(|kv| !kv.is_empty())
                    .filter_map(|kv| {
                        let mut it = kv.splitn(2, '=');
                        Some((it.next()?.to_string(), it.next().unwrap_or("").to_string()))
                    })
                    .collect();

                let cors = "access-control-allow-origin: *\r\n";
                let resp = match (method, path.as_str()) {
                    ("GET", "/status") => {
                        let st = serde_json::to_string(&state.backend.current_status())
                            .unwrap_or_else(|_| "{}".to_string());
                        format!(
                            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n{}\r\n{}",
                            cors, st
                        )
                    }
                    ("GET", "/action") => {
                        let name = url_decode(&q.get("name").cloned().unwrap_or_default());
                        state.log.info(&format!("[http] action: {}", name));
                        let ok = run_action(&app, &state, &name);
                        format!(
                            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n{}\r\n{{\"ok\":{}}}",
                            cors, ok
                        )
                    }
                    ("GET", "/icon") => {
                        let bytes: &'static [u8] = include_bytes!("../icons/128x128@2x.png");
                        let mut head = format!(
                            "HTTP/1.1 200 OK\r\ncontent-type: image/png\r\n{}\r\n",
                            cors
                        )
                        .into_bytes();
                        head.extend_from_slice(bytes);
                        String::from_utf8_lossy(&head).to_string()
                    }
                    ("GET", "/log") => {
                        let msg = url_decode(&q.get("msg").cloned().unwrap_or_default());
                        state.log.info(&format!("[web] {}", msg));
                        format!("HTTP/1.1 200 OK\r\n{}\r\n{{}}", cors)
                    }
                    _ => format!("HTTP/1.1 404 Not Found\r\n{}\r\n{{}}", cors),
                };
                let _ = s.write_all(resp.as_bytes());
                let _ = s.flush();
            });
        }
    });
}

fn url_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v as char);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            out.push(' ');
        } else {
            out.push(bytes[i] as char);
        }
        i += 1;
    }
    out
}

// ?????????????????????????????????????????????????????? ????????gent ???????????????????????????????????????????????????????????

pub fn start_control_watcher(state: Arc<AppState>) {
    std::thread::spawn(move || loop {
        let dir = control_dir();
        let _ = std::fs::create_dir_all(&dir);
        let entries = std::fs::read_dir(&dir)
            .map(|it| it.flatten().map(|e| e.path()).collect::<Vec<_>>())
            .unwrap_or_default();
        for f in entries {
            let name = f.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if name.starts_with("cmd-") && name.ends_with(".json") {
                let content = std::fs::read_to_string(&f).unwrap_or_default();
                let _ = std::fs::remove_file(&f);
                if let Ok(cmd) = serde_json::from_str::<serde_json::Value>(&content) {
                    let id = cmd.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let cmd_name = cmd.get("cmd").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    match cmd_name.as_str() {
                        "cancel-restart" => {}
                        "restart" => state.backend.restart("control"),
                        "install-plugin" => {
                            let spec = cmd.get("spec").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let profile = cmd
                                .get("profile")
                                .and_then(|v| v.as_str())
                                .unwrap_or("web")
                                .to_string();
                            let timeout_ms = cmd
                                .get("timeoutMs")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(300_000);
                            if spec.is_empty() {
                                write_result(&state, &id, &serde_json::json!({
                                    "id": id, "state": "failed", "error": "spec ???"
                                }));
                            } else {
                                let _ = &profile;
                                let _ = timeout_ms;
                                state.log.info(&format!("[install] ??????? {} (id={})", spec, id));
                                write_result(&state, &id, &serde_json::json!({
                                    "id": id, "spec": spec, "state": "installing",
                                    "startedAt": now_iso()
                                }));
                                let result = run_install_plugin(&state, &spec);
                                write_result(&state, &id, &serde_json::json!({
                                    "id": id, "spec": spec, "state": if result { "done" } else { "failed" },
                                    "finishedAt": now_iso(),
                                    "note": "手动重启后端后生效",
                                }));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(400));
    });
}

fn run_install_plugin(state: &Arc<AppState>, spec: &str) -> bool {
    // ??? dsh ???
    let (dsh, node_bin) = {
        let s = state.settings.lock().unwrap();
        let node = s
            .node_bin
            .clone()
            .unwrap_or_else(|| resolve_node_fallback());
        let dsh = s.dsh_bin.clone().or_else(|| find_dsh_on_path());
        (dsh, node)
    };
    let Some(dsh) = dsh else {
        return false;
    };
    // pnpm ????? profile ???
    let dsh_home = std::env::var("DSH_HOME").unwrap_or_else(|_| {
        std::env::var("USERPROFILE")
            .map(|h| format!("{}{}", h, "\\.dsh"))
            .unwrap_or_default()
    });
    let profile_dir = PathBuf::from(&dsh_home).join("profiles").join("web");
    let _ = std::fs::create_dir_all(&profile_dir);
    let out = Command::new(&node_bin)
        .args([
            dsh.as_str(),
            "plugin",
            "--profile",
            "web",
            "add",
            spec,
        ])
        .current_dir(&profile_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(0x08000000)
        .output();
    match out {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}

fn resolve_node_fallback() -> String {
    for c in [
        "C:\\Program Files\\nodejs\\node.exe",
        "C:\\Program Files (x86)\\nodejs\\node.exe",
    ] {
        if std::path::Path::new(c).exists() {
            return c.to_string();
        }
    }
    "node.exe".to_string()
}

fn find_dsh_on_path() -> Option<String> {
    let path = std::env::var("PATH").ok()?;
    for dir in std::env::split_paths(&path) {
        let p = dir.join("node_modules").join("@deepseek-ai").join("dsh").join("lib").join("bin.js");
        if p.exists() {
            return Some(p.to_string_lossy().to_string());
        }
    }
    None
}

fn write_result(_state: &Arc<AppState>, id: &str, result: &serde_json::Value) {
    let dir = control_dir();
    let _ = std::fs::create_dir_all(&dir);
    let f = dir.join(format!("result-{}.json", id));
    let _ = std::fs::write(f, serde_json::to_string_pretty(result).unwrap_or_default());
}

fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs() as i64;
    crate::util::format_utc(secs).replace(' ', "T") + "Z"
}

// ?????????????????????????????????????????????????????? ????????????????????? ??????????????????????????????????????????????????????

pub fn start_plugin_watcher(state: Arc<AppState>) {
    std::thread::spawn(move || {
        let profile_dir = {
            let s = state.settings.lock().unwrap();
            let home = std::env::var("DSH_HOME")
                .ok()
                .or_else(|| {
                    std::env::var("USERPROFILE")
                        .ok()
                        .map(|h| format!("{}\\{}", h, ".dsh"))
                })
                .unwrap_or_default();
            format!("{}\\profiles\\{}", home, s.profile)
        };
        if !std::path::Path::new(&profile_dir).exists() {
            state.log.info(&format!("profile ??????????????????????? {}", profile_dir));
            return;
        }
        let (tx, rx) = channel::<notify::Result<notify::Event>>();
        let mut watcher = match RecommendedWatcher::new(tx, notify::Config::default()) {
            Ok(w) => w,
            Err(e) => {
                state.log.warn(&format!("???????????????: {}", e));
                return;
            }
        };
        if watcher
            .watch(std::path::Path::new(&profile_dir), RecursiveMode::NonRecursive)
            .is_err()
        {
            state.log.warn("????????? watch ???");
            return;
        }
        state.log.info(&format!("?????????????? {}", profile_dir));

        let mut pending: Option<std::time::Instant> = None;
        loop {
            // ???????????????6s ????????????
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(Ok(ev)) => {
                    let changed = ev.paths.iter().any(|p| {
                        p.file_name()
                            .map(|n| n == "package.json" || n == "pnpm-lock.yaml")
                            .unwrap_or(false)
                    });
                    if changed {
                        pending = Some(std::time::Instant::now());
                    }
                }
                Ok(Err(_)) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {}
            }
            if let Some(t) = pending {
                if t.elapsed() >= Duration::from_secs(6) {
                    pending = None;
                    state.backend.on_plugin_change();
                }
            }
        }
    });
}

// ?????????????????????????????????????????????????????? ??????????????? ??????????????????????????????????????????????????????

pub fn start_auto_report(state: Arc<AppState>) {
    std::thread::spawn(move || loop {
        // ??? running ??? URL ???????????? result ???
        let status = state.backend.current_status();
        if status.state == "running" || status.state == "external" {
            if let Some(base) = status.url.clone() {
                report_pending(state.clone(), base);
            }
        }
        std::thread::sleep(Duration::from_secs(5));
    });
}

fn report_pending(state: Arc<AppState>, base: String) {
    let dir = control_dir();
    let entries = match std::fs::read_dir(&dir) {
        Ok(it) => it.flatten().map(|e| e.path()).collect::<Vec<_>>(),
        Err(_) => return,
    };
    for f in entries {
        let name = f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if !(name.starts_with("result-") && name.ends_with(".json")) {
            continue;
        }
        let content = match std::fs::read_to_string(&f) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let v: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if v.get("reportedAt").is_some() {
            continue;
        }
        if v.get("state")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            != "done"
        {
            continue;
        }
        if let Some(session_id) = latest_session(&base) {
            let text = format!(
                "【系统】插件安装已完成，请向用户简洁汇报（汇报后删除该 result 文件：{}）",
                f.display()
            );
            if post_prompt(&base, &session_id, &text) {
                // ??? reportedAt
                let mut nv = v;
                nv["reportedAt"] = serde_json::json!(now_iso());
                let _ = std::fs::write(&f, serde_json::to_string_pretty(&nv).unwrap_or_default());
            }
        }
        state.log.info(&format!("[auto-report] ??? result ???: {}", name));
    }
}

fn latest_session(base: &str) -> Option<String> {
    let body = json_rpc("session.list", &serde_json::json!({}));
    let res = ureq_post(base, "session.list", &body)?;
    let v: serde_json::Value = serde_json::from_str(&res).ok()?;
    let items = v.pointer("/result/value/items")?.as_array()?;
    let mut items = items.clone();
    items.sort_by(|a, b| {
        let au = a.get("updatedAt").and_then(|x| x.as_u64()).unwrap_or(0);
        let bu = b.get("updatedAt").and_then(|x| x.as_u64()).unwrap_or(0);
        bu.cmp(&au)
    });
    items
        .first()?
        .get("sessionId")?
        .as_str()
        .map(|s| s.to_string())
}

fn post_prompt(base: &str, session_id: &str, text: &str) -> bool {
    let body = json_rpc(
        "session.prompt",
        &serde_json::json!({ "sessionId": session_id, "mode": "queue", "content": [{ "type": "text", "text": text }] }),
    );
    ureq_post(base, "session.prompt", &body).is_some()
}

fn json_rpc(method: &str, payload: &serde_json::Value) -> String {
    serde_json::json!({
        "type": "client-request",
        "rpcId": format!("dshd-rust-{}-{}", std::process::id(), now_iso()),
        "method": method,
        "payload": payload
    })
    .to_string()
}

fn ureq_post(base: &str, method: &str, body_json: &str) -> Option<String> {
    let url = format!("{}/api/{}", base.trim_end_matches('/'), method);
    ureq::post(&url)
        .set("content-type", "application/json")
        .timeout(Duration::from_secs(10))
        .send_string(body_json)
        .ok()?
        .into_string()
        .ok()
}

