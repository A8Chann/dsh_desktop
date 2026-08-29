//! Desktop controls: titlebar inject script, local HTTP control service,
//! plugin install control channel (agent cooperation) and auto-report.

use crate::backend::Backend;
use crate::downloads::Downloads;
use crate::settings::{control_dir, Settings};
use crate::util::Logger;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::borrow::Cow;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::window::{Effect, EffectsBuilder};
use tauri::WebviewBuilder;
use tauri::{AppHandle, Manager, WebviewUrl};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
pub struct AppState {
    pub settings: Arc<Mutex<Settings>>,
    pub backend: Arc<Backend>,
    pub log: Arc<Logger>,
    /// 用户已明确选择退出（托盘/菜单/弹窗）→ 放行窗口关闭，不再拦截。
    pub force_exit: AtomicBool,
    /// 当前是否显示在 DeepSeek 内容页（标题栏「切换」拨片状态位）。
    pub deepseek_shown: AtomicBool,
    /// 菜单弹窗窗口是否可见。
    pub popup_menu_visible: AtomicBool,
    /// 退出选择弹窗窗口是否可见。
    pub popup_close_visible: AtomicBool,
    /// 下载管理弹窗窗口是否可见。
    pub popup_downloads_visible: AtomicBool,
    /// 下载设置弹层是否可见。
    pub popup_settings_visible: AtomicBool,
    /// 最近一次弹窗显示时间（毫秒时间戳）：用于弹层守卫（打开瞬间不计为“点外部”）。
    pub last_popup_shown_ms: AtomicU64,
    /// DeepSeek 内容页是否已完成首次导航（chat.deepseek.com）。
    pub deepseek_loaded: AtomicBool,
    /// 各内容页最近上报的主题色（bg, fg）：切页时按当前显示页取用。
    pub theme_dsh: Mutex<Option<(String, String)>>,
    pub theme_deepseek: Mutex<Option<(String, String)>>,
}

impl AppState {
    /// 当前显示页（dsh / deepseek）的主题缓存。
    pub fn current_theme(&self) -> Option<(String, String)> {
        if self.deepseek_shown.load(Ordering::SeqCst) {
            self.theme_deepseek.lock().unwrap().clone()
        } else {
            self.theme_dsh.lock().unwrap().clone()
        }
    }
}

// ?????????????????????????????????????????????????????? ???????????????????????Deepseek-Harness-EAC????????????????????????????????????????????????????????

