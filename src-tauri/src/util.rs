//! 通用工具：文件日志、netstat PID 查询、简单 HTTP 探测、URL 解析。
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub struct Logger {
    path: PathBuf,
    lock: Mutex<()>,
}

impl Logger {
    pub fn new(path: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(path.parent().unwrap_or(&PathBuf::from(".")));
        Self {
            path,
            lock: Mutex::new(()),
        }
    }

    pub fn info(&self, msg: &str) {
        self.write("INFO", msg);
    }

    #[allow(dead_code)]
    pub fn warn(&self, msg: &str) {
        self.write("WARN", msg);
    }

    #[allow(dead_code)]
    pub fn error(&self, msg: &str) {
        self.write("ERROR", msg);
    }

    fn write(&self, level: &str, msg: &str) {
        let _g = self.lock.lock();
        let ts = chrono_like_now();
        let line = format!("[{}] {} {}\n", ts, level, msg);
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = f.write_all(line.as_bytes());
        }
    }
}

/// 无依赖的最小时间格式：用系统时间 + 手工转换（避免引入 chrono）。
fn chrono_like_now() -> String {
    // 简单方案：Unix 秒 -> UTC 日期
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();
    format_utc(secs as i64)
}

pub fn format_utc(secs: i64) -> String {
    let days = secs.div_euclid(86400);
    let tod = secs.rem_euclid(86400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        y,
        m,
        d,
        tod / 3600,
        (tod % 3600) / 60,
        tod % 60
    )
}

/// Howard Hinnant 的 civil_from_days 算法。
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// 通过 netstat -ano 找到监听 127.0.0.1:port 的 PID。
pub fn find_pid_by_port(port: u16) -> Option<u32> {
    let out = std::process::Command::new("netstat.exe")
        .arg("-ano")
        .creation_flags(0x08000000) // CREATE_NO_WINDOW：避免弹出控制台窗口
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let wanted = format!("127.0.0.1:{}", port);
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 5 && parts[0] == "TCP" && parts[1] == wanted && parts[3] == "LISTENING" {
            if let Ok(pid) = parts[4].parse::<u32>() {
                return Some(pid);
            }
        }
    }
    None
}

/// 探测端口：向 / 发 GET，返回 (是否有响应, 是否 dsh web 签名)。
pub fn probe_port(port: u16, timeout_ms: u64) -> (bool, bool) {
    let addr = match ("127.0.0.1", port).to_socket_addrs() {
        Ok(mut it) => match it.next() {
            Some(a) => a,
            None => return (false, false),
        },
        Err(_) => return (false, false),
    };
    let mut stream = match TcpStream::connect_timeout(&addr, Duration::from_millis(timeout_ms)) {
        Ok(s) => s,
        Err(_) => return (false, false),
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(timeout_ms)));
    let req = format!(
        "GET / HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
        port
    );
    if stream.write_all(req.as_bytes()).is_err() {
        return (false, false);
    }
    let mut buf = String::new();
    let mut chunk = [0u8; 8192];
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > Duration::from_millis(timeout_ms) {
            break;
        }
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                buf.push_str(&String::from_utf8_lossy(&chunk[..n]));
                if buf.len() > 300_000 {
                    break;
                }
            }
        }
    }
    let ok = buf.to_ascii_lowercase().contains("200 ok")
        || buf.contains("<!doctype")
        || buf.contains("<html")
        || buf.contains("__DSH_BOOT__");
    (ok, buf.contains("__DSH_BOOT__"))
}

/// taskkill 杀进程树（Windows）。
#[allow(dead_code)]
pub fn kill_tree(pid: u32) {
    let _ = std::process::Command::new("taskkill.exe")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output();
}

/// 简单 HTTP GET 文本（用于探测，不走 TLS）。
#[allow(dead_code)]
pub fn http_get(port: u16, path: &str, timeout_ms: u64) -> Option<String> {
    let addr = ("127.0.0.1", port)
        .to_socket_addrs()
        .ok()?
        .next()?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(timeout_ms)).ok()?;
    let _ = stream.set_read_timeout(Some(Duration::from_millis(timeout_ms)));
    let req = format!(
        "GET {} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
        path, port
    );
    stream.write_all(req.as_bytes()).ok()?;
    let mut buf = String::new();
    let mut chunk = [0u8; 8192];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => buf.push_str(&String::from_utf8_lossy(&chunk[..n])),
        }
    }
    Some(buf)
}

/// 从 dsh stdout 行解析 `dsh web: http://...`。
pub fn parse_url_line(line: &str) -> Option<String> {
    let idx = line.find("dsh web:")?;
    let rest = line[idx + "dsh web:".len()..].trim();
    let url: String = rest
        .split_whitespace()
        .find(|s| s.starts_with("http://") || s.starts_with("https://"))?
        .trim_end_matches(|c: char| c == ',' || c == ';' || c == ')')
        .to_string();
    Some(url)
}
