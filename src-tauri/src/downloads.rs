//! 自管下载管理器：拦截 WebView2 的下载请求（on_download 返回 false），
//! 由 Rust 后台线程流式下载到系统下载目录，支持进度/暂停/继续/取消，
//! 并向注入面板提供 /downloads 列表。

use crate::util::Logger;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct DownloadTask {
    pub id: u64,
    pub name: String,
    pub url: String,
    pub file: String,
    pub state: String, // downloading / paused / done / canceled / error
    pub bytes: u64,
    pub total: Option<u64>,
    pub error: Option<String>,
    /// 服务端资源版本（ETag 或 Last-Modified）：续传时经 If-Range 校验，
    /// 文件在暂停期间变过就整体重下，避免新旧字节拼接成损坏文件。
    pub etag: Option<String>,
    /// 文件是否仍在磁盘（运行时计算，不入持久化）
    #[serde(default)]
    pub exists: bool,
    /// 当前速度 B/s（运行时统计，不入持久化）
    #[serde(default)]
    pub speed: u64,
}

impl Default for DownloadTask {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            url: String::new(),
            file: String::new(),
            state: "error".to_string(),
            bytes: 0,
            total: None,
            error: None,
            etag: None,
            exists: false,
            speed: 0,
        }
    }
}

struct Inner {
    tasks: Mutex<Vec<DownloadTask>>,
    next_id: AtomicU64,
    flags: Mutex<std::collections::HashMap<u64, (Arc<AtomicBool>, Arc<AtomicBool>)>>, // (cancel, pause)
    last_save: Mutex<Option<Instant>>,
    /// 状态变化通知（外壳层即时刷新徽标/面板）
    notify: Arc<dyn Fn() + Send + Sync>,
}

impl Inner {
    fn fire(&self) {
        (self.notify)();
    }
    /// 持久化到磁盘（限流：2 秒内不重复写，除非 force）。
    fn maybe_persist(&self, force: bool) {
        let tasks = self.tasks.lock().unwrap().clone();
        let mut ls = self.last_save.lock().unwrap();
        let due = match *ls {
            Some(t) => t.elapsed() >= Duration::from_secs(2),
            None => true,
        };
        if !force && !due {
            return;
        }
        *ls = Some(Instant::now());
        drop(ls);
        let _ = std::fs::create_dir_all(persist_path().parent().unwrap_or(Path::new(".")));
        let _ = std::fs::write(persist_path(), serde_json::to_string(&tasks).unwrap_or_default());
    }
}

pub struct Downloads {
    inner: Arc<Inner>,
    log: Arc<Logger>,
    /// “下载前询问保存位置”的待确认项 (url, name)
    pending: Mutex<Option<(String, String)>>,
    /// 单个任务下载完成时回调（用于系统通知）
    on_done: Arc<dyn Fn(&str) + Send + Sync>,
}

fn persist_path() -> PathBuf {
    crate::settings::settings_dir().join("downloads.json")
}

impl Downloads {
    pub fn new(
        log: Arc<Logger>,
        notify: Arc<dyn Fn() + Send + Sync>,
        on_done: Arc<dyn Fn(&str) + Send + Sync>,
    ) -> Self {
        let mut tasks: Vec<DownloadTask> = std::fs::read_to_string(persist_path())
            .ok()
            .and_then(|text| serde_json::from_str::<Vec<DownloadTask>>(&text).ok())
            .unwrap_or_default();
        // 重启后遗留的“下载中/暂停”任务：进度文件还在就保留为可续传的暂停态
        // （点“继续”会用 Range 接着下），否则才算失败。
        for t in tasks.iter_mut() {
            if t.state == "downloading" || t.state == "paused" {
                let on_disk = std::fs::metadata(&t.file).map(|m| m.len()).unwrap_or(0);
                if on_disk > 0 {
                    t.bytes = on_disk; // 以磁盘实际大小为准，避免记录比文件超前
                    t.state = "paused".to_string();
                    t.error = None;
                } else {
                    t.state = "error".to_string();
                    if t.error.is_none() {
                        t.error = Some("程序重启，下载中断".to_string());
                    }
                }
            }
            t.speed = 0;
        }
        let max_id = tasks.iter().map(|t| t.id).max().unwrap_or(0);
        Self {
            inner: Arc::new(Inner {
                tasks: Mutex::new(tasks),
                next_id: AtomicU64::new(max_id + 1),
                flags: Mutex::new(std::collections::HashMap::new()),
                last_save: Mutex::new(None),
                notify,
            }),
            log,
            pending: Mutex::new(None),
            on_done,
        }
    }