#[allow(dead_code)]
pub const INJECT_JS: &str = r##"
(function () {
  if (window.__dshdInjected) return;
  window.__dshdInjected = true;
  var BAR_ID = '__dsh_desktop_chrome__';
  var BAR_HEIGHT = 36;
  // 区分主窗（DSH）与副窗（chat.deepseek.com）：副窗标题栏只保留「切换」等窗口按钮，不显示 DSH 品牌/后端状态
  var isDeepseek = /chat\.deepseek\.com/.test(location.hostname) || /deepseek\.com/.test(location.href);
  var APP_TITLE = isDeepseek ? 'DeepSeek 聊天' : 'DSH Desktop';
  var APP_BADGE = isDeepseek ? 'DeepSeek' : 'v1.9.0';

  var GLYPHS = {
    menu: '<svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor"><circle cx="2.4" cy="6" r="1.15"/><circle cx="6" cy="6" r="1.15"/><circle cx="9.6" cy="6" r="1.15"/></svg>',
    min: '<svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"><path d="M2.5 6h7"/></svg>',
    max: '<svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.1"><rect x="2.6" y="2.6" width="6.8" height="6.8" rx="1.4"/></svg>',
    restore: '<svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.1"><path d="M4.2 4.2V2.6h5.2v5.2H7.8"/><rect x="2.6" y="4.2" width="5.2" height="5.2" rx="1.2"/></svg>',
    close: '<svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"><path d="M2.6 2.6l6.8 6.8M9.4 2.6l-6.8 6.8"/></svg>',
    toggle: '<svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"><path d="M2.5 4h6M7 2l2.2 2L7 6M9.5 8h-6M5 6l-2.2 2L5 10"/></svg>'
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
    '#' + BAR_ID + ' .dch-sep{height:1px;background:var(--dsw-alias-border-l2,rgba(255,255,255,.08));margin:5px 6px}',
    '#' + BAR_ID + ' .dch-switch{width:34px;height:18px;border-radius:999px;border:1px solid var(--dsw-alias-border-l1,rgba(255,255,255,.14));background:var(--dsw-alias-interactive-bg-hover,rgba(255,255,255,.16));position:relative;cursor:pointer;padding:0;flex:none;-webkit-app-region:no-drag;transition:background .15s,border-color .15s}',
    '#' + BAR_ID + ' .dch-switch::after{content:"";position:absolute;top:2px;left:2px;width:12px;height:12px;border-radius:50%;background:#dbe4f8;transition:transform .15s}',
    '#' + BAR_ID + ' .dch-switch.on{background:#16a34a;border-color:rgba(34,197,94,.55)}',
    '#' + BAR_ID + ' .dch-switch.on::after{transform:translateX(16px);background:#fff}',
    '#' + BAR_ID + ' .dch-switch:hover{filter:brightness(1.12)}'
  ].join('');

  var menuEl = null, maxBtn = null, menuOpen = false;
  // ── 关闭选择框（Rust 拦截窗口关闭后调用 __dshdShowCloseDialog 弹出）──
  // 注意：弹窗挂在 document.body 下（不是标题栏 #BAR_ID 的后代），
  // 所以 CSS 必须用弹窗自己的 id 选择器，不能用 '#BAR_ID .xxx'（会失配导致裸文本）。
  var CLOSE_DLG_ID = '__dshd_close_overlay__';
  var closeDlgBuilt = false, closeDlgEl = null, closeDlgOpen = false;
  var CLOSE_DLG_CSS = [
    '#' + CLOSE_DLG_ID + '[hidden]{display:none!important}',
    '#' + CLOSE_DLG_ID + '{position:fixed;inset:0;z-index:2147483002;display:grid;place-items:center;',
    'background:rgba(3,7,16,.55);backdrop-filter:blur(5px);-webkit-backdrop-filter:blur(5px);-webkit-app-region:no-drag}',
    '#' + CLOSE_DLG_ID + ' .dch-close-box{width:390px;box-sizing:border-box;padding:18px 18px 14px;border-radius:16px;',
    'background:var(--dsw-alias-bg-layer-2,color-mix(in srgb,var(--dsw-alias-bg-base,#0b1220) 94%,white));',
    'border:1px solid var(--dsw-alias-border-l1,rgba(255,255,255,.12));',
    'box-shadow:0 18px 60px rgba(0,0,0,.55),0 2px 10px rgba(0,0,0,.4);',
    'font-family:var(--dsw-font-family,"Segoe UI","Microsoft YaHei",system-ui,sans-serif);color:var(--dsw-alias-label-primary,#e6ecff)}',
    '#' + CLOSE_DLG_ID + ' .dch-close-box h3{margin:0 0 6px;font-size:14px;font-weight:600;letter-spacing:.2px}',
    '#' + CLOSE_DLG_ID + ' .dch-close-box p{margin:0 0 16px;font-size:12px;line-height:19px;color:var(--dsw-alias-label-tertiary,#93a5d8)}',
    '#' + CLOSE_DLG_ID + ' .dch-close-actions{display:flex;gap:8px;justify-content:flex-end}',
    '#' + CLOSE_DLG_ID + ' .dch-close-actions button{flex:1;min-width:88px;padding:7px 10px;border-radius:9px;border:1px solid var(--dsw-alias-border-l1,rgba(255,255,255,.12));',
    'background:transparent;color:var(--dsw-alias-label-primary,#dbe4f8);font:inherit;font-size:12.5px;cursor:pointer;transition:background .12s}',
    '#' + CLOSE_DLG_ID + ' .dch-close-actions button:hover{background:var(--dsw-alias-interactive-bg-hover,rgba(255,255,255,.1))}',
    '#' + CLOSE_DLG_ID + ' .dch-close-actions .dch-quit{background:rgba(232,17,35,.14);border-color:rgba(232,17,35,.45);color:#ff7a85}',
    '#' + CLOSE_DLG_ID + ' .dch-close-actions .dch-quit:hover{background:rgba(232,17,35,.24)}'
  ].join('');
  function ensureCloseDlg() {
    if (closeDlgBuilt) return;
    closeDlgBuilt = true;
    var style = document.createElement('style');
    style.textContent = CLOSE_DLG_CSS;
    document.head.appendChild(style);
    var box = document.createElement('div');
    box.id = CLOSE_DLG_ID;
    box.className = 'dch-overlay';
    box.hidden = true;
    box.innerHTML =
      '<div class="dch-close-box">' +
      '<h3>关闭 DSH Desktop</h3>' +
      '<p>选择关闭方式：缩小到托盘后应用仍在后台运行（后端不中断），可随时从托盘图标重新打开。</p>' +
      '<div class="dch-close-actions">' +
      '<button data-c="cancel">取消</button>' +
      '<button data-c="tray">缩小到托盘</button>' +
      '<button data-c="quit" class="dch-quit">退出</button>' +
      '</div></div>';
    box.querySelector('[data-c="cancel"]').addEventListener('click', hideCloseDlg);
    box.querySelector('[data-c="tray"]').addEventListener('click', function () { hideCloseDlg(); act('min-tray'); });
    box.querySelector('[data-c="quit"]').addEventListener('click', function () { hideCloseDlg(); act('quit'); });
    box.addEventListener('click', function (e) { if (e.target === box) hideCloseDlg(); });
    document.body.appendChild(box);
    closeDlgEl = box;
  }
  function hideCloseDlg() { closeDlgOpen = false; if (closeDlgEl) closeDlgEl.hidden = true; }
  // Rust 在拦截到关闭请求时调用；幂等：已弹出则不重复叠加
  window.__dshdShowCloseDialog = function () {
    if (closeDlgOpen) return;
    closeDlgOpen = true;
    ensureCloseDlg();
    closeDlgEl.hidden = false;
  };

  // ── 下载管理面板（自管下载器 /downloads + download-* action）──
  var DLS_ID = '__dshd_downloads__';
  var dlPanelBuilt = false, dlPanelEl = null;
  // 右上角下拉卡片（类 Edge 下载浮层）：无全屏遮罩，标题栏下方右对齐
  var DLS_CSS = [
    '#' + DLS_ID + '[hidden]{display:none!important}',
    '#' + DLS_ID + '{position:fixed;top:44px;right:12px;width:400px;max-width:calc(100vw - 24px);z-index:2147483002;',
    '-webkit-app-region:no-drag;animation:dshdDlsIn .14s ease-out}',
    '@keyframes dshdDlsIn{from{opacity:0;transform:translateY(-6px)}to{opacity:1;transform:none}}',
    '#' + DLS_ID + ' .dch-dls-box{box-sizing:border-box;padding:12px 14px;border-radius:14px;',
    'background:var(--dsw-alias-bg-layer-2,color-mix(in srgb,var(--dsw-alias-bg-base,#0b1220) 96%,white));',
    'border:1px solid var(--dsw-alias-border-l1,rgba(255,255,255,.12));',
    'box-shadow:0 16px 48px rgba(0,0,0,.5),0 2px 10px rgba(0,0,0,.35);',
    'font-family:var(--dsw-font-family,"Segoe UI","Microsoft YaHei",system-ui,sans-serif);color:var(--dsw-alias-label-primary,#e6ecff)}',
    '#' + DLS_ID + ' .dch-dls-head{display:flex;align-items:center;justify-content:space-between;margin-bottom:8px}',
    '#' + DLS_ID + ' .dch-dls-head h3{margin:0;font-size:13px;font-weight:600;letter-spacing:.2px}',
    '#' + DLS_ID + ' .dch-dls-head .dch-dls-count{font-size:11px;color:var(--dsw-alias-label-tertiary,#93a5d8);margin-left:6px}',
    '#' + DLS_ID + ' .dch-dls-close{width:24px;height:24px;border:none;border-radius:7px;background:transparent;color:var(--dsw-alias-label-secondary,#b8c5ea);cursor:pointer;font-size:12px;line-height:1}',
    '#' + DLS_ID + ' .dch-dls-close:hover{background:var(--dsw-alias-interactive-bg-hover,rgba(255,255,255,.1))}',
    '#' + DLS_ID + ' .dch-dls-list{max-height:420px;overflow:auto;display:flex;flex-direction:column;gap:7px;scrollbar-width:thin}',
    '#' + DLS_ID + ' .dch-dls-empty{font-size:12px;color:var(--dsw-alias-label-tertiary,#93a5d8);padding:16px 0;text-align:center}',
    '#' + DLS_ID + ' .dch-dls-item{border:1px solid var(--dsw-alias-border-l1,rgba(255,255,255,.09));border-radius:10px;padding:9px 11px;background:rgba(255,255,255,.02)}',
    '#' + DLS_ID + ' .dch-dls-row1{display:flex;align-items:center;justify-content:space-between;gap:10px;margin-bottom:6px}',
    '#' + DLS_ID + ' .dch-dls-name{font-size:12px;font-weight:600;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}',
    '#' + DLS_ID + ' .dch-dls-state{font-size:10.5px;flex:none;padding:1px 7px;border-radius:999px;border:1px solid var(--dsw-alias-border-l1,rgba(255,255,255,.1))}',
    '#' + DLS_ID + ' .dch-dls-state.ok{color:#22c55e;border-color:rgba(34,197,94,.4)}',
    '#' + DLS_ID + ' .dch-dls-state.err{color:#ff7a85;border-color:rgba(232,17,35,.45)}',
    '#' + DLS_ID + ' .dch-dls-track{height:4px;border-radius:99px;background:rgba(255,255,255,.08);overflow:hidden;margin-bottom:6px}',
    '#' + DLS_ID + ' .dch-dls-fill{height:100%;width:0%;border-radius:99px;background:#3b82f6;transition:width .25s}',
    '#' + DLS_ID + ' .dch-dls-meta{font-size:10.5px;color:var(--dsw-alias-label-tertiary,#93a5d8);font-family:var(--ds-font-family-code,Consolas,monospace);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}',
    '#' + DLS_ID + ' .dch-dls-actions{display:flex;gap:5px;justify-content:flex-end;margin-top:7px;flex-wrap:wrap}',
    '#' + DLS_ID + ' .dch-dls-actions button{min-width:56px;padding:3px 9px;border-radius:7px;border:1px solid var(--dsw-alias-border-l1,rgba(255,255,255,.12));background:transparent;color:var(--dsw-alias-label-primary,#dbe4f8);font:inherit;font-size:11px;cursor:pointer;transition:background .12s}',
    '#' + DLS_ID + ' .dch-dls-actions button:hover{background:var(--dsw-alias-interactive-bg-hover,rgba(255,255,255,.1))}',
    '#' + DLS_ID + ' .dch-dls-actions .dch-dls-cancel{color:#ff7a85;border-color:rgba(232,17,35,.4)}',
    '#' + DLS_ID + ' .dch-dls-actions .dch-dls-cancel:hover{background:rgba(232,17,35,.15)}',
    '#' + DLS_ID + ' .dch-dls-actions .dch-dls-del{color:#ff7a85;border-color:rgba(232,17,35,.4)}',
    '#' + DLS_ID + ' .dch-dls-actions .dch-dls-del:hover{background:rgba(232,17,35,.15)}'
  ].join('');
  var DLS_STATE_CN = { downloading: '下载中', paused: '已暂停', done: '完成', canceled: '已取消', error: '失败' };
  function fmtBytes(b) {
    if (b >= 1073741824) return (b / 1073741824).toFixed(2) + ' GB';
    if (b >= 1048576) return (b / 1048576).toFixed(1) + ' MB';
    if (b >= 1024) return (b / 1024).toFixed(1) + ' KB';
    return b + ' B';
  }
  function fmtPct(bytes, total) {
    if (!total || total <= 0) return '';
    return Math.min(100, Math.round(bytes / total * 100)) + '%';
  }
  function buildDlsPanel() {
    if (dlPanelBuilt) return;
    dlPanelBuilt = true;
    var style = document.createElement('style');
    style.textContent = DLS_CSS;
    document.head.appendChild(style);
    var box = document.createElement('div');
    box.id = DLS_ID;
    box.hidden = true;
    box.innerHTML =
      '<div class="dch-dls-box">' +
      '<div class="dch-dls-head"><h3>下载管理<span class="dch-dls-count"></span></h3><button class="dch-dls-close" title="关闭" aria-label="关闭">✕</button></div>' +
      '<div class="dch-dls-list"></div>' +
      '</div>';
    box.querySelector('.dch-dls-close').addEventListener('click', hideDlsPanel);
    document.body.appendChild(box);
    dlPanelEl = box;
    // 点击面板外部：有未完成任务（下载中/暂停）时不关闭（Edge 下载浮层逻辑），否则关闭
    document.addEventListener('click', function (e) {
      if (dlPanelEl && !dlPanelEl.hidden && !dlPanelEl.contains(e.target) && !dlActive) {
        hideDlsPanel();
      }
    });
  }
  function renderDls(list) {
    var listEl = dlPanelEl.querySelector('.dch-dls-list');
    var countEl = dlPanelEl.querySelector('.dch-dls-count');
    if (!list || !list.length) {
      listEl.innerHTML = '<div class="dch-dls-empty">暂无下载任务</div>';
      if (countEl) countEl.textContent = '';
      return;
    }
    if (countEl) countEl.textContent = '(' + list.length + ')';
    listEl.innerHTML = list.map(function (t) {
      var stateCls = t.state === 'done' ? ' ok' : (t.state === 'error' || t.state === 'canceled' ? ' err' : '');
      var pct = fmtPct(t.bytes, t.total);
      var metaText = t.state === 'done'
        ? fmtBytes(t.bytes) + ' · 已保存'
        : (pct ? pct + ' · ' + fmtBytes(t.bytes) + (t.total ? ' / ' + fmtBytes(t.total) : '') : fmtBytes(t.bytes));
      var btns = '';
      if (t.state === 'downloading') {
        btns += '<button data-dl="pause" data-id="' + t.id + '" data-on="1">暂停</button>';
        btns += '<button data-dl="browser" data-id="' + t.id + '">用浏览器下载</button>';
        btns += '<button data-dl="cancel" data-id="' + t.id + '" class="dch-dls-cancel">取消</button>';
      } else if (t.state === 'paused') {
        btns += '<button data-dl="pause" data-id="' + t.id + '" data-on="0">继续</button>';
        btns += '<button data-dl="cancel" data-id="' + t.id + '" class="dch-dls-cancel">取消</button>';
      } else if (t.state === 'done') {
        btns += '<button data-dl="open" data-id="' + t.id + '">打开文件夹</button>';
        btns += '<button data-dl="delete" data-id="' + t.id + '" class="dch-dls-del">删除</button>';
      } else {
        // canceled / error：可重试、转浏览器或删除
        btns += '<button data-dl="retry" data-id="' + t.id + '">重试</button>';
        btns += '<button data-dl="browser" data-id="' + t.id + '">用浏览器下载</button>';
        btns += '<button data-dl="delete" data-id="' + t.id + '" class="dch-dls-del">删除</button>';
      }
      return '<div class="dch-dls-item">' +
        '<div class="dch-dls-row1"><span class="dch-dls-name" title="' + t.file + '">' + t.name + '</span>' +
        '<span class="dch-dls-state' + stateCls + '">' + (DLS_STATE_CN[t.state] || t.state) + '</span></div>' +
        '<div class="dch-dls-track"><div class="dch-dls-fill" style="width:' + (pct || (t.state === 'done' ? '100' : '0')) + '"></div></div>' +
        '<div class="dch-dls-meta">' + metaText + (t.error ? ' · ' + t.error : '') + '</div>' +
        '<div class="dch-dls-actions">' + btns + '</div>' +
        '</div>';
    }).join('');
    listEl.querySelectorAll('[data-dl]').forEach(function (b) {
      b.addEventListener('click', function () {
        var op = b.getAttribute('data-dl');
        var id = b.getAttribute('data-id');
        if (op === 'pause') actQ('download-pause', 'id=' + id + '&on=' + b.getAttribute('data-on'));
        else if (op === 'cancel') actQ('download-cancel', 'id=' + id);
        else if (op === 'open') actQ('download-open', 'id=' + id);
        else if (op === 'delete') actQ('download-delete', 'id=' + id);
        else if (op === 'retry') actQ('download-retry', 'id=' + id);
        else if (op === 'browser') actQ('download-browser', 'id=' + id);
      });
    });
  }
  // ── 下载轮询：常驻轻量轮询（1s）──
  // 1) 有进行中/暂停任务且面板未开 → 自动弹出（Edge 下载逻辑：下载开始即显示浮层）
  // 2) 面板开着 → 渲染最新列表
  // 3) 维护 dlActive（供“未完成任务时点击外部不关闭”）
  var dlActive = false;
  var dlDismissed = false, lastCount = 0;
  function pollDls() {
    fetch(HTTP + '/downloads').then(function (r) { return r.json(); })
      .then(function (list) {
        var active = list.some(function (t) { return t.state === 'downloading' || t.state === 'paused'; });
        dlActive = active;
        var count = list.length;
        if (active && !dlPanelEl) buildDlsPanel();
        // 自动弹出：未手动收起；或手动收起后出现了新任务（任务数增加）
        if (active && dlPanelEl && dlPanelEl.hidden && (!dlDismissed || count > lastCount)) {
          dlPanelEl.hidden = false;
          renderDls(list);
        } else if (dlPanelEl && !dlPanelEl.hidden) {
          renderDls(list);
        }
        lastCount = count;
      })
      .catch(function () {});
  }
  setInterval(pollDls, 1000);
  function showDlsPanel() {
    buildDlsPanel();
    dlPanelEl.hidden = false;
    dlDismissed = false;
    pollDls();
  }
  function hideDlsPanel() {
    if (dlPanelEl) dlPanelEl.hidden = true;
    dlDismissed = true;
  }
  window.__dshdShowDownloads = showDlsPanel;

  var HTTP = 'http://127.0.0.1:19431';
  // 外部页面按钮：标准 HTTP img 请求到本地控制服务（WebView2 必达 Rust）
  function act(a) { new Image().src = HTTP + '/action?name=' + encodeURIComponent(a); }
  // 带参数的 action（下载管理用）：actQ('download-cancel', 'id=3')
  function actQ(a, q) { new Image().src = HTTP + '/action?name=' + encodeURIComponent(a) + '&' + q; }
  function logWeb(m) { try { new Image().src = HTTP + '/log?msg=' + encodeURIComponent(m); } catch (e) {} }
  function setMaximized(isMax) {
    if (!maxBtn) return;
    maxBtn.innerHTML = isMax ? GLYPHS.restore : GLYPHS.max;
    maxBtn.title = isMax ? '还原' : '最大化';
  }
  function closeMenu() {
    menuOpen = false;
    if (menuEl) menuEl.hidden = true;
    // 菜单关闭：若在 DeepSeek 内容页，恢复显示（菜单打开期间被临时隐藏以免被盖住）
    act('ds-menu-close');
  }
  function renderMenu() {
    if (!menuEl) return;
    var sw = document.querySelector('[data-act="toggle-deepseek"]');
    var swOn = sw ? sw.classList.contains('on') : false;
    menuEl.innerHTML = [
      '<div class="dch-mh"><div class="dch-mh-title">' + APP_TITLE + (isDeepseek ? '' : ' <span style="font-weight:400;color:var(--dsw-alias-label-tertiary)">封装 v1.9.0</span>') + '</div>',
      (isDeepseek ? '</div>' : '<div class="dch-mh-sub"><span>后端状态：<span id="dshd-menu-status">连接中…</span></span></div></div>'),
      '<button class="dch-item" data-act="restart"><span>重启 Web 服务</span><span class="dch-kbd">重启后端</span></button>',
      '<button class="dch-item" data-act="reload"><span>重新加载</span><span class="dch-kbd">刷新页面</span></button>',
      '<button class="dch-item" data-act="toggle-deepseek"><span>' + (swOn ? '切回 DeepSeek Harness' : '切换 DeepSeek 聊天页') + '</span><span class="dch-kbd">DeepSeek</span></button>',
      '<button class="dch-item" data-act="downloads"><span>下载管理</span><span class="dch-kbd">导出文件</span></button>',
      '<div class="dch-sep"></div>',
      '<button class="dch-item" data-act="browser">在浏览器中打开</button>',
      '<div class="dch-sep"></div>',
      '<button class="dch-item" data-act="about">关于 ' + APP_TITLE + '</button>',
      '<button class="dch-item" data-danger="1" data-act="quit">退出</button>'
    ].join('');
    menuEl.querySelectorAll('.dch-item').forEach(function (item) {
      item.addEventListener('click', function () {
        var actName = item.getAttribute('data-act');
        closeMenu();
        // 下载管理 = 直接开面板（不经过 HTTP action）；退出 = 直接退出（不弹窗）
        if (actName === 'downloads') { window.__dshdShowDownloads(); return; }
        act(actName);
      });
    });
  }
  function openMenu() {
    if (!menuEl) return;
    // 菜单打开：若在 DeepSeek 内容页，先让 Rust 临时隐藏子 WebView，否则下拉菜单被盖住
    act('ds-menu-open');
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
      '<span class="dch-title">' + APP_TITLE + '</span>' +
      (isDeepseek ? '<span class="dch-badge">DeepSeek</span>' : '<span class="dch-badge">v1.9.0</span>' + '<span class="dch-status"><span class="dshd-dot warn" id="dshd-dot"></span><span id="dshd-status-label">连接中…</span></span>') +
      '</div>' +
      '<div class="dch-right">' +
      '<button class="dch-switch" data-act="toggle-deepseek" title="切换到 DeepSeek 聊天页" aria-label="切换页面"></button>' +
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
    bar.querySelector('[data-act="toggle-deepseek"]').addEventListener('click', function () {
      // 拨片：点击 → 通知 Rust 切换内容页 + 本地翻转开关态/提示文案
      var nextOn = !this.classList.contains('on');
      act('toggle-deepseek');
      this.classList.toggle('on', nextOn);
      this.title = nextOn ? '切回 DeepSeek Harness' : '切换到 DeepSeek 聊天页';
    });
    bar.querySelector('.dch-close').addEventListener('click', function () {
      // ✕ 直接弹「退出 / 缩小到托盘」选择框；
      // 不能走 act('close')→win.close()：程序化 close 不触发 CloseRequested，
      // Rust 的拦截弹窗会被绕过（窗口直接关闭、应用退出、后端成孤儿）。
      window.__dshdShowCloseDialog();
    });
    bar.querySelector('[data-act="menu"]').addEventListener('click', function (e) { e.stopPropagation(); if (menuOpen) closeMenu(); else openMenu(); });
    // Tauri 不支持 -webkit-app-region：整栏拖动走 dshd://drag（Rust start_dragging）
    bar.addEventListener('mousedown', function (e) {
      if (e.target.closest('button') || e.target.closest('.dch-menu')) return;
      if (e.button !== 0) return;
      act('drag');
    });
    document.addEventListener('click', function (e) { if (menuOpen && !bar.contains(e.target)) closeMenu(); });
    document.addEventListener('keydown', function (e) {
      if (e.key === 'Escape') { hideCloseDlg(); hideDlsPanel(); closeMenu(); }
    });
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
            let state = match app.try_state::<Arc<AppState>>() {
                Some(s) => s,
                None => return json("{}".to_string()),
            };
            let st = state.backend.current_status();
            match serde_json::to_string(&st) {
                Ok(s) => json(s),
                Err(_) => json("{}".to_string()),
            }
        }
        "/restart" => {
            if let Some(state) = app.try_state::<Arc<AppState>>() {
                state.backend.restart("menu");
            }
            json(r#"{"ok":true}"#.to_string())
        }
        "/reload" => {
            if let Some(w) = app.get_webview("dsh") {
                let _ = w.eval("location.reload()");
            }
            json(r#"{"ok":true}"#.to_string())
        }
        "/browser" => {
            let u = app
                .try_state::<Arc<AppState>>()
                .map(|s| s.backend.current_status().url)
                .flatten();
            if let Some(u) = u {
                let _ = tauri_plugin_opener::OpenerExt::opener(app).open_url(u, None::<&str>);
            }
            json(r#"{"ok":true}"#.to_string())
        }
        // ???? ???????????????Tauri ??? API ????????????????
        "/min" => {
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(win) = app2.get_window("main") {
                    let r = win.minimize();
                    let _ = r;
                }
            });
            json(r#"{"ok":true}"#.to_string())
        }
        "/max" => {
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(win) = app2.get_window("main") {
                    if win.is_maximized().unwrap_or(false) {
                        let _ = win.unmaximize();
                    } else {
                        let _ = win.maximize();
                    }
                }
            });
            json(r#"{"ok":true}"#.to_string())
        }
        // 程序化关闭一律转为弹出「退出 / 缩小到托盘」选择框（与系统关闭一致）
        "/close" => {
            show_close_dialog(&app);
            json(r#"{"ok":true}"#.to_string())
        }
        "/drag" => {
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(win) = app2.get_window("main") {
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

/// 弹出「退出 / 缩小到托盘」选择框（外壳层内的 close 弹层）。
pub fn show_close_dialog(app: &AppHandle) {
    let Some(state) = app.try_state::<Arc<AppState>>() else {
        return;
    };
    state.log.info("[close] 弹出退出选择弹层");
    overlay_set(app, state.inner(), "popup-close", true);
}

/// 取 DeepSeek 内容子 WebView（启动时已创建并隐藏，见 main.rs；这里只负责首次导航）。
fn ensure_deepseek_webview(app: &AppHandle) -> Option<tauri::Webview> {
    app.get_webview("deepseek")
}

/// 下载拦截（主窗 DSH 与 DeepSeek 内容 WebView 共用）：走自管下载器，避免原生下载链崩溃。
pub fn intercept_download(webview: tauri::Webview, event: tauri::webview::DownloadEvent) -> bool {
    let app = webview.app_handle().clone();
    let Some(dl) = app.try_state::<Downloads>() else {
        return false; // 下载器未就绪：拦截下载（宁可放弃也不崩进程）
    };
    if let tauri::webview::DownloadEvent::Requested { url, destination } = event {
        // 保存目录：设置优先，否则系统下载目录
        let dir = app
            .try_state::<Arc<AppState>>()
            .map(|st| {
                st.settings
                    .lock()
                    .unwrap()
                    .download_dir
                    .clone()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| dirs::download_dir().unwrap_or_else(std::env::temp_dir))
            })
            .unwrap_or_else(|| dirs::download_dir().unwrap_or_else(std::env::temp_dir));
        let ask = app
            .try_state::<Arc<AppState>>()
            .map(|st| st.settings.lock().unwrap().ask_download_location)
            .unwrap_or(false);
        let name = destination
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                url.path_segments()
                    .and_then(|mut it| it.next_back())
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty() && s != "session.export")
                    .unwrap_or_else(|| "dsh-export.zip".to_string())
            });
        dl.offer(url.as_str(), &name, dir, ask);
        false // 阻止 WebView2 下载，由自管下载器接管
    } else {
        true
    }
}

/// 当前毫秒时间戳（弹层守卫用）。
fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 按弹层名取可见标志位。
fn overlay_flag<'a>(state: &'a AppState, name: &str) -> &'a AtomicBool {
    match name {
        "popup-close" => &state.popup_close_visible,
        "popup-downloads" => &state.popup_downloads_visible,
        "popup-settings" => &state.popup_settings_visible,
        _ => &state.popup_menu_visible,
    }
}

