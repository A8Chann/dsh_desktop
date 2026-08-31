//! dsh 后端管理器：拉起 `dsh web --no-open`、解析就绪 URL、端口探测/接管、
//! 崩溃退避自愈、插件变更只提示（不自动重启）。所有状态经 `backend-status` 事件推送。
use crate::controls::AppState;
use crate::settings::{save_settings, Settings};
use crate::util::{find_pid_by_port, parse_url_line, probe_port, Logger};
use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
pub enum Phase {
    Idle,
    Starting,
    Running,
    External,
    Error,
    Stopped,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallState {
    pub phase: String,
    pub detail: String,
    pub fetched: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackendStatus {
    pub state: String,
    pub url: Option<String>,
    pub port: Option<u16>,
    pub pid: Option<u32>,
    pub owned: bool,
    pub error: Option<String>,
    pub next_retry_sec: Option<u32>,
    pub profile_dir: Option<String>,
    pub install: Option<InstallState>,
    #[serde(skip)]
    #[allow(dead_code)]
    pub gone: bool,
}

impl BackendStatus {
    fn new(state: &str) -> Self {
        Self {
            state: state.to_string(),
            url: None,
            port: None,
            pid: None,
            owned: true,
            error: None,
            next_retry_sec: None,
            profile_dir: None,
            install: None,
            gone: false,
        }
    }
}

pub struct Backend {
    pub app: AppHandle,
    pub settings: Arc<Mutex<Settings>>,
    pub status: Arc<Mutex<BackendStatus>>,
    stop: Arc<AtomicBool>,
    thread: Arc<Mutex<Option<JoinHandle<()>>>>,
    pub log: Arc<Logger>,
}

const RETRY_DELAYS_SEC: [u64; 6] = [1, 2, 4, 8, 15, 30];
const EXTERNAL_HEALTH_MS: u64 = 1000;
const EXTERNAL_FAIL_LIMIT: u32 = 3;

impl Backend {
    pub fn new(app: AppHandle, settings: Arc<Mutex<Settings>>, log: Arc<Logger>) -> Self {
        let profile_dir = settings.lock().unwrap().profile.clone();
        let status = BackendStatus {
            profile_dir: Some(profile_display(&settings)),
            ..BackendStatus::new("idle")
        };
        let _ = profile_dir;
        Self {
            app,
            settings,
            status: Arc::new(Mutex::new(status)),
            stop: Arc::new(AtomicBool::new(false)),
            thread: Arc::new(Mutex::new(None)),
            log,
        }
    }

    pub fn emit(&self) {
        let s = self.status.lock().unwrap().clone();
        let _ = self.app.emit("backend-status", &s);
        let _ = self.app.emit("backend-status-lite", &s.state);
        // 同步推送到主窗页面（右上角菜单直接读全局函数），JSON 安全嵌入 JS
        if let Ok(json) = serde_json::to_string(&s) {
            if let Some(win) = self.app.get_webview_window("main") {
                let js = format!("window.__dshdStatus && window.__dshdStatus({});", json);
                let _ = win.eval(&js);
            }
        }
    }

    pub fn update<F: FnOnce(&mut BackendStatus)>(&self, f: F) {
        let mut s = self.status.lock().unwrap();
        f(&mut s);
        let owned = s.owned;
        let _ = owned;
        let out = s.clone();
        let _ = out;
        drop(s);
        self.emit();
    }

    pub fn set_state(&self, state: &str) {
        self.update(|s| {
            s.state = state.to_string();
            s.error = None;
        });
    }

    pub fn current_status(&self) -> BackendStatus {
        self.status.lock().unwrap().clone()
    }

    /// 启动管理线程（后台运行）。
    pub fn start(&self) {
        self.stop.store(false, Ordering::SeqCst);
        let app = self.app.clone();
        let settings = self.settings.clone();
        let status = self.status.clone();
        let stop = self.stop.clone();
        let log = self.log.clone();
        let handle = std::thread::spawn(move || run_loop(app, settings, status, stop, log));
        *self.thread.lock().unwrap() = Some(handle);
    }

    /// 重启后端：先终止自有进程树（解除管理线程在 stdout 读取上的阻塞），
    /// 等旧循环退出后重新拉起；后端就绪后自动刷新页面（统一各入口行为）。
    pub fn restart(&self, reason: &str) {
        self.log_info(&format!("==== 重启后端: {} ====", reason));
        self.set_state("restarting");
        // stop=true + 杀掉自有后端进程树。管理线程正阻塞在 read_line 等子进程输出，
        // 必须先把子进程杀掉才能让它退出（否则 join 会永久卡死，重启即失效）。
        self.kill_owned();
        let handle = self.thread.lock().unwrap().take();
        if let Some(h) = handle {
            let _ = h.join();
        }
        self.start();

        // 统一：后端重新就绪后自动刷新页面（标题栏菜单/托盘/控制通道同一行为）
        let app = self.app.clone();
        std::thread::spawn(move || {
            for _ in 0..80 {
                std::thread::sleep(Duration::from_millis(500));
                let st = match app.try_state::<Arc<AppState>>() {
                    Some(s) => s.backend.current_status(),
                    None => continue,
                };
                if (st.state == "running" || st.state == "external") && st.url.is_some() {
                    if let Some(w) = app.get_webview_window("main") {
                        let _ = w.eval("location.reload()");
                    }
                    return;
                }
            }
        });
    }

    /// 立即终止自有后端进程树（应用退出前、重启前调用）；外部接管实例（owned=false）不杀。
/// 自带健壮性：① pid 缺失时按端口反查（任意状态，不限于 running）；② 杀完探测端口，
/// 若仍在监听则按端口补杀一次；③ 记录 taskkill 成败，便于日志定位。
pub fn kill_owned(&self) {
    self.stop.store(true, Ordering::SeqCst);
    let s = self.status.lock().unwrap().clone();
    let port = s.port;
    // 自有实例才有权杀；外部接管实例保留。
    let mut pid = if s.owned { s.pid } else { None };
    if pid.is_none() && s.owned {
        if let Some(p) = port {
            pid = crate::util::find_pid_by_port(p);
        }
    }
    let Some(pid) = pid else {
        self.log_info("==== 终止自有后端：未找到 pid（可能未运行） ====");
        return;
    };
    self.log_info(&format!("==== 终止自有后端进程树 pid={} ====", pid));
    if !crate::util::kill_tree(pid) {
        self.log_info(&format!("==== taskkill pid={} 未成功，按端口补杀 ====", pid));
        if let Some(p) = port {
            if let Some(pid2) = crate::util::find_pid_by_port(p) {
                let ok2 = crate::util::kill_tree(pid2);
                self.log_info(&format!(
                    "==== 补杀 pid={} 结果={} ====",
                    pid2,
                    if ok2 { "成功" } else { "失败" }
                ));
            } else {
                self.log_info("==== 补杀：端口已无进程 ====");
            }
        }
    } else {
        // 杀成功但保险起见仍探测端口，残留则补杀
        if let Some(p) = port {
            if crate::util::probe_port(p, 500).0 {
                if let Some(pid2) = crate::util::find_pid_by_port(p) {
                    self.log_info(&format!("==== 端口仍被占用，补杀 pid={} ====", pid2));
                    crate::util::kill_tree(pid2);
                }
            }
        }
    }
}

    pub fn log_info(&self, msg: &str) {
        self.log.info(msg);
    }

    /// 等待插件变更提示（模拟原本的安静期）；由 fs 监控另行触发提示。
    /// Windows 下走 win_toast（tauri-plugin-notification 在 Windows 上无法设置
    /// toast 小图标——notify-rust 忽略 icon 字段，图标只随 AUMID 解析，
    /// 便携版 AUMID 未注册会回退到默认图标）。
    pub fn on_plugin_change(&self) {
        self.log_info("插件已变更：请手动重启后端以加载新插件");
        #[cfg(windows)]
        {
            if let Err(e) = crate::win_toast::show_plugin_change_toast() {
                self.log_info(&format!("插件变更 toast 发送失败: {e}"));
            }
        }
        #[cfg(not(windows))]
        {
            let _ = tauri_plugin_notification::NotificationExt::notification(&self.app)
                .builder()
                .title("DSH Desktop — 插件已变更")
                .body("请手动重启后端（悬浮条「重启」按钮）以加载新插件")
                .show();
        }
    }
}

fn profile_display(settings: &Mutex<Settings>) -> String {
    let s = settings.lock().unwrap();
    format!(
        "{}/profiles/{}",
        std::env::var("DSH_HOME")
            .unwrap_or_else(|_| {
                std::env::var("USERPROFILE")
                    .unwrap_or_else(|_| String::from("."))
                    .replace('\\', "/")
                    + "/.dsh"
            }),
        s.profile
    )
}

/// 统一状态发布：写回共享状态 + emit 事件 + 推送主窗右上角菜单。
fn publish(app: &AppHandle, status: &Arc<Mutex<BackendStatus>>, st: BackendStatus) {
    *status.lock().unwrap() = st.clone();
    let _ = app.emit("backend-status", &st);
    if let Ok(json) = serde_json::to_string(&st) {
        if let Some(win) = app.get_webview_window("main") {
            let js = format!("window.__dshdStatus && window.__dshdStatus({});", json);
            let _ = win.eval(&js);
        }
    }
}

fn spawn_own(
    app: AppHandle,
    settings: Arc<Mutex<Settings>>,
    status: Arc<Mutex<BackendStatus>>,
    stop: Arc<AtomicBool>,
    log: Arc<Logger>,
) {
    // 1) 解析 node/dsh
    let (node_bin, dsh_bin) = {
        let s = settings.lock().unwrap();
        (
            resolve_node(s.node_bin.as_deref()),
            resolve_dsh(s.dsh_bin.as_deref()),
        )
    };

    let dsh_bin = match dsh_bin {
        Some(b) => b,
        None => {
            // 未找到 dsh：默认安装（测速选最快源 + npm 安装）。
            // 这一步耗时可达 1~2 分钟，必须持续上报 install 进度，
            // 否则启动页只有一行静态文字，看起来像卡死。
            let report = |phase: &str, detail: &str, fetched: u32| {
                let st = BackendStatus {
                    state: "starting".to_string(),
                    install: Some(InstallState {
                        phase: phase.to_string(),
                        detail: detail.to_string(),
                        fetched,
                    }),
                    ..BackendStatus::new("starting")
                };
                publish(&app, &status, st);
            };
            match default_install_dsh(&node_bin, &log, &report) {
                Ok(bin) => {
                    {
                        let mut s = settings.lock().unwrap();
                        s.dsh_bin = Some(bin.clone());
                        save_settings(&s);
                    }
                    bin
                }
                Err(e) => {
                    fail(app, status, &stop, &format!("默认安装 dsh 失败: {}", e));
                    return;
                }
            }
        }
    };

    // 2) 端口探测（外部实例接管）
    let port = settings.lock().unwrap().port;
    if port != 0 {
        let (ok, is_dsh) = probe_port(port, 2500);
        if ok && is_dsh {
            let pid = find_pid_by_port(port).unwrap_or(0);
            log.info(&format!("[backend] 接管外部实例 pid={} 端口={}（退出时不杀）", pid, port));
            adopt_external(app.clone(), port, pid, status.clone(), stop.clone());
            return;
        }
        if ok && !is_dsh {
            // 被非 dsh 服务占用 → 回退随机端口
            let mut s = settings.lock().unwrap();
            s.port = 0;
        }
    }

    let port = settings.lock().unwrap().port;
    let workspace = settings.lock().unwrap().workspace.clone().unwrap_or_default();

    // 3) 拉起
    log.info(&format!(
        "[backend] 尝试启动：node={} dsh={} args=web --no-open --port={} cwd={}",
        node_bin, dsh_bin, port, workspace
    ));
    let mut child = match Command::new(&node_bin)
        .args([&dsh_bin, "web", "--no-open", "--port", &port.to_string()])
        .current_dir(&workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            fail(app, status, &stop, &format!("无法启动 dsh 进程: {}", e));
            return;
        }
    };

    let pid = child.id();
    log.info(&format!("[backend] 拉起自有后端 pid={} 端口={}", pid, port));
    // 状态推送
    {
        let st = BackendStatus {
            state: "starting".to_string(),
            port: Some(port),
            pid: Some(pid),
            ..BackendStatus::new("starting")
        };
        publish(&app, &status, st);
    }

    let stdout = child.stdout.take().map(BufReader::new);
    if let Some(er) = child.stderr.take() {
        let app2 = app.clone();
        let log2 = log.clone();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(er);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        let t = line.trim_end();
                        if !t.is_empty() {
                            let _ = app2.emit("backend-log", t.to_string());
                            log2.info(&format!("[backend-stderr] {}", t)); // 落盘，便于排查启动失败
                        }
                    }
                }
            }
        });
    }

    let mut url: Option<String> = None;
    if let Some(mut out) = stdout {
        let mut line = String::new();
        loop {
            if stop.load(Ordering::SeqCst) {
                let _ = child.kill();
                break;
            }
            line.clear();
            match out.read_line(&mut line) {
                Ok(0) => break,
                Err(_) => break,
                Ok(_) => {
                    let t = line.trim_end();
                    if t.is_empty() {
                        continue;
                    }
                    let _ = app.emit("backend-log", t.to_string());
                    if url.is_none() {
                        if let Some(u) = parse_url_line(t) {
                            url = Some(u.clone());
                            let st = BackendStatus {
                                state: "running".to_string(),
                                url: Some(u),
                                port: Some(port),
                                pid: Some(pid),
                                owned: true,
                                error: None,
                                next_retry_sec: None,
                                profile_dir: None,
                                install: None,
                                gone: false,
                            };
                            publish(&app, &status, st);
                        }
                    }
                }
            }
        }
    }

    // 4) 等待退出
    let code = child.wait();

    let exit_code = match code {
        Ok(s) => s.code(),
        Err(_) => None,
    };

    if stop.load(Ordering::SeqCst) {
        return; // 主动停止，不再重试
    }

    match url {
        Some(_) => {
            fail(
                app,
                status,
                &stop,
                &format!(
                    "后端进程退出 (code={:?})，安排自动重启",
                    exit_code
                ),
            );
        }
        None => {
            fail(
                app,
                status,
                &stop,
                &format!("dsh 启动失败 (code={:?})", exit_code),
            );
        }
    }
}