    /// 收到下载请求（ask=true 时进入待确认，否则直接开始）。
    /// 返回 true 表示请求已被吸收（待确认或已开始）；false 表示重复/失败。
    pub fn offer(&self, url: &str, name: &str, dir: PathBuf, ask: bool) -> bool {
        if ask {
            if self.find_dup(url) {
                return false;
            }
            *self.pending.lock().unwrap() = Some((url.to_string(), name.to_string()));
            self.log.info(&format!("[download] 待确认(询问保存位置) url={}", url));
            self.inner.fire();
            true
        } else {
            self.start(url, name, dir).is_some()
        }
    }

    pub fn take_pending(&self) -> Option<(String, String)> {
        self.pending.lock().unwrap().take()
    }

    pub fn clear_pending(&self) {
        *self.pending.lock().unwrap() = None;
    }

    pub fn pending_name(&self) -> Option<String> {
        self.pending.lock().unwrap().as_ref().map(|(_, n)| n.clone())
    }

    /// 持久化（限流调用入口）。
    fn maybe_persist(&self, force: bool) {
        self.inner.maybe_persist(force);
    }

    /// 从现有任务里找同名未完成的（避免重复）
    fn find_dup(&self, url: &str) -> bool {
        let t = self.inner.tasks.lock().unwrap();
        t.iter()
            .any(|x| x.url == url && (x.state == "downloading" || x.state == "paused"))
    }