/// 弹层展开后外壳层需覆盖的高度（逻辑 px，含阴影余量）。退出弹层居中 → 全窗。
fn overlay_span(name: &str) -> f64 {
    match name {
        "popup-menu" => 36.0 + 372.0,
        "popup-downloads" => 36.0 + 546.0,
        "popup-settings" => 36.0 + 420.0,
        _ => f64::INFINITY,
    }
}

/// 根据各弹层开关调整外壳层 WebView 覆盖范围：
/// 空闲 = 仅标题栏条（内容层完整可点）；有弹层 = 扩展覆盖弹层区域（透明，内容透过可见）；退出弹层 = 全窗。
fn set_shell_extent(app: &AppHandle) {
    let Some(state) = app.try_state::<Arc<AppState>>() else {
        return;
    };
    let Some(win) = app.get_window("main") else {
        return;
    };
    let scale = win.scale_factor().unwrap_or(1.0);
    let size = win.inner_size().unwrap_or_default();
    let w = size.width;
    let h = size.height;
    let base = (36.0 * scale).round() as u32;
    let mut need = base;
    for name in ["popup-menu", "popup-downloads", "popup-settings", "popup-close"] {
        if overlay_flag(state.inner(), name).load(Ordering::SeqCst) {
            let span = overlay_span(name);
            if span.is_infinite() {
                need = h;
                break;
            }
            need = need.max(base + (span * scale).round() as u32);
        }
    }
    let want_h = need.min(h).max(base);
    if let Some(c) = app.get_webview("chrome") {
        let _ = c.set_position(tauri::PhysicalPosition::new(0, 0));
        let _ = c.set_size(tauri::PhysicalSize::new(w, want_h));
    }
}

