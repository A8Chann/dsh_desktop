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
    /// 文件是否仍在磁盘（运行时计算，不入持久化）
    #[serde(default)]
    pub exists: bool,
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
            exists: false,
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
}

fn persist_path() -> PathBuf {
    crate::settings::settings_dir().join("downloads.json")
}

impl Downloads {
    pub fn new(log: Arc<Logger>, notify: Arc<dyn Fn() + Send + Sync>) -> Self {
        let mut tasks: Vec<DownloadTask> = std::fs::read_to_string(persist_path())
            .ok()
            .and_then(|text| serde_json::from_str::<Vec<DownloadTask>>(&text).ok())
            .unwrap_or_default();
        // 重启后遗留的“下载中/暂停”任务标记为中断
        for t in tasks.iter_mut() {
            if t.state == "downloading" || t.state == "paused" {
                t.state = "error".to_string();
                if t.error.is_none() {
                    t.error = Some("程序重启，下载中断".to_string());
                }
            }
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
        let id = self.inner.next_id.fetch_add(1, Ordering::SeqCst);
        let file = dir.join(sanitize(name));
        let cancel = Arc::new(AtomicBool::new(false));
        let pause = Arc::new(AtomicBool::new(false));
        self.inner.flags.lock().unwrap().insert(id, (cancel.clone(), pause.clone()));
        {
            let mut t = self.inner.tasks.lock().unwrap();
            t.insert(
                0,
                DownloadTask {
                    id,
                    name: name.to_string(),
                    url: url.to_string(),
                    file: file.to_string_lossy().to_string(),
                    state: "downloading".to_string(),
                    bytes: 0,
                    total: None,
                    error: None,
                    exists: false,
                },
            );
        }
        self.maybe_persist(true);
        self.log.info(&format!("[download] start id={} url={} -> {}", id, url, file.display()));
        self.inner.fire();

        let me = self.inner.clone();
        let log = self.log.clone();
        let url = url.to_string();
        std::thread::spawn(move || {
            // catch_unwind：下载线程任何 panic 都不允许拖垮整个进程（记录为失败任务）
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run_download(me.clone(), id, &url, &file, cancel.clone(), pause.clone())
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
            {
                let mut t = me.tasks.lock().unwrap();
                if let Some(task) = t.iter_mut().find(|x| x.id == id) {
                    task.state = state.to_string();
                    task.error = err.clone();
                    if state == "done" {
                        task.bytes = bytes;
                        task.total = total;
                    }
                }
            }
            me.flags.lock().unwrap().remove(&id);
            me.maybe_persist(true); // 持久化最终状态
            me.fire(); // 通知外壳层即时刷新（徽标/面板）
            log.info(&format!(
                "[download] finished id={} state={} err={:?}",
                id, state, err
            ));
        });
        Some(id)
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

    pub fn pause(&self, id: u64, on: bool) -> bool {
        let found = {
            let f = self.inner.flags.lock().unwrap();
            f.get(&id)
                .map(|(_, p)| {
                    p.store(on, Ordering::SeqCst);
                    let mut t = self.inner.tasks.lock().unwrap();
                    if let Some(task) = t.iter_mut().find(|x| x.id == id) {
                        task.state = if on { "paused".to_string() } else { "downloading".to_string() };
                    }
                })
                .is_some()
        };
        if found {
            self.log.info(&format!("[download] pause(id={})={}", id, on));
            self.maybe_persist(false);
            self.inner.fire();
        }
        found
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
    pub fn delete(&self, id: u64) -> bool {
        let (removed_file, active) = {
            let mut t = self.inner.tasks.lock().unwrap();
            let Some(idx) = t.iter().position(|x| x.id == id) else {
                return false;
            };
            let task = t.remove(idx);
            (task.file, task.state == "downloading" || task.state == "paused")
        };
        if active {
            // 通知下载线程停止（线程结束时会 remove_file）
            if let Some((c, _)) = self.inner.flags.lock().unwrap().get(&id) {
                c.store(true, Ordering::SeqCst);
            }
        } else {
            let _ = std::fs::remove_file(&removed_file);
        }
        self.maybe_persist(true);
        self.log.info(&format!("[download] delete id={} file={}", id, removed_file));
        self.inner.fire();
        true
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

    /// 重试（重新发起下载，生成新任务）；返回新任务 id。
    pub fn retry(&self, id: u64) -> Option<u64> {
        let t = self.inner.tasks.lock().unwrap();
        let task = t.iter().find(|x| x.id == id)?;
        let url = task.url.clone();
        let name = task.name.clone();
        let dir = std::path::Path::new(&task.file)
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| dirs::download_dir().unwrap_or_else(std::env::temp_dir));
        drop(t);
        self.log.info(&format!("[download] retry id={} url={}", id, url));
        self.start(&url, &name, dir)
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

/// 下载线程主体：HEAD 拿总长 → GET 流式写盘；逐块检查 cancel/pause。
fn run_download(
    me: Arc<Inner>,
    id: u64,
    url: &str,
    file: &PathBuf,
    cancel: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,
) -> Result<(u64, u64), String> {
    // 1) HEAD 探测（在闭包内把 &str 拷贝为 String，避免借用逃逸）
    let total = ureq::head(url)
        .timeout(Duration::from_secs(15))
        .call()
        .ok()
        .and_then(|r| r.header("content-length").map(|s| s.trim().to_string()))
        .and_then(|s| s.parse::<u64>().ok());

    // 2) GET 流式下载
    let resp = ureq::get(url)
        .timeout(Duration::from_secs(60))
        .call()
        .map_err(|e| format!("请求失败: {}", e))?;
    let mut reader = resp.into_reader();
    let mut out = File::create(file).map_err(|e| format!("创建文件失败: {}", e))?;
    let mut buf = [0u8; 64 * 1024];
    let mut bytes: u64 = 0;
    loop {
        if cancel.load(Ordering::SeqCst) {
            drop(out);
            let _ = std::fs::remove_file(file);
            return Err("已取消".to_string());
        }
        if pause.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(150));
            continue;
        }
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("读取失败: {}", e))?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])
            .map_err(|e| format!("写入失败: {}", e))?;
        bytes += n as u64;
        let mut t = me.tasks.lock().unwrap();
        if let Some(task) = t.iter_mut().find(|x| x.id == id) {
            task.bytes = bytes;
            task.total = total;
        }
    }
    out.flush().ok();
    Ok((bytes, total.unwrap_or(bytes)))
}

/// 文件名清洗：去掉路径分隔符与非法字符
fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "dsh-export.zip".to_string()
    } else {
        trimmed.to_string()
    }
}