fn adopt_external(
    app: AppHandle,
    port: u16,
    pid: u32,
    status: Arc<Mutex<BackendStatus>>,
    stop: Arc<AtomicBool>,
) {
    let url = format!("http://127.0.0.1:{}", port);
    let st = BackendStatus {
        state: "external".to_string(),
        url: Some(url.clone()),
        port: Some(port),
        pid: Some(pid),
        owned: false,
        ..BackendStatus::new("external")
    };
    publish(&app, &status, st);

    // 健康检查：外部实例消失后接管（回落到 spawn 自己的）
    let mut fail_streak = 0u32;
    loop {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        std::thread::sleep(Duration::from_millis(EXTERNAL_HEALTH_MS));
        let (ok, _) = probe_port(port, 1500);
        if ok {
            fail_streak = 0;
        } else {
            fail_streak += 1;
            if fail_streak >= EXTERNAL_FAIL_LIMIT {
                // 外部实例已消失：由调用方继续 spawn 自己的（直接 return，外层继续）
                return;
            }
        }
    }
}

/// 发布错误态并按退避重试。保留已知的 port/pid/profile_dir——
/// 整体重建会把它们清空，导致 kill_owned 之后无法按端口反查补杀。
fn fail(app: AppHandle, status: Arc<Mutex<BackendStatus>>, stop: &Arc<AtomicBool>, msg: &str) {
    let base = status.lock().unwrap().clone();
    let err_status = |secs: Option<u32>| BackendStatus {
        state: "error".to_string(),
        url: None,
        error: Some(msg.to_string()),
        next_retry_sec: secs,
        install: None,
        ..base.clone()
    };
    publish(&app, &status, err_status(Some(RETRY_DELAYS_SEC[0] as u32)));
    if stop.load(Ordering::SeqCst) {
        return;
    }
    let mut retry = 0u32;
    loop {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        let idx = (retry as usize).min(RETRY_DELAYS_SEC.len() - 1);
        let secs = RETRY_DELAYS_SEC[idx];
        publish(&app, &status, err_status(Some(secs as u32)));
        // 退避 sleep 按 1 秒切片，保证 stop（重启/退出）能及时打断
        for _ in 0..secs {
            if stop.load(Ordering::SeqCst) {
                return;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
        if stop.load(Ordering::SeqCst) {
            return;
        }
        retry += 1;
        // 重试上限后仍失败则继续尝试（退避到 30s 封顶）
        if retry > 10 {
            break;
        }
    }
}

fn run_loop(
    app: AppHandle,
    settings: Arc<Mutex<Settings>>,
    status: Arc<Mutex<BackendStatus>>,
    stop: Arc<AtomicBool>,
    log: Arc<Logger>,
) {
    log.info("dsh 后端管理线程启动");
    let mut attempts = 0u32;
    loop {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        // 插件变更提示阈值：等待安装结束由 fs 监控触发，这里仅保证循环可持续
        spawn_own(
            app.clone(),
            settings.clone(),
            status.clone(),
            stop.clone(),
            log.clone(),
        );
        attempts += 1;
        if attempts > 100 {
            // 彻底放弃：必须发一个没有 next_retry_sec 的终态，
            // 否则界面停在“异常 · N 秒后重试”，用户不知道已经不再自愈。
            log.info("dsh 后端连续失败次数过多，停止自动重试（需手动「重启 Web 服务」）");
            let base = status.lock().unwrap().clone();
            publish(
                &app,
                &status,
                BackendStatus {
                    state: "stopped".to_string(),
                    url: None,
                    error: Some(
                        "自动重试次数已用尽，请从标题栏 ⋯ 菜单手动「重启 Web 服务」".to_string(),
                    ),
                    next_retry_sec: None,
                    install: None,
                    ..base
                },
            );
            break;
        }
    }
}

// ─────────────────────────── 解析 node / dsh ───────────────────────────

fn resolve_node(explicit: Option<&str>) -> String {
    if let Some(p) = explicit {
        if std::path::Path::new(p).exists() {
            return p.to_string();
        }
    }
    let cands = [
        "C:\\Program Files\\nodejs\\node.exe",
        "C:\\Program Files (x86)\\nodejs\\node.exe",
    ];
    for c in cands {
        if std::path::Path::new(c).exists() {
            return c.to_string();
        }
    }
    // PATH 上找 node
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            let p = dir.join("node.exe");
            if p.exists() {
                return p.to_string_lossy().to_string();
            }
        }
    }
    "node.exe".to_string()
}