/// 弹层 JS 名（chrome.html 的 POPUP 键）。
fn popup_js_key(name: &str) -> &'static str {
    match name {
        "popup-close" => "close",
        "popup-downloads" => "dl",
        "popup-settings" => "settings",
        _ => "menu",
    }
}

/// 设置单个弹层开/关（主线程执行），并重算外壳层覆盖范围。
fn overlay_set(app: &AppHandle, state: &Arc<AppState>, name: &str, on: bool) {
    let app2 = app.clone();
    let state2 = state.clone();
    let name2 = name.to_string();
    let _ = app.run_on_main_thread(move || {
        let flag = overlay_flag(&state2, &name2);
        let current = flag.load(Ordering::SeqCst);
        if on && !current {
            flag.store(true, Ordering::SeqCst);
            state2
                .last_popup_shown_ms
                .store(now_millis(), Ordering::SeqCst);
            state2.log.info(&format!("[overlay] {} 打开", name2));
        } else if !on && current {
            flag.store(false, Ordering::SeqCst);
            state2.log.info(&format!("[overlay] {} 关闭", name2));
        }
        if let Some(c) = app2.get_webview("chrome") {
            let js = format!(
                "window.__dshdShowPopup && window.__dshdShowPopup('{}', {});",
                popup_js_key(&name2),
                if flag.load(Ordering::SeqCst) { "true" } else { "false" }
            );
            let _ = c.eval(&js);
        }
        set_shell_extent(&app2);
    });
}