    /// 启动一个下载。name 为展示文件名，url 为实际下载地址，dir 为目标目录。
    pub fn start(&self, url: &str, name: &str, dir: PathBuf) -> Option<u64> {
        if self.find_dup(url) {
            return None;
        }
        // 目录可能是用户在设置里手输的、尚不存在的路径
        if let Err(e) = std::fs::create_dir_all(&dir) {
            self.log.info(&format!("[download] 创建目录失败 {}: {}", dir.display(), e));
        }
        let id = self.inner.next_id.fetch_add(1, Ordering::SeqCst);
        // 同名文件加序号，不覆盖已有文件（浏览器行为）
        let file = unique_path(&dir, &sanitize(name));
        let shown_name = file
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| name.to_string());
        {
            let mut t = self.inner.tasks.lock().unwrap();
            t.insert(
                0,
                DownloadTask {
                    id,
                    name: shown_name,
                    url: url.to_string(),
                    file: file.to_string_lossy().to_string(),
                    state: "downloading".to_string(),
                    ..DownloadTask::default()
                },
            );
        }
        self.maybe_persist(true);
        self.log.info(&format!("[download] start id={} url={} -> {}", id, url, file.display()));
        self.inner.fire();
        self.spawn_worker(id, url.to_string(), file, 0);
        Some(id)
    }

    /// 派发下载线程。`resume_from` > 0 时从该字节数续传（重启后恢复用）。
    fn spawn_worker(&self, id: u64, url: String, file: PathBuf, resume_from: u64) {
        let cancel = Arc::new(AtomicBool::new(false));
        let pause = Arc::new(AtomicBool::new(false));
        self.inner
            .flags
            .lock()
            .unwrap()
            .insert(id, (cancel.clone(), pause.clone()));

        let me = self.inner.clone();
        let log = self.log.clone();
        let on_done = self.on_done.clone();
        std::thread::spawn(move || {
            // catch_unwind：下载线程任何 panic 都不允许拖垮整个进程（记录为失败任务）
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run_download(me.clone(), id, &url, &file, cancel.clone(), pause.clone(), resume_from)
            }))
            .unwrap_or_else(|e| {
                let msg = if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "下载线程 panic".to_string()
                };
                Err(format!("内部错误: {}", msg))
            });
            let (state, err, bytes, total) = match result {
                Ok((bytes, total)) => ("done".to_string(), None, bytes, Some(total)),
                Err(e) => {
                    let c = cancel.load(Ordering::SeqCst);
                    let s = if c { "canceled".to_string() } else { "error".to_string() };
                    (s, Some(e), 0u64, None)
                }
            };
            let mut done_name = String::new();
            {
                let mut t = me.tasks.lock().unwrap();
                if let Some(task) = t.iter_mut().find(|x| x.id == id) {
                    task.state = state.to_string();
                    task.error = err.clone();
                    task.speed = 0;
                    if state == "done" {
                        task.bytes = bytes;
                        task.total = total;
                        done_name = task.name.clone();
                    }
                }
            }
            me.flags.lock().unwrap().remove(&id);
            me.maybe_persist(true); // 持久化最终状态
            me.fire(); // 通知外壳层即时刷新（徽标/面板）
            if !done_name.is_empty() {
                on_done(&done_name);
            }
            log.info(&format!(
                "[download] finished id={} state={} err={:?}",
                id, state, err
            ));
        });
    }

    pub fn cancel(&self, id: u64) -> bool {
        let found = {
            let f = self.inner.flags.lock().unwrap();
            f.get(&id).map(|(c, _)| c.store(true, Ordering::SeqCst)).is_some()
        };
        if found {
            self.log.info(&format!("[download] cancel id={}", id));
        }
        found
    }

    /// 暂停 / 继续。继续时若线程已不存在（重启后恢复的任务），按磁盘已有字节续传。
    pub fn pause(&self, id: u64, on: bool) -> bool {
        let has_worker = self.inner.flags.lock().unwrap().contains_key(&id);
        if has_worker {
            let f = self.inner.flags.lock().unwrap();
            let Some((_, p)) = f.get(&id) else { return false };
            p.store(on, Ordering::SeqCst);
            drop(f);
            {
                let mut t = self.inner.tasks.lock().unwrap();
                if let Some(task) = t.iter_mut().find(|x| x.id == id) {
                    task.state = if on { "paused".to_string() } else { "downloading".to_string() };
                    if on {
                        task.speed = 0;
                    }
                }
            }
            self.log.info(&format!("[download] pause(id={})={}", id, on));
            self.maybe_persist(false);
            self.inner.fire();
            return true;
        }
        if on {
            return false; // 没有线程可暂停
        }
        // 继续一个重启后恢复的任务：从磁盘现有字节数续传
        let Some((url, file)) = ({
            let mut t = self.inner.tasks.lock().unwrap();
            t.iter_mut().find(|x| x.id == id).map(|task| {
                task.state = "downloading".to_string();
                task.error = None;
                (task.url.clone(), PathBuf::from(task.file.clone()))
            })
        }) else {
            return false;
        };
        let from = std::fs::metadata(&file).map(|m| m.len()).unwrap_or(0);
        self.log.info(&format!("[download] resume id={} from={} 字节", id, from));
        self.maybe_persist(true);
        self.inner.fire();
        self.spawn_worker(id, url, file, from);
        true
    }

    /// 全部暂停（仅对进行中的任务）。返回受影响数量。
    pub fn pause_all(&self) -> usize {
        let ids: Vec<u64> = self
            .inner
            .tasks
            .lock()
            .unwrap()
            .iter()
            .filter(|t| t.state == "downloading")
            .map(|t| t.id)
            .collect();
        let n = ids.len();
        for id in ids {
            self.pause(id, true);
        }
        n
    }

    /// 下载列表（最新在前）；为每个任务标注文件是否仍在磁盘（exists）——前端据此区分“已删除”态。
    pub fn list(&self) -> Vec<DownloadTask> {
        let mut t = self.inner.tasks.lock().unwrap().clone();
        for task in t.iter_mut() {
            task.exists = !task.file.is_empty() && std::path::Path::new(&task.file).exists();
        }
        t
    }

    /// 删除任务：进行中先取消；移除记录；若文件存在则一并删除。
    /// 返回文件是否确实已不在磁盘——删除失败（被占用/只读）时返回 false，
    /// 否则前端会提示“已删除”但文件仍留在盘上，且列表里再也找不到它。
    pub fn delete(&self, id: u64) -> bool {
        let (removed_file, active) = {
            let mut t = self.inner.tasks.lock().unwrap();
            let Some(idx) = t.iter().position(|x| x.id == id) else {
                return false;
            };
            let task = t.remove(idx);
            (task.file, task.state == "downloading" || task.state == "paused")
        };
        let mut file_gone = true;
        if active {
            // 通知下载线程停止（线程结束时会 remove_file）
            if let Some((c, _)) = self.inner.flags.lock().unwrap().get(&id) {
                c.store(true, Ordering::SeqCst);
            }
        } else if std::path::Path::new(&removed_file).exists() {
            match std::fs::remove_file(&removed_file) {
                Ok(_) => {}
                Err(e) => {
                    file_gone = false;
                    self.log.info(&format!(
                        "[download] delete id={} 文件删除失败（记录已移除）: {} - {}",
                        id, removed_file, e
                    ));
                }
            }
        }
        self.maybe_persist(true);
        if file_gone {
            self.log.info(&format!("[download] delete id={} file={}", id, removed_file));
        }
        self.inner.fire();
        file_gone
    }

    /// 仅从列表移除记录（保留文件）。进行中先取消。
    pub fn remove(&self, id: u64) -> bool {
        let active = {
            let mut t = self.inner.tasks.lock().unwrap();
            let Some(idx) = t.iter().position(|x| x.id == id) else {
                return false;
            };
            let st = t[idx].state.clone();
            t.remove(idx);
            st == "downloading" || st == "paused"
        };
        if active {
            if let Some((c, _)) = self.inner.flags.lock().unwrap().get(&id) {
                c.store(true, Ordering::SeqCst);
            }
        }
        self.maybe_persist(true);
        self.inner.fire();
        true
    }

    /// 是否有进行中的任务（用于面板“点外部不关闭”与徽标）。
    pub fn has_active(&self) -> bool {
        let t = self.inner.tasks.lock().unwrap();
        t.iter()
            .any(|x| x.state == "downloading" || x.state == "paused")
    }

    /// 取任务文件路径（存在性校验由调用方做）。
    pub fn file_of(&self, id: u64) -> Option<String> {
        self.inner
            .tasks
            .lock()
            .unwrap()
            .iter()
            .find(|x| x.id == id)
            .map(|x| x.file.clone())
    }

    /// 重试：**原地复用同一条目**（另起新任务会在列表里留下失败的僵尸记录）。
    pub fn retry(&self, id: u64) -> bool {
        if self.inner.flags.lock().unwrap().contains_key(&id) {
            return false; // 还在跑，不重复起
        }
        let Some((url, file)) = ({
            let mut t = self.inner.tasks.lock().unwrap();
            t.iter_mut().find(|x| x.id == id).map(|task| {
                task.state = "downloading".to_string();
                task.error = None;
                task.bytes = 0;
                task.speed = 0;
                task.etag = None;
                (task.url.clone(), PathBuf::from(task.file.clone()))
            })
        }) else {
            return false;
        };
        self.log.info(&format!("[download] retry id={} url={}", id, url));
        self.maybe_persist(true);
        self.inner.fire();
        self.spawn_worker(id, url, file, 0); // 从 0 重下
        true
    }

    /// 取任务信息（转交浏览器下载用）
    pub fn url_of(&self, id: u64) -> Option<String> {
        self.inner
            .tasks
            .lock()
            .unwrap()
            .iter()
            .find(|x| x.id == id)
            .map(|x| x.url.clone())
    }
}