fn resolve_dsh(explicit: Option<&str>) -> Option<String> {
    if let Some(p) = explicit {
        if std::path::Path::new(p).exists() && p.ends_with(".js") {
            return Some(p.to_string());
        }
        return None;
    }
    // PATH 上找 dsh / dsh.cmd
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            for name in ["dsh.cmd", "dsh.bat", "dsh.exe"] {
                let p = dir.join(name);
                if p.exists() {
                    // 优先：npm 全局布局的真实 bin.js（<npm>\node_modules\@deepseek-ai\dsh\lib\bin.js）
                    let binjs = dir
                        .join("node_modules")
                        .join("@deepseek-ai")
                        .join("dsh")
                        .join("lib")
                        .join("bin.js");
                    if binjs.exists() {
                        return Some(binjs.to_string_lossy().to_string());
                    }
                    // 解析 shim 内容中引用的 bin.js 路径（npm cmd shim 形如：node "%~dp0\node_modules\...\bin.js" %*）
                    if let Ok(content) = std::fs::read_to_string(&p) {
                        for line in content.lines() {
                            if !line.to_lowercase().contains("bin.js") {
                                continue;
                            }
                            if let Some(qs) = line.find('"').map(|i| &line[i + 1..]) {
                                if let Some(qe) = qs.find('"') {
                                    let raw = &qs[..qe];
                                    let expanded = raw
                                        .replace("%~dp0", "")
                                        .replace("%dp0", "")
                                        .trim_start_matches(['\\', ' ', '\t', '/'])
                                        .to_string();
                                    let full = if std::path::Path::new(&expanded).is_absolute() {
                                        expanded
                                    } else {
                                        dir.join(&expanded).to_string_lossy().to_string()
                                    };
                                    if std::path::Path::new(&full).exists() {
                                        return Some(full);
                                    }
                                }
                            }
                        }
                    }
                    // 兜底：返回 shim 本身（node 直接执行 .cmd 会失败——仅在最坏情况保留）
                    return Some(p.to_string_lossy().to_string());
                }
            }
            let p = dir.join("node_modules").join("@deepseek-ai").join("dsh").join("lib").join("bin.js");
            if p.exists() {
                return Some(p.to_string_lossy().to_string());
            }
        }
    }
    // 兜底：npm 全局目录（%APPDATA%\npm\node_modules）
    if let Ok(appdata) = std::env::var("APPDATA") {
        let p = PathBuf::from(&appdata)
            .join("npm")
            .join("node_modules")
            .join("@deepseek-ai")
            .join("dsh")
            .join("lib")
            .join("bin.js");
        if p.exists() {
            return Some(p.to_string_lossy().to_string());
        }
    }
    None
}