/// 切换弹层开关（菜单按钮/退出按钮等）。
fn overlay_toggle(app: &AppHandle, state: &Arc<AppState>, name: &str) {
    let on = overlay_flag(state, name).load(Ordering::SeqCst);
    overlay_set(app, state, name, !on);
}

/// 关闭所有弹层（“点外部”）。
fn close_all_overlays(app: &AppHandle, state: &Arc<AppState>) {
    overlay_set(app, state, "popup-menu", false);
    overlay_set(app, state, "popup-close", false);
    overlay_set(app, state, "popup-downloads", false);
    overlay_set(app, state, "popup-settings", false);
}

/// 注入内容页的主题桥（事件驱动，替代 Rust 轮询）：
/// 页面内 MutationObserver 监听主题相关属性变化 + 1s 本地 diff（值变了才上报）→ img /set-theme →
/// Rust 按 src 存入对应主题槽；仅当前显示页的上报会立即推给外壳层（标题栏与弹层共用）。
/// 注：`--dsw-alias-*` 只是启动页别名，主界面根部读不到；改采样 documentElement/body 真实渲染色。
/// src = "dsh" | "deepseek"（DeepSeek 页无 data-ds-dark-theme，暗色兜底用 prefers-color-scheme）。
pub fn theme_bridge_js(src: &str) -> String {
    r##"(function () {
  if (window.__dshdThemeBridge) return;
  window.__dshdThemeBridge = true;
  var last = '';
  function read() {
    try {
      var el = null, i;
      var cand = [document.documentElement, document.body];
      for (i = 0; i < cand.length; i++) {
        if (!cand[i]) continue;
        var cs = getComputedStyle(cand[i]);
        var b = cs.backgroundColor;
        if (b && b !== 'rgba(0, 0, 0, 0)' && b !== 'transparent') { el = cand[i]; break; }
      }
      // 官方默认皮肤下 html/body 背景透明（底色由 #root / AppFrame 等面板绘制），
      // 继续向下采样真实渲染色，避免落到硬编码回退值把标题栏带黑。
      if (!el) {
        var deep = [document.querySelector('[data-dsh-frame]'), document.querySelector('#root'), document.querySelector('[data-dsh-app]')];
        for (i = 0; i < deep.length; i++) {
          if (!deep[i]) continue;
          var dcs = getComputedStyle(deep[i]);
          var db = dcs.backgroundColor;
          if (db && db !== 'rgba(0, 0, 0, 0)' && db !== 'transparent') { el = deep[i]; break; }
        }
      }
      var bg = el ? getComputedStyle(el).backgroundColor : '';
      // 暗色判定：dsh 页有 data-ds-dark-theme（无属性 = 亮色）；DeepSeek 页兜底跟系统主题
      var dark = false;
      if (document.body && document.body.hasAttribute('data-ds-dark-theme')) dark = true;
      else if ('__SRC__' !== 'dsh') {
        dark = !!(window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches);
      }
      if (!bg) bg = dark ? '#0b1220' : '#ffffff';
      // 前景色按背景亮度推导，不取页面 body 的 color：外部页（如 chat.deepseek.com）
      // 的 body color 可能是链接紫等与标题栏无关的值，直接套用会让标题栏文字失调。
      var m = bg.match(/(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/);
      var lum = m ? (0.299 * +m[1] + 0.587 * +m[2] + 0.114 * +m[3]) : (dark ? 0 : 255);
      var fg = lum < 140 ? 'rgb(230, 236, 255)' : 'rgb(20, 28, 48)';
      return JSON.stringify({ bg: bg, fg: fg });
    } catch (e) { return ''; }
  }
  function send() {
    var v = read();
    if (!v || v === last) return; // 值没变不重复上报
    last = v;
    new Image().src = 'http://127.0.0.1:19431/set-theme?src=__SRC__&t=' + encodeURIComponent(v);
  }
  send();
  window.addEventListener('load', send);
  function startObs() {
    var mo = new MutationObserver(send);
    mo.observe(document.documentElement, { attributes: true, attributeFilter: ['class', 'style', 'data-theme'] });
    try {
      if (document.body) mo.observe(document.body, { attributes: true, attributeFilter: ['class', 'style', 'data-theme', 'data-ds-dark-theme'] });
    } catch (e) {}
  }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', startObs);
  else startObs();
  setInterval(send, 1000);
})();"##
    .replace("__SRC__", src)
}

/// 解析 CSS 颜色字符串（#rrggbb / #rgb / rgb(r,g,b)）为 (r,g,b)。
fn parse_color(s: &str) -> Option<(u8, u8, u8)> {
    let t = s.trim();
    if let Some(hex) = t.strip_prefix('#') {
        let h = if hex.len() == 3 {
            let b = hex.as_bytes();
            format!(
                "{0}{0}{1}{1}{2}{2}",
                b[0] as char, b[1] as char, b[2] as char
            )
        } else {
            hex.to_string()
        };
        if h.len() == 6 {
            let r = u8::from_str_radix(&h[0..2], 16).ok()?;
            let g = u8::from_str_radix(&h[2..4], 16).ok()?;
            let b = u8::from_str_radix(&h[4..6], 16).ok()?;
            return Some((r, g, b));
        }
        return None;
    }
    if let Some(inner) = t.strip_prefix("rgb(").and_then(|x| x.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
        if parts.len() >= 3 {
            let r = parts[0].parse::<u8>().ok()?;
            let g = parts[1].parse::<u8>().ok()?;
            let b = parts[2].parse::<u8>().ok()?;
            return Some((r, g, b));
        }
    }
    None
}

/// 按设置的窗口材质 + 主题亮度应用窗口效果。
/// acrylic = 亚克力（实时模糊下层窗口，真"透"）；mica = 云母（仅采样壁纸做底纹，不透窗口）；
/// none / 材质不可用（Win10 等）→ 纯色底，避免透明窗口露底。
pub fn apply_window_effect(app: &AppHandle, state: &AppState, bg: &str) {
    let Some(win) = app.get_window("main") else {
        return;
    };
    let rgb = parse_color(bg);
    let dark = rgb
        .map(|(r, g, b)| 0.299 * (r as f64) + 0.587 * (g as f64) + 0.114 * (b as f64) < 140.0)
        .unwrap_or(true);
    let material = state.settings.lock().unwrap().window_material.to_lowercase();
    let effect = match material.as_str() {
        "none" => None,
        "mica" => Some(if dark { Effect::MicaDark } else { Effect::MicaLight }),
        _ => Some(Effect::Acrylic),
    };
    let applied = match effect {
        Some(e) => win
            .set_effects(EffectsBuilder::new().effect(e).build())
            .is_ok(),
        None => {
            // 必须传 None 才会清除材质；传空的 EffectsBuilder 是空效果列表，DWM 材质会原样保留
            let _ = win.set_effects(None::<tauri::utils::config::WindowEffectsConfig>);
            false
        }
    };
    if !applied {
        if let Some((r, g, b)) = rgb {
            let _ = win.set_background_color(Some(tauri::window::Color(r, g, b, 255)));
        }
    }
}

/// 把主题色立即下发给外壳层（标题栏 + 弹层都在同一 WebView），并刷新窗口材质。
fn push_theme(app: &AppHandle, state: &AppState, bg: &str, fg: &str) {
    if let Some(chrome) = app.get_webview("chrome") {
        let esc = |x: &str| x.replace('\\', "\\\\").replace('"', "\\\"");
        let json = format!(r#"{{"bg":"{}","fg":"{}"}}"#, esc(bg), esc(fg));
        let _ = chrome.eval(&format!("window.__dshdTheme && window.__dshdTheme({});", json));
    }
    apply_window_effect(app, state, bg);
}

/// 注入 DSH 内容页的点击转发：用户在 DSH 页任意处按下鼠标左键 → 上报 main-click（视为“点外部”关弹层）。
/// 纯坐标转发，无任何视觉/交互副作用；img 请求与主题桥一致，不受 CSP 影响。
pub fn click_forwarder_js() -> String {
    r##"(function () {
  if (window.__dshdClickProxy) return;
  window.__dshdClickProxy = true;
  document.addEventListener('mousedown', function (e) {
    if (e.button !== 0) return;
    try { new Image().src = 'http://127.0.0.1:19431/action?name=main-click'; } catch (err) {}
  }, true);
})();"##
    .to_string()
}

/// 壳窗口移动/缩放：重排内容层子 WebView + 重算外壳层覆盖范围。
pub fn on_shell_resize(app: &AppHandle) {
    relayout_children(app);
    set_shell_extent(app);
    // 缩放/移动后强制外壳页重排标题栏（清掉陈旧裁切）
    if let Some(c) = app.get_webview("chrome") {
        let _ = c.eval("window.__dshdOnResize && window.__dshdOnResize();");
    }
    // 兜底校准：拖拽缩放事件序列与最终尺寸存在竞态，150ms 后按最终尺寸再校一遍
    let app2 = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(150));
        let app3 = app2.clone();
        let _ = app2.run_on_main_thread(move || {
            relayout_children(&app3);
            set_shell_extent(&app3);
            if let Some(c) = app3.get_webview("chrome") {
                let _ = c.eval("window.__dshdOnResize && window.__dshdOnResize();");
            }
        });
    });
}

/// 重排 dsh/deepseek 内容层子 WebView（标题栏 36px 之下）。外壳层范围由 set_shell_extent 单独管理。
fn relayout_children(app: &AppHandle) {
    let Some(win) = app.get_window("main") else {
        return;
    };
    let scale = win.scale_factor().unwrap_or(1.0);
    let size = win.inner_size().unwrap_or_default();
    let top = (36.0 * scale).round() as i32;
    let w = size.width;
    let content_h = size.height.saturating_sub(top as u32);
    if let Some(d) = app.get_webview("dsh") {
        let _ = d.set_position(tauri::PhysicalPosition::new(0, top));
        let _ = d.set_size(tauri::PhysicalSize::new(w, content_h));
    }
    if let Some(ds) = app.get_webview("deepseek") {
        let _ = ds.set_position(tauri::PhysicalPosition::new(0, top));
        let _ = ds.set_size(tauri::PhysicalSize::new(w, content_h));
    }
}