/// 下载线程主体。要点：
/// - 总长优先取 GET 响应的 Content-Length（很多服务器不支持 HEAD，只靠 HEAD 会没有进度条）；
/// - 暂停即**断开连接**，恢复时用 `Range` 续传，并带 `If-Range` 校验资源版本——
///   文件在暂停期间变过时服务端会回 200 全量，此处据此丢弃旧字节重下，避免拼出损坏文件；
/// - 每 500ms 统计一次速度供界面显示。
fn run_download(
    me: Arc<Inner>,
    id: u64,
    url: &str,
    file: &PathBuf,
    cancel: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,
    resume_from: u64,
) -> Result<(u64, u64), String> {
    // 资源版本：优先用已记录的（重启恢复场景），否则 HEAD 探一次
    let mut total: Option<u64> = None;
    let mut version: Option<String> = me
        .tasks
        .lock()
        .unwrap()
        .iter()
        .find(|x| x.id == id)
        .and_then(|t| t.etag.clone());
    if let Ok(r) = ureq::head(url).timeout(Duration::from_secs(15)).call() {
        total = r
            .header("content-length")
            .and_then(|s| s.trim().parse::<u64>().ok());
        if version.is_none() {
            version = r
                .header("etag")
                .or_else(|| r.header("last-modified"))
                .map(|s| s.to_string());
        }
    }

    let mut bytes: u64 = resume_from;
    loop {
        if cancel.load(Ordering::SeqCst) {
            let _ = std::fs::remove_file(file);
            return Err("已取消".to_string());
        }

        let mut req = ureq::get(url).timeout(Duration::from_secs(60));
        if bytes > 0 {
            req = req.set("Range", &format!("bytes={}-", bytes));
            // 资源变了就别拿旧字节拼：服务端会忽略 Range 并回 200 全量
            if let Some(v) = version.as_deref() {
                req = req.set("If-Range", v);
            }
        }
        let resp = req.call().map_err(|e| format!("请求失败: {}", e))?;
        let resuming = resp.status() == 206;
        if bytes > 0 && !resuming {
            bytes = 0; // 服务端不接受续传（或资源已变）→ 整体重下
        }
        // 总长兜底：HEAD 拿不到时用 GET 的 Content-Length（206 时它是剩余长度，需加上已下载）
        if total.is_none() {
            if let Some(len) = resp
                .header("content-length")
                .and_then(|s| s.trim().parse::<u64>().ok())
            {
                total = Some(if resuming { bytes + len } else { len });
            }
        }
        if version.is_none() {
            version = resp
                .header("etag")
                .or_else(|| resp.header("last-modified"))
                .map(|s| s.to_string());
        }
        {
            let mut t = me.tasks.lock().unwrap();
            if let Some(task) = t.iter_mut().find(|x| x.id == id) {
                task.total = total;
                task.etag = version.clone();
                task.bytes = bytes;
            }
        }

        let mut out = if resuming && bytes > 0 {
            File::options()
                .append(true)
                .open(file)
                .map_err(|e| format!("打开文件失败: {}", e))?
        } else {
            if let Some(parent) = file.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            File::create(file).map_err(|e| format!("创建文件失败: {}", e))?
        };

        let mut reader = resp.into_reader();
        let mut buf = [0u8; 64 * 1024];
        let mut paused_out = false;
        let mut tick = Instant::now();
        let mut tick_bytes = bytes;
        loop {
            if cancel.load(Ordering::SeqCst) {
                drop(out);
                let _ = std::fs::remove_file(file);
                return Err("已取消".to_string());
            }
            if pause.load(Ordering::SeqCst) {
                paused_out = true;
                break; // 断开连接，保留已写入部分
            }
            let n = reader
                .read(&mut buf)
                .map_err(|e| format!("读取失败: {}", e))?;
            if n == 0 {
                out.flush().ok();
                return Ok((bytes, total.unwrap_or(bytes)));
            }
            out.write_all(&buf[..n])
                .map_err(|e| format!("写入失败: {}", e))?;
            bytes += n as u64;

            let elapsed = tick.elapsed();
            let report_speed = elapsed >= Duration::from_millis(500);
            let speed = if report_speed {
                let v = ((bytes - tick_bytes) as f64 / elapsed.as_secs_f64()) as u64;
                tick = Instant::now();
                tick_bytes = bytes;
                Some(v)
            } else {
                None
            };
            let mut t = me.tasks.lock().unwrap();
            if let Some(task) = t.iter_mut().find(|x| x.id == id) {
                task.bytes = bytes;
                task.total = total;
                if let Some(v) = speed {
                    task.speed = v;
                }
            }
            drop(t);
            // 进度事件驱动：每 500ms 推送一次外壳层刷新（不再依赖前端轮询）
            if report_speed {
                me.fire();
            }
        }

        if !paused_out {
            return Ok((bytes, total.unwrap_or(bytes)));
        }
        out.flush().ok();
        drop(out);
        {
            let mut t = me.tasks.lock().unwrap();
            if let Some(task) = t.iter_mut().find(|x| x.id == id) {
                task.speed = 0;
            }
        }
        me.maybe_persist(true); // 暂停点落盘，重启后仍可续传
        while pause.load(Ordering::SeqCst) && !cancel.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(200));
        }
    }
}