// ─────────────────────────── 默认安装（测速选最快源 + npm 全局安装） ───────────────────────────

const REGISTRY_CANDIDATES: [(&str, &str); 4] = [
    ("npmmirror（国内）", "https://registry.npmmirror.com/"),
    ("npmjs（官方）", "https://registry.npmjs.org/"),
    ("腾讯云镜像", "https://mirrors.cloud.tencent.com/npm/"),
    ("华为云镜像", "https://repo.huaweicloud.com/repository/npm/"),
];

fn registry_latency(url: &str) -> Option<u64> {
    use std::net::{TcpStream, ToSocketAddrs};
    let host_port = url
        .trim_start_matches("https://")
        .trim_end_matches('/')
        .to_string();
    let start = std::time::Instant::now();
    // TCP 握手延迟近似（TLS 握手不在内，仅用作排序）
    let addr = match (host_port.as_str(), 443u16).to_socket_addrs() {
        Ok(mut it) => it.next(),
        Err(_) => None,
    };
    match addr {
        Some(a) => match TcpStream::connect_timeout(&a, Duration::from_millis(4000)) {
            Ok(_) => Some(start.elapsed().as_millis() as u64),
            Err(_) => None,
        },
        None => None,
    }
}

/// 默认安装 dsh：测速选最快 npm 源 → `npm install -g @deepseek-ai/dsh@latest` → 定位 bin.js。
/// `report(phase, detail, fetched)` 在每个阶段回调，用于把进度推到启动页（否则这 1~2 分钟毫无反馈）。
fn default_install_dsh(
    node_bin: &str,
    log: &Arc<Logger>,
    report: &dyn Fn(&str, &str, u32),
) -> Result<String, String> {
    // 1) 测速选源（TCP 握手排序，4s 超时）
    report("measuring", "正在测速选择最快的 npm 源…", 0);
    let mut scored: Vec<(u64, &str, &str)> = Vec::new();
    for (name, url) in REGISTRY_CANDIDATES {
        if let Some(ms) = registry_latency(url) {
            scored.push((ms, name, url));
        }
    }
    scored.sort_by_key(|(ms, _, _)| *ms);
    let (_, name, registry) = scored
        .first()
        .copied()
        .ok_or("所有 npm 源均无法连接")?;
    let reg = registry.to_string();
    log.info(&format!("[install] 选用 npm 源：{} {}", name, reg));

    // 2) node npm-cli.js install -g @deepseek-ai/dsh@latest --registry <reg>
    let npm_cli = PathBuf::from(node_bin)
        .parent()
        .map(|d| {
            d.join("node_modules")
                .join("npm")
                .join("bin")
                .join("npm-cli.js")
        })
        .filter(|p| p.exists())
        .ok_or("无法定位 npm（npm-cli.js）")?;

    report("installing", &format!("正在从 {} 安装 dsh…", name), 0);
    let mut child = Command::new(node_bin)
        .args([
            npm_cli.to_str().unwrap_or(""),
            "install",
            "-g",
            "@deepseek-ai/dsh@latest",
            "--registry",
            &reg,
            "--loglevel",
            "info",
            "--no-fund",
            "--no-audit",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|e| format!("npm 安装 dsh 失败: {}", e))?;

    // 流式读 stderr：npm --loglevel info 每抓一个包打一行 "http fetch"，据此报进度；
    // 同时留存尾部若干行，失败时作为错误详情。
    let mut tail: Vec<String> = Vec::new();
    let mut fetched = 0u32;
    if let Some(err) = child.stderr.take() {
        let mut reader = BufReader::new(err);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let t = line.trim_end().to_string();
                    if t.is_empty() {
                        continue;
                    }
                    if t.contains("http fetch") {
                        fetched += 1;
                        report(
                            "installing",
                            &format!("正在从 {} 下载依赖…", name),
                            fetched,
                        );
                    }
                    tail.push(t);
                    if tail.len() > 40 {
                        tail.remove(0);
                    }
                }
            }
        }
    }
    let st = child.wait().map_err(|e| format!("npm 安装 dsh 失败: {}", e))?;
    if !st.success() {
        let detail: String = tail.iter().rev().take(4).cloned().collect::<Vec<_>>().join(" ");
        let detail = if detail.is_empty() {
            String::new()
        } else {
            format!(": {}", detail)
        };
        return Err(format!("npm 安装 dsh 失败 (exit={:?}){}", st.code(), detail));
    }

    // 3) npm root -g 定位全局目录
    report("locating", "安装完成，正在定位 dsh…", fetched);
    let out = Command::new(node_bin)
        .args([npm_cli.to_str().unwrap_or(""), "root", "-g"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("npm root -g 失败: {}", e))?;
    let root = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if root.is_empty() {
        return Err("无法确定 npm 全局安装目录".to_string());
    }
    let bin = PathBuf::from(&root)
        .join("node_modules")
        .join("@deepseek-ai")
        .join("dsh")
        .join("lib")
        .join("bin.js");
    if !bin.exists() {
        return Err(format!("安装完成但未找到 dsh 入口: {}", bin.display()));
    }
    log.info(&format!("[install] dsh 安装完成：{}", bin.display()));
    Ok(bin.to_string_lossy().to_string())
}