pub fn run_action(app: &tauri::AppHandle, state: &Arc<AppState>, action: &str) -> bool {
    match action {
        "min" => {
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(w) = app2.get_window("main") {
                    let _ = w.minimize();
                }
            });
            true
        }
        "max" => {
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(w) = app2.get_window("main") {
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
            // 程序化关闭一律弹「退出 / 缩小到托盘」选择框（win.close() 会绕过 CloseRequested 拦截）
            show_close_dialog(app);
            true
        }
        "drag" => {
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(w) = app2.get_window("main") {
                    let _ = w.start_dragging();
                }
            });
            true
        }
        "restart" => {
            // 重启与“就绪后自动刷新”已统一在 Backend::restart 内完成
            state.backend.restart("titlebar");
            true
        }
        "reload" => {
            if let Some(w) = app.get_webview("dsh") {
                let _ = w.eval("location.reload()");
            }
            true
        }
        // 关闭弹窗 → 缩小到托盘
        "min-tray" => {
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(w) = app2.get_window("main") {
                    let _ = w.hide();
                }
            });
            true
        }
        // 关闭弹窗 / 菜单 / 托盘 → 真正退出。
        // 注意：不能只依赖 RunEvent::Exit 回调做清理——AppHandle::exit 的 request_exit
        // 在部分线程/时机下会失败并直接 std::process::exit，Exit 事件可能根本不触发。
        // 所以这里先同步 kill_owned 杀自有后端进程树，再请求退出（Exit 回调仅作兜底）。
        "quit" => {
            state.log.info("[titlebar] 用户选择退出");
            state.force_exit.store(true, Ordering::SeqCst);
            state.backend.kill_owned();
            app.exit(0);
            true
        }
        "browser" => {
            if let Some(u) = state.backend.current_status().url {
                let _ = tauri_plugin_opener::OpenerExt::opener(app).open_url(u, None::<&str>);
            }
            true
        }
        // 标题栏「切换」拨片：DSH ⇄ DeepSeek 内容页（同层并存，显示/隐藏，不导航、不刷新）
        "toggle-deepseek" => {
            state.log.info("[titlebar] 切换 DeepSeek / DSH 内容页（同层并存）");
            let app2 = app.clone();
            let state2 = state.clone();
            let _ = app.run_on_main_thread(move || {
                if state2.deepseek_shown.load(Ordering::SeqCst) {
                    // 切回 DSH：隐藏 DeepSeek 内容页（外壳层不动）
                    state2.log.info("[titlebar] 隐藏 DeepSeek 内容页");
                    if let Some(ds) = app2.get_webview("deepseek") {
                        let _ = ds.hide();
                    }
                    state2.deepseek_shown.store(false, Ordering::SeqCst);
                } else {
                    // 切到 DeepSeek：显示启动时已建好的内容页；首次显示时导航到 chat.deepseek.com
                    state2.log.info("[titlebar] 显示 DeepSeek 内容页");
                    if let Some(ds) = ensure_deepseek_webview(&app2) {
                        if !state2.deepseek_loaded.load(Ordering::SeqCst) {
                            if let Ok(u) = "https://chat.deepseek.com/".parse::<tauri::Url>() {
                                let _ = ds.navigate(u);
                            }
                            state2.deepseek_loaded.store(true, Ordering::SeqCst);
                        }
                        let _ = ds.show();
                        state2.deepseek_shown.store(true, Ordering::SeqCst);
                    }
                }
                if let Some(main) = app2.get_window("main") {
                    let _ = main.set_focus();
                }
                // 切换后立即按目标页主题缓存刷新标题栏/窗口材质（无缓存则等桥上报）
                if let Some((bg, fg)) = state2.current_theme() {
                    push_theme(&app2, &state2, &bg, &fg);
                }
                // 回推开关状态：菜单项/托盘等入口切换时，标题栏滑柄与菜单文案同步
                if let Some(c) = app2.get_webview("chrome") {
                    let on = state2.deepseek_shown.load(Ordering::SeqCst);
                    let _ = c.eval(&format!(
                        "window.__dshdSwitchState && window.__dshdSwitchState({});",
                        on
                    ));
                }
            });
            true
        }
        // ── 外壳层弹层：菜单 / 退出选择 / 下载管理（DOM 弹层，非独立窗口）──
        "popup-menu" => {
            overlay_toggle(app, state, "popup-menu");
            true
        }
        "popup-menu-hide" => {
            overlay_set(app, state, "popup-menu", false);
            true
        }
        "popup-downloads" | "popup-dl" => {
            overlay_toggle(app, state, "popup-downloads");
            true
        }
        "popup-downloads-hide" => {
            overlay_set(app, state, "popup-downloads", false);
            true
        }
        "popup-close" => {
            overlay_toggle(app, state, "popup-close");
            true
        }
        "popup-close-hide" => {
            overlay_set(app, state, "popup-close", false);
            true
        }
        "popup-dl-settings" => {
            overlay_toggle(app, state, "popup-settings");
            true
        }
        // 下载“询问保存位置”确认/取消由 /action 分发到 handle_download_action（带 dir 参数）
        // 用户点击主窗内容/标题栏非菜单按钮处（“点外部”）→ 关闭弹层。
        // Edge 行为：下载面板在有进行中任务时不因点外部关闭（仅有徽标/显式关闭）。
        "main-click" => {
            let app2 = app.clone();
            let state2 = state.clone();
            let _ = app.run_on_main_thread(move || {
                if now_millis().saturating_sub(state2.last_popup_shown_ms.load(Ordering::SeqCst)) < 400 {
                    return; // 刚打开弹层（点击衔接期）：不关
                }
                // 菜单/设置/退出弹层：一律关闭
                overlay_set(&app2, &state2, "popup-menu", false);
                overlay_set(&app2, &state2, "popup-close", false);
                overlay_set(&app2, &state2, "popup-settings", false);
                // 下载面板：有进行中任务时保留（Edge 逻辑）
                let keep_dl = state2.popup_downloads_visible.load(Ordering::SeqCst)
                    && app2
                        .try_state::<Arc<Downloads>>()
                        .map(|dl| dl.has_active())
                        .unwrap_or(false);
                if !keep_dl {
                    overlay_set(&app2, &state2, "popup-downloads", false);
                }
            });
            true
        }
        "ping" => {
            state.log.info("[http] ping received");
            true
        }
        _ => false,
    }
}