/// 同名文件加序号：`a.zip` → `a (1).zip`，不覆盖已存在的文件。
fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    if !p.exists() {
        return p;
    }
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    for i in 1..1000 {
        let cand = dir.join(format!("{} ({}){}", stem, i, ext));
        if !cand.exists() {
            return cand;
        }
    }
    p
}

/// Windows 保留设备名（不分大小写、忽略扩展名），用作文件名会创建失败。
const RESERVED_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// 文件名清洗：去掉路径分隔符与非法字符、避开保留名、限制长度。
fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    // 结尾的点和空格会被 Windows 静默去掉，先行处理
    let trimmed = cleaned.trim().trim_end_matches('.').trim().to_string();
    if trimmed.is_empty() {
        return "dsh-export.zip".to_string();
    }
    // 保留名：CON.txt 同样非法，只看主干
    let stem = trimmed.split('.').next().unwrap_or(&trimmed).to_ascii_uppercase();
    let trimmed = if RESERVED_NAMES.contains(&stem.as_str()) {
        format!("_{}", trimmed)
    } else {
        trimmed
    };
    // 路径整体 260 上限，给目录留足余量
    const MAX: usize = 120;
    if trimmed.chars().count() <= MAX {
        return trimmed;
    }
    let (stem, ext) = match trimmed.rfind('.') {
        Some(i) if i > 0 => (&trimmed[..i], &trimmed[i..]),
        _ => (trimmed.as_str(), ""),
    };
    let keep: String = stem.chars().take(MAX.saturating_sub(ext.chars().count())).collect();
    format!("{}{}", keep, ext)
}