/// 下载管理 action：download-cancel / download-pause(id,&on) / download-open。
fn handle_download_action(
    app: &tauri::AppHandle,
    state: &Arc<AppState>,
    name: &str,
    q: &std::collections::HashMap<String, String>,
) -> bool {
    let Some(dl) = app.try_state::<Downloads>() else {
        return false;
    };
    let id: u64 = q.get("id").and_then(|v| v.parse().ok()).unwrap_or(0);
    match name {
        "download-cancel" => dl.cancel(id),
        "download-pause" => {
            let on = q.get("on").map(|v| v == "1").unwrap_or(true);
            dl.pause(id, on)
        }
        "download-open" => {
            let target = dl
                .list()
                .into_iter()
                .find(|t| t.id == id)
                .map(|t| t.file.clone());
            match target {
                Some(path) if std::path::Path::new(&path).exists() => {
                    // 资源管理器打开并选中文件
                    let _ = Command::new("explorer.exe")
                        .args(["/select,", &path])
                        .creation_flags(0x08000000)
                        .spawn();
                    state.log.info(&format!("[download] open folder: {}", path));
                    true
                }
                _ => false,
            }
        }
        // 打开文件（默认程序）
        "download-open-file" => {
            match dl.file_of(id).filter(|p| std::path::Path::new(p).exists()) {
                Some(path) => {
                    state.log.info(&format!("[download] open file: {}", path));
                    let _ = tauri_plugin_opener::OpenerExt::opener(app)
                        .open_path(path, None::<&str>);
                    true
                }
                None => false,
            }
        }
        // 复制下载链接到剪贴板
        "download-copy-url" => match dl.url_of(id) {
            Some(url) => {
                let ok = crate::util::copy_to_clipboard(&url);
                state.log.info(&format!("[download] copy url (id={}) ok={}", id, ok));
                ok
            }
            None => false,
        },
        // 仅从列表移除（保留文件）
        "download-remove" => dl.remove(id),
        "download-delete" => dl.delete(id),
        // 下载“询问保存位置”确认：dir 为选定的保存目录
        "dl-ask-confirm" => {
            let Some((url, name)) = dl.take_pending() else {
                return false;
            };
            let dir = url_decode(&q.get("dir").cloned().unwrap_or_default());
            let dir_path = if dir.trim().is_empty() {
                dirs::download_dir().unwrap_or_else(std::env::temp_dir)
            } else {
                PathBuf::from(dir.trim())
            };
            state.log.info(&format!("[download] 询问确认：保存到 {} （{}）", dir_path.display(), name));
            dl.start(&url, &name, dir_path).is_some()
        }
        "dl-ask-cancel" => {
            dl.clear_pending();
            state.log.info("[download] 询问被取消");
            true
        }
        "download-retry" => dl.retry(id).is_some(),
        "download-browser" => {
            // 转交系统浏览器下载（浏览器自带下载管理；127.0.0.1 URL 用默认浏览器打开）
            match dl.url_of(id) {
                Some(url) => {
                    dl.cancel(id); // 转交后停止自管下载，避免重复下载
                    state.log.info(&format!("[download] handover to browser: {}", url));
                    tauri_plugin_opener::OpenerExt::opener(app).open_url(url, None::<&str>).is_ok()
                }
                None => false,
            }
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
                state.log.error(&format!("[http] 无法绑定 19431 端口: {}", e));
                return;
            }
        };
        state.log.info("[http] 本地控制服务已启动 http://127.0.0.1:19431");
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
                        let ok = if name.starts_with("download-") || name == "dl-ask-confirm" || name == "dl-ask-cancel" {
                            handle_download_action(&app, &state, &name, &q)
                        } else {
                            run_action(&app, &state, &name)
                        };
                        format!(
                            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n{}\r\n{{\"ok\":{}}}",
                            cors, ok
                        )
                    }
                    ("GET", "/downloads") => {
                        let list = match app.try_state::<Downloads>() {
                            Some(dl) => serde_json::to_string(&dl.list())
                                .unwrap_or_else(|_| "[]".to_string()),
                            None => "[]".to_string(),
                        };
                        format!(
                            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n{}\r\n{}",
                            cors, list
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
                    // 内容页主题桥上报（src=dsh|deepseek，t=JSON）→ 存对应主题槽；
                    // 仅当前显示页的上报立即推给标题栏与窗口材质（事件驱动）
                    ("GET", "/set-theme") => {
                        let raw = url_decode(&q.get("t").cloned().unwrap_or_default());
                        let src = q.get("src").map(|s| s.as_str()).unwrap_or("dsh").to_string();
                        let mut bg = String::new();
                        let mut fg = String::new();
                        if raw.starts_with('{') {
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                                bg = v.get("bg").and_then(|x| x.as_str()).unwrap_or("").to_string();
                                fg = v.get("fg").and_then(|x| x.as_str()).unwrap_or("").to_string();
                            }
                        }
                        if !bg.is_empty() {
                            let from_deepseek = src == "deepseek";
                            {
                                let slot = if from_deepseek { &state.theme_deepseek } else { &state.theme_dsh };
                                *slot.lock().unwrap() = Some((bg.clone(), fg.clone()));
                            }
                            // 隐藏页的上报只入缓存，不打扰当前显示页的标题栏
                            if from_deepseek == state.deepseek_shown.load(Ordering::SeqCst) {
                                let app2 = app.clone();
                                let state2 = state.clone();
                                let _ = app.run_on_main_thread(move || {
                                    push_theme(&app2, &state2, &bg, &fg);
                                });
                            }
                        }
                        format!("HTTP/1.1 200 OK\r\n{}\r\n{{}}", cors)
                    }
                    // 弹窗打开时自取当前主题（当前显示页的主题槽）
                    ("GET", "/theme") => {
                        let t = state.current_theme();
                        let payload = match t {
                            Some((bg, fg)) => format!(
                                r#"{{"bg":"{}","fg":"{}"}}"#,
                                bg.replace('\\', "\\\\").replace('"', "\\\""),
                                fg.replace('\\', "\\\\").replace('"', "\\\"")
                            ),
                            None => "{}".to_string(),
                        };
                        format!(
                            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n{}\r\n{}",
                            cors, payload
                        )
                    }
                    // 下载“询问保存位置”状态读取
                    ("GET", "/dl-ask") => {
                        let pending = app
                            .try_state::<Downloads>()
                            .and_then(|dl| dl.pending_name())
                            .unwrap_or_default();
                        let dir = state
                            .settings
                            .lock()
                            .unwrap()
                            .download_dir
                            .clone()
                            .unwrap_or_else(|| {
                                dirs::download_dir()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_default()
                            });
                        let payload = format!(
                            r#"{{"pending":{},"name":"{}","dir":"{}"}}"#,
                            if pending.is_empty() { "false" } else { "true" },
                            pending.replace('\\', "\\\\").replace('"', "\\\""),
                            dir.replace('\\', "\\\\").replace('"', "\\\"")
                        );
                        format!(
                            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n{}\r\n{}",
                            cors, payload
                        )
                    }
                    // 下载设置读取
                    ("GET", "/dl-settings") => {
                        let s = state.settings.lock().unwrap();
                        let dir = s
                            .download_dir
                            .clone()
                            .unwrap_or_else(|| {
                                dirs::download_dir()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_default()
                            });
                        let payload = format!(
                            r#"{{"dir":"{}","ask":{},"autoShow":{},"tint":{},"material":"{}"}}"#,
                            dir.replace('\\', "\\\\").replace('"', "\\\""),
                            s.ask_download_location,
                            s.show_downloads_on_start,
                            s.titlebar_tint,
                            s.window_material
                        );
                        format!(
                            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n{}\r\n{}",
                            cors, payload
                        )
                    }
                    // 设置保存（dir=&ask=&auto=&tint=&material=）
                    ("GET", "/dl-set") => {
                        let dir = url_decode(&q.get("dir").cloned().unwrap_or_default());
                        let ask = q.get("ask").map(|v| v == "1").unwrap_or(false);
                        let auto = q.get("auto").map(|v| v == "1").unwrap_or(true);
                        let tint = q
                            .get("tint")
                            .and_then(|v| v.parse::<u8>().ok())
                            .map(|v| v.min(100));
                        let material = q
                            .get("material")
                            .map(|v| v.to_lowercase())
                            .filter(|v| v == "acrylic" || v == "mica" || v == "none");
                        {
                            let mut s = state.settings.lock().unwrap();
                            s.download_dir = if dir.trim().is_empty() { None } else { Some(dir.trim().to_string()) };
                            s.ask_download_location = ask;
                            s.show_downloads_on_start = auto;
                            if let Some(t) = tint {
                                s.titlebar_tint = t;
                            }
                            if let Some(m) = material.clone() {
                                s.window_material = m;
                            }
                            crate::settings::save_settings(&s);
                        }
                        state.log.info(&format!(
                            "[settings] 已保存 dir={:?} ask={} auto={} tint={:?} material={:?}",
                            state.settings.lock().unwrap().download_dir, ask, auto, tint, material
                        ));
                        // 材质改动立即生效（无需重启）
                        if material.is_some() {
                            if let Some((bg, _)) = state.current_theme() {
                                let app2 = app.clone();
                                let state2 = state.clone();
                                let _ = app.run_on_main_thread(move || {
                                    apply_window_effect(&app2, &state2, &bg);
                                });
                            }
                        }
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
                                    "id": id, "state": "failed", "error": "spec 为空"
                                }));
                            } else {
                                let _ = &profile;
                                let _ = timeout_ms;
                                state.log.info(&format!("[install] 开始安装 {} (id={})", spec, id));
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
            state.log.info(&format!("插件监控：profile 目录不存在 {}", profile_dir));
            return;
        }
        let (tx, rx) = channel::<notify::Result<notify::Event>>();
        let mut watcher = match RecommendedWatcher::new(tx, notify::Config::default()) {
            Ok(w) => w,
            Err(e) => {
                state.log.warn(&format!("插件监控器初始化失败: {}", e));
                return;
            }
        };
        if watcher
            .watch(std::path::Path::new(&profile_dir), RecursiveMode::NonRecursive)
            .is_err()
        {
            state.log.warn("插件监控器 watch 启动失败");
            return;
        }
        state.log.info(&format!("插件变更监控已启动 {}", profile_dir));

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
        state.log.info(&format!("[auto-report] 汇总结果: {}", name));
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

