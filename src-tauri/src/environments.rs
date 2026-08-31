//! 环境管理：dsh 版本（独立目录安装/更新/删除/切换）与 dsh profile
//! （扫描/新建空模板/复制/重命名/删除/切换）。
//!
//! 设计目标：
//! - 每个受管版本安装到 `%APPDATA%\DSH Desktop\versions\<id>\`，更新一个版本
//!   不会覆盖其它版本，也不影响当前正在运行的实例（下次重启才用新版本）。
//! - profile 以 `$DSH_HOME/profiles/<name>` 目录为准；支持新建空 web 模板
//!   （base+web-app，立即可启动）或复制现有 profile（可带 node_modules）来
//!   保留现场改造 / 修复出错环境。

use crate::settings::{save_settings, DshVersionEntry, Settings};
use crate::util::Logger;
use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

// ─────────────────────────── 对外数据结构 ───────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct VersionView {
    pub id: String,
    pub label: String,
    pub spec: String,
    pub installed: Option<String>,
    pub dir: String,
    pub bin: Option<String>,
    pub source: String,
    pub exists: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileView {
    pub name: String,
    pub dir: String,
    pub exists: bool,
    pub web_ready: bool,
    pub bundles: usize,
    pub plugins: usize,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnvTask {
    pub kind: String,
    pub phase: String,
    pub detail: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnvData {
    pub dsh_home: String,
    pub versions: Vec<VersionView>,
    pub profiles: Vec<ProfileView>,
    pub active_version_id: Option<String>,
    pub active_profile: String,
    pub busy: Option<EnvTask>,
    pub last_error: Option<String>,
}

// ─────────────────────────── 路径 ───────────────────────────

pub fn dsh_home() -> PathBuf {
    if let Ok(h) = std::env::var("DSH_HOME") {
        if !h.is_empty() {
            return PathBuf::from(h);
        }
    }
    std::env::var("USERPROFILE")
        .map(|h| PathBuf::from(h).join(".dsh"))
        .unwrap_or_else(|_| PathBuf::from("."))
}

pub fn profiles_dir() -> PathBuf {
    dsh_home().join("profiles")
}

pub fn versions_dir() -> PathBuf {
    crate::settings::settings_dir().join("versions")
}

fn gen_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("v{:x}", nanos)
}

fn dsh_pkg_dir(prefix: &Path) -> PathBuf {
    prefix
        .join("node_modules")
        .join("@deepseek-ai")
        .join("dsh")
}

fn dsh_bin_for(prefix: &Path) -> PathBuf {
    dsh_pkg_dir(prefix).join("lib").join("bin.js")
}

pub fn read_actual_version(prefix: &Path) -> Option<String> {
    read_pkg_version(&dsh_pkg_dir(prefix))
}

/// 读取某个 dsh 包目录（`.../@deepseek-ai/dsh`）下的实际版本。
fn read_pkg_version(pkg_dir: &Path) -> Option<String> {
    let pkg = pkg_dir.join("package.json");
    let text = std::fs::read_to_string(pkg).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    v.get("version")?.as_str().map(|s| s.to_string())
}

/// 由 bin.js 路径推导 dsh 包目录（`.../@deepseek-ai/dsh`）并读版本。
fn version_of_bin(bin: &str) -> Option<String> {
    Path::new(bin)
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| read_pkg_version(p))
}

/// 按路径判断版本类别：受管（桌面端 versions 目录）/ 全局（npm node_modules）/ 手动。
fn classify_bin(bin: &str) -> &'static str {
    let p = Path::new(bin);
    if p.starts_with(versions_dir()) {
        return "managed";
    }
    let lower = bin.to_lowercase();
    if lower.contains("node_modules")
        && lower.contains("@deepseek-ai")
        && lower.contains("dsh")
        && lower.contains("lib")
        && lower.contains("bin.js")
    {
        return "global";
    }
    "manual"
}

fn validate_version_name(label: &str) -> bool {
    !label.trim().is_empty()
}

pub fn validate_profile_name(name: &str) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("profile 名称不能为空".to_string());
    }
    if name == "." || name == ".." || name == "node_modules" {
        return Err(format!("非法 profile 名称: {}", name));
    }
    if name.contains('/') || name.contains('\\') {
        return Err("profile 名称不能包含路径分隔符".to_string());
    }
    Ok(())
}

// ─────────────────────────── 启动时登记现有版本 ───────────────────────────

/// 首次启动 / 迁移：把当前 settings.dsh_bin（手动路径）或 PATH 上的全局 dsh
/// 登记为一个版本条目，保证环境面板能显示并高亮当前版本。
pub fn ensure_seed_versions(settings: &mut Settings) {
    let current_bin = settings.dsh_bin.clone().filter(|b| !b.is_empty());
    if let Some(bin) = current_bin {
        // 已有条目：按真实路径重新归类（全局/受管/手动），并补全实际版本号
        if let Some(pos) = settings
            .dsh_versions
            .iter()
            .position(|v| v.bin.as_deref() == Some(bin.as_str()))
        {
            let source = classify_bin(&bin);
            let installed = version_of_bin(&bin).or_else(|| settings.dsh_versions[pos].installed.clone());
            let e = &mut settings.dsh_versions[pos];
            e.source = source.to_string();
            e.installed = installed.clone();
            if source == "global" {
                e.spec = "global".to_string();
                e.label = format!("全局 dsh{}", installed.as_deref().map(|v| format!(" ({v})")).unwrap_or_default());
            }
            if settings.active_version_id.is_none() {
                settings.active_version_id = Some(e.id.clone());
            }
            return;
        }
        let prefix = Path::new(&bin)
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let installed = version_of_bin(&bin);
        let source = classify_bin(&bin);
        let label = if source == "global" {
            format!("全局 dsh{}", installed.as_deref().map(|v| format!(" ({v})")).unwrap_or_default())
        } else {
            format!("当前 dsh{}", installed.as_deref().map(|v| format!(" ({v})")).unwrap_or_default())
        };
        let id = gen_id();
        settings.dsh_versions.push(DshVersionEntry {
            id: id.clone(),
            label,
            spec: source.to_string(),
            installed,
            dir: prefix,
            bin: Some(bin),
            source: source.to_string(),
        });
        settings.active_version_id = Some(id);
        return;
    }
    // 没有手动路径：找全局安装（与后端 resolve_dsh 相同的兜底逻辑）
    let mut found: Option<String> = None;
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            let p = dir
                .join("node_modules")
                .join("@deepseek-ai")
                .join("dsh")
                .join("lib")
                .join("bin.js");
            if p.exists() {
                found = Some(p.to_string_lossy().to_string());
                break;
            }
        }
    }
    if found.is_none() {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let p = PathBuf::from(&appdata)
                .join("npm")
                .join("node_modules")
                .join("@deepseek-ai")
                .join("dsh")
                .join("lib")
                .join("bin.js");
            if p.exists() {
                found = Some(p.to_string_lossy().to_string());
            }
        }
    }
    if let Some(bin) = found {
        let known = settings
            .dsh_versions
            .iter()
            .any(|v| v.bin.as_deref() == Some(bin.as_str()));
        if known {
            return;
        }
        let prefix = PathBuf::from(&bin)
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .unwrap_or_default();
        let installed = version_of_bin(&bin);
        let id = gen_id();
        let label = installed
            .as_deref()
            .map(|v| format!("全局 dsh ({v})"))
            .unwrap_or_else(|| "全局 dsh".to_string());
        settings.dsh_versions.push(DshVersionEntry {
            id: id.clone(),
            label,
            spec: "global".to_string(),
            installed,
            dir: prefix.to_string_lossy().to_string(),
            bin: Some(bin.clone()),
            source: "global".to_string(),
        });
        settings.dsh_bin = Some(bin);
        settings.active_version_id = Some(id);
    }
}

// ─────────────────────────── 列表 / 视图 ───────────────────────────

fn list_versions(s: &Settings) -> Vec<VersionView> {
    let mut out: Vec<VersionView> = s
        .dsh_versions
        .iter()
        .map(|v| {
            let exists = v
                .bin
                .as_deref()
                .map(|b| Path::new(b).exists())
                .unwrap_or(false);
            let active = s.active_version_id.as_deref() == Some(v.id.as_str())
                || s.dsh_bin.as_deref() == v.bin.as_deref();
            VersionView {
                id: v.id.clone(),
                label: v.label.clone(),
                spec: v.spec.clone(),
                installed: v.installed.clone(),
                dir: v.dir.clone(),
                bin: v.bin.clone(),
                source: v.source.clone(),
                exists,
                active,
            }
        })
        .collect();
    // 当前 dsh_bin 未在任何条目中出现（例如用户在别处手动改过 settings）
    // 时补一条只读“当前路径”，保证面板始终能显示高亮。
    if let Some(bin) = s.dsh_bin.clone().filter(|b| !b.is_empty()) {
        let hit = out.iter().any(|v| v.bin.as_deref() == Some(bin.as_str()));
        if !hit {
            let exists = Path::new(&bin).exists();
            let installed = version_of_bin(&bin);
            let source = classify_bin(&bin);
            let label = if source == "global" {
                format!("全局 dsh{}", installed.as_deref().map(|v| format!(" ({v})")).unwrap_or_default())
            } else {
                format!("当前 dsh{}", installed.as_deref().map(|v| format!(" ({v})")).unwrap_or_default())
            };
            out.push(VersionView {
                id: "__current__".to_string(),
                label,
                spec: source.to_string(),
                installed,
                dir: String::new(),
                bin: Some(bin),
                source: source.to_string(),
                exists,
                active: true,
            });
        }
    }
    // 完全没有条目：尝试登记全局安装（面板至少能看到一项可切换）
    if out.is_empty() {
        if let Some(bin) = find_global_bin() {
            let exists = true;
            let installed = version_of_bin(&bin);
            out.push(VersionView {
                id: "__global__".to_string(),
                label: format!("全局 dsh{}", installed.as_deref().map(|v| format!(" ({v})")).unwrap_or_default()),
                spec: "global".to_string(),
                installed,
                dir: String::new(),
                bin: Some(bin),
                source: "global".to_string(),
                exists,
                active: false,
            });
        }
    }
    out
}

fn find_global_bin() -> Option<String> {
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            let p = dir
                .join("node_modules")
                .join("@deepseek-ai")
                .join("dsh")
                .join("lib")
                .join("bin.js");
            if p.exists() {
                return Some(p.to_string_lossy().to_string());
            }
        }
    }
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

fn list_profiles(active: &str) -> Vec<ProfileView> {
    let dir = profiles_dir();
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(&dir) {
        Ok(it) => it.flatten().collect::<Vec<_>>(),
        Err(_) => return out,
    };
    for e in entries {
        let path = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if name == "node_modules" || !path.is_dir() {
            continue;
        }
        let (bundles, plugins) = read_profile_manifest(&path);
        let web_ready = bundles.iter().any(|b| b == "@deepseek-ai/dsh-web-app");
        out.push(ProfileView {
            name: name.clone(),
            dir: path.to_string_lossy().to_string(),
            exists: true,
            web_ready,
            bundles: bundles.len(),
            plugins,
            active: name == active,
        });
    }
    out.sort_by(|a, b| {
        a.active.cmp(&b.active).reverse().then_with(|| a.name.cmp(&b.name))
    });
    out
}

fn read_profile_manifest(dir: &Path) -> (Vec<String>, usize) {
    let pkg = dir.join("package.json");
    let mut bundles = Vec::new();
    let mut plugins = 0usize;
    if let Ok(text) = std::fs::read_to_string(pkg) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(arr) = v.pointer("/dsh/profile/bundles").and_then(|x| x.as_array()) {
                bundles = arr
                    .iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect();
            }
            if let Some(deps) = v.get("dependencies").and_then(|x| x.as_object()) {
                plugins = deps.len();
            }
        }
    }
    (bundles, plugins)
}

pub fn env_view(settings: &Mutex<Settings>) -> EnvData {
    let s = settings.lock().unwrap();
    EnvData {
        dsh_home: dsh_home().to_string_lossy().to_string(),
        versions: list_versions(&s),
        profiles: list_profiles(&s.profile),
        active_version_id: s.active_version_id.clone(),
        active_profile: s.profile.clone(),
        busy: None,
        last_error: None,
    }
}

// ─────────────────────────── 版本 CRUD ───────────────────────────

pub fn activate_version(settings: &Mutex<Settings>, id: &str) -> Result<(), String> {
    let mut s = settings.lock().unwrap();
    // 面板里可能出现的伪条目（无持久记录时的当前/全局路径）
    if id == "__global__" || id == "__current__" {
        let bin = if let Some(b) = s.dsh_bin.clone().filter(|b| !b.is_empty()) {
            b
        } else {
            find_global_bin().ok_or_else(|| "未找到全局 dsh".to_string())?
        };
        if !Path::new(&bin).exists() {
            return Err("当前 dsh 路径不存在".to_string());
        }
        s.dsh_bin = Some(bin);
        s.active_version_id = None;
        save_settings(&s);
        return Ok(());
    }
    let entry = s
        .dsh_versions
        .iter()
        .find(|v| v.id == id)
        .ok_or_else(|| "版本不存在".to_string())?;
    let bin = entry
        .bin
        .clone()
        .ok_or_else(|| "该版本缺少 bin.js 路径".to_string())?;
    let entry_id = entry.id.clone();
    if !Path::new(&bin).exists() {
        return Err(format!("版本目录不存在或未安装完整: {}", entry.dir));
    }
    s.dsh_bin = Some(bin);
    s.active_version_id = Some(entry_id);
    save_settings(&s);
    Ok(())
}

pub fn rename_version(settings: &Mutex<Settings>, id: &str, label: &str) -> Result<(), String> {
    if !validate_version_name(label) {
        return Err("备注名不能为空".to_string());
    }
    let mut s = settings.lock().unwrap();
    let pos = s
        .dsh_versions
        .iter()
        .position(|v| v.id == id)
        .ok_or_else(|| "版本不存在".to_string())?;
    if s.active_version_id.as_deref() == Some(id) || s.dsh_bin.as_deref() == s.dsh_versions[pos].bin.as_deref() {
        return Err("当前使用中的版本不能重命名，请先切换到其他版本".to_string());
    }
    let entry = &mut s.dsh_versions[pos];
    entry.label = label.trim().to_string();
    save_settings(&s);
    Ok(())
}

pub fn remove_version(settings: &Mutex<Settings>, log: &Arc<Logger>, id: &str) -> Result<(), String> {
    let mut s = settings.lock().unwrap();
    let pos = s
        .dsh_versions
        .iter()
        .position(|v| v.id == id)
        .ok_or_else(|| "版本不存在".to_string())?;
    let entry = s.dsh_versions[pos].clone();
    if entry.source == "global" || entry.source == "manual" {
        return Err("系统/手动路径版本不能删除，只能删除桌面端管理的版本".to_string());
    }
    if s.active_version_id.as_deref() == Some(id) || s.dsh_bin.as_deref() == entry.bin.as_deref() {
        return Err("当前使用中的版本不能删除，请先切换到其他版本".to_string());
    }
    s.dsh_versions.remove(pos);
    // 删除的是当前激活版本：把激活指向第一个仍然存在的版本
    if s.active_version_id.as_deref() == Some(id) {
        let next = s
            .dsh_versions
            .iter()
            .find(|v| {
                v.bin
                    .as_deref()
                    .map(|b| Path::new(b).exists())
                    .unwrap_or(false)
            })
            .map(|v| (v.id.clone(), v.bin.clone()));
        if let Some((next_id, next_bin)) = next {
            s.active_version_id = Some(next_id);
            s.dsh_bin = next_bin;
        } else {
            s.active_version_id = None;
            s.dsh_bin = None;
        }
    }
    let dir = PathBuf::from(&entry.dir);
    // 只允许删除桌面端管理目录下的内容；用户手动填写的目录不碰。
    let managed_root = versions_dir();
    if dir.starts_with(&managed_root) && dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(&dir) {
            log.warn(&format!("[env] 删除版本目录失败 {}: {}", dir.display(), e));
        }
    }
    save_settings(&s);
    Ok(())
}

pub fn update_version(
    settings: &Arc<Mutex<Settings>>,
    log: &Arc<Logger>,
    id: &str,
    spec: Option<&str>,
    label: Option<&str>,
    report: &dyn Fn(&str, &str, u32),
) -> Result<DshVersionEntry, String> {
    let s = settings.lock().unwrap();
    let pos = s
        .dsh_versions
        .iter()
        .position(|v| v.id == id)
        .ok_or_else(|| "版本不存在".to_string())?;
    let entry = s.dsh_versions[pos].clone();
    if entry.source == "global" || entry.source == "manual" {
        return Err("系统/手动路径版本不能由桌面端更新，请使用 npm 或直接切换".to_string());
    }
    let new_spec = spec
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .unwrap_or_else(|| entry.spec.clone());
    let new_label = label
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .unwrap_or_else(|| entry.label.clone());
    let node_bin = resolve_node(&s);
    drop(s);
    let prefix = PathBuf::from(&entry.dir);
    std::fs::create_dir_all(&prefix).map_err(|e| format!("创建版本目录失败: {}", e))?;
    let actual = install_dsh_to_prefix(&node_bin, &new_spec, &prefix, log, report)?;
    let bin = dsh_bin_for(&prefix);
    let mut s = settings.lock().unwrap();
    let entry = s.dsh_versions.get_mut(pos).ok_or_else(|| "版本不存在".to_string())?;
    entry.spec = new_spec;
    entry.installed = Some(actual.clone());
    entry.label = new_label;
    entry.bin = Some(bin.to_string_lossy().to_string());
    let out = entry.clone();
    save_settings(&s);
    Ok(out)
}

/// 新增一个受管版本：安装到独立目录，返回新条目（不自动切换）。
pub fn add_version(
    settings: &Arc<Mutex<Settings>>,
    log: &Arc<Logger>,
    label: Option<&str>,
    spec: &str,
    report: &dyn Fn(&str, &str, u32),
) -> Result<DshVersionEntry, String> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Err("请填写 dsh 版本（如 latest 或 0.1.2-alpha.2）".to_string());
    }
    let node_bin = resolve_node(&settings.lock().unwrap());
    let id = gen_id();
    let prefix = versions_dir().join(&id);
    std::fs::create_dir_all(&prefix).map_err(|e| format!("创建版本目录失败: {}", e))?;
    let actual = install_dsh_to_prefix(&node_bin, spec, &prefix, log, report)?;
    let bin = dsh_bin_for(&prefix);
    let entry = DshVersionEntry {
        id: id.clone(),
        label: label
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .unwrap_or_else(|| format!("dsh {actual}")),
        spec: spec.to_string(),
        installed: Some(actual),
        dir: prefix.to_string_lossy().to_string(),
        bin: Some(bin.to_string_lossy().to_string()),
        source: "managed".to_string(),
    };
    let mut s = settings.lock().unwrap();
    s.dsh_versions.push(entry.clone());
    save_settings(&s);
    log.info(&format!("[env] 新增版本 {} -> {} ({})", entry.label, entry.dir, entry.spec));
    Ok(entry)
}

/// 找到第一个 bin.js 存在的受管/登记版本（启动兜底用）。
pub fn find_available_version(settings: &Mutex<Settings>) -> Option<DshVersionEntry> {
    let s = settings.lock().unwrap();
    s.dsh_versions
        .iter()
        .find(|v| {
            v.bin
                .as_deref()
                .map(|b| Path::new(b).exists())
                .unwrap_or(false)
        })
        .cloned()
}

// ─────────────────────────── Profile CRUD ───────────────────────────

fn profile_dir(name: &str) -> Result<PathBuf, String> {
    validate_profile_name(name)?;
    Ok(profiles_dir().join(name.trim()))
}

fn write_profile_init(dir: &Path, name: &str) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("创建 profile 目录失败: {}", e))?;
    let manifest = serde_json::json!({
        "name": format!("dsh-profile-{}", name),
        "private": true,
        "dependencies": {},
        "dsh": { "profile": {
            "bundles": ["@deepseek-ai/dsh-base", "@deepseek-ai/dsh-web-app"],
            "patchReload": "live"
        } }
    });
    std::fs::write(
        dir.join("package.json"),
        serde_json::to_string_pretty(&manifest).unwrap_or_default(),
    )
    .map_err(|e| format!("写入 package.json 失败: {}", e))?;
    let patch = format!(
        "# Your patch layer for this dsh profile, applied after every bundle layer:\n# a top-level YAML array of loader patch entries.\n[]\n"
    );
    std::fs::write(dir.join("cordis.patch.yml"), patch)
        .map_err(|e| format!("写入 cordis.patch.yml 失败: {}", e))?;
    std::fs::write(
        dir.join("pnpm-workspace.yaml"),
        "packages:\n  - .\n\nnodeLinker: hoisted\nautoInstallPeers: false\n",
    )
    .map_err(|e| format!("写入 pnpm-workspace.yaml 失败: {}", e))?;
    Ok(())
}

pub fn add_empty_profile(settings: &Mutex<Settings>, name: &str) -> Result<ProfileView, String> {
    let name = name.trim();
    let dir = profile_dir(name)?;
    if dir.exists() {
        return Err(format!("profile 已存在: {}", name));
    }
    write_profile_init(&dir, name)?;
    let (bundles, plugins) = read_profile_manifest(&dir);
    let active = settings.lock().unwrap().profile == name;
    Ok(ProfileView {
        name: name.to_string(),
        dir: dir.to_string_lossy().to_string(),
        exists: true,
        web_ready: bundles.iter().any(|b| b == "@deepseek-ai/dsh-web-app"),
        bundles: bundles.len(),
        plugins,
        active,
    })
}

pub fn copy_profile(
    settings: &Mutex<Settings>,
    source: &str,
    name: &str,
    include_modules: bool,
) -> Result<ProfileView, String> {
    let name = name.trim();
    let src = profile_dir(source)?;
    let dst = profile_dir(name)?;
    if !src.exists() {
        return Err(format!("源 profile 不存在: {}", source));
    }
    if dst.exists() {
        return Err(format!("profile 已存在: {}", name));
    }
    if src == dst {
        return Err("源与目标同名".to_string());
    }
    copy_dir_recursive(&src, &dst, include_modules)
        .map_err(|e| format!("复制 profile 失败: {}", e))?;
    // 新 profile 的 manifest 名称纠正为自身名字（避免多个 profile 同名）
    let manifest_path = dst.join("package.json");
    if let Ok(text) = std::fs::read_to_string(&manifest_path) {
        if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(obj) = v.as_object_mut() {
                obj.insert(
                    "name".to_string(),
                    serde_json::json!(format!("dsh-profile-{}", name)),
                );
            }
            let _ = std::fs::write(
                &manifest_path,
                serde_json::to_string_pretty(&v).unwrap_or_default(),
            );
        }
    }
    let (bundles, plugins) = read_profile_manifest(&dst);
    let active = settings.lock().unwrap().profile == name;
    Ok(ProfileView {
        name: name.to_string(),
        dir: dst.to_string_lossy().to_string(),
        exists: true,
        web_ready: bundles.iter().any(|b| b == "@deepseek-ai/dsh-web-app"),
        bundles: bundles.len(),
        plugins,
        active,
    })
}

fn copy_dir_recursive(src: &Path, dst: &Path, include_modules: bool) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        if !include_modules && name == "node_modules" {
            continue;
        }
        let from = entry.path();
        let to = dst.join(&name);
        let ft = entry.file_type()?;
        if ft.is_symlink() {
            let target = std::fs::read_link(&from)?;
            #[cfg(windows)]
            {
                if from.is_dir() {
                    std::os::windows::fs::symlink_dir(target, &to)?;
                } else {
                    std::os::windows::fs::symlink_file(target, &to)?;
                }
            }
            #[cfg(not(windows))]
            {
                std::os::unix::fs::symlink(target, &to)?;
            }
        } else if ft.is_dir() {
            copy_dir_recursive(&from, &to, include_modules)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

pub fn rename_profile(settings: &Mutex<Settings>, old: &str, name: &str) -> Result<(), String> {
    let name = name.trim();
    let src = profile_dir(old)?;
    let dst = profile_dir(name)?;
    if !src.exists() {
        return Err(format!("profile 不存在: {}", old));
    }
    if dst.exists() {
        return Err(format!("profile 已存在: {}", name));
    }
    {
        let s = settings.lock().unwrap();
        if s.profile == old {
            return Err("当前正在使用的 profile 不能重命名；请先切换到别的 profile".to_string());
        }
    }
    std::fs::rename(&src, &dst).map_err(|e| format!("重命名失败: {}", e))?;
    Ok(())
}

pub fn remove_profile(settings: &Mutex<Settings>, log: &Arc<Logger>, name: &str) -> Result<(), String> {
    let dir = profile_dir(name)?;
    let active = {
        let s = settings.lock().unwrap();
        s.profile == name
    };
    if active {
        return Err("当前正在使用的 profile 不能删除；请先切换到别的 profile".to_string());
    }
    if !dir.exists() {
        return Err(format!("profile 不存在: {}", name));
    }
    let root = profiles_dir();
    if !dir.starts_with(&root) {
        return Err("拒绝删除非 profiles 目录".to_string());
    }
    std::fs::remove_dir_all(&dir).map_err(|e| format!("删除失败: {}", e))?;
    log.info(&format!("[env] 删除 profile: {}", name));
    Ok(())
}

pub fn activate_profile(settings: &Mutex<Settings>, name: &str) -> Result<(), String> {
    let name = name.trim();
    validate_profile_name(name)?;
    let dir = profile_dir(name)?;
    if !dir.exists() {
        return Err("profile 不存在或未初始化".to_string());
    }
    let mut s = settings.lock().unwrap();
    s.profile = name.to_string();
    save_settings(&s);
    Ok(())
}

// ─────────────────────────── npm 安装到独立目录 ───────────────────────────

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

fn pick_registry() -> Result<(&'static str, &'static str), String> {
    let mut scored: Vec<(u64, &str, &str)> = Vec::new();
    for (name, url) in REGISTRY_CANDIDATES {
        if let Some(ms) = registry_latency(url) {
            scored.push((ms, name, url));
        }
    }
    scored.sort_by_key(|(ms, _, _)| *ms);
    scored
        .first()
        .map(|(_, n, u)| (*n, *u))
        .ok_or_else(|| "所有 npm 源均无法连接".to_string())
}

fn resolve_node(s: &Settings) -> String {
    if let Some(p) = s.node_bin.as_deref() {
        if Path::new(p).exists() {
            return p.to_string();
        }
    }
    let cands = [
        "C:\\Program Files\\nodejs\\node.exe",
        "C:\\Program Files (x86)\\nodejs\\node.exe",
    ];
    for c in cands {
        if Path::new(c).exists() {
            return c.to_string();
        }
    }
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

fn npm_cli_for(node_bin: &str) -> Result<PathBuf, String> {
    PathBuf::from(node_bin)
        .parent()
        .map(|d| {
            d.join("node_modules")
                .join("npm")
                .join("bin")
                .join("npm-cli.js")
        })
        .filter(|p| p.exists())
        .ok_or_else(|| "无法定位 npm（npm-cli.js）".to_string())
}

#[allow(clippy::too_many_arguments)]
fn install_dsh_to_prefix(
    node_bin: &str,
    spec: &str,
    prefix: &Path,
    log: &Arc<Logger>,
    report: &dyn Fn(&str, &str, u32),
) -> Result<String, String> {
    // 空目录里安装前先放一个私有 package.json，避免 npm 行为差异
    let pkg = prefix.join("package.json");
    if !pkg.exists() {
        std::fs::write(
            &pkg,
            "{\n  \"name\": \"dsh-desktop-runtime\",\n  \"private\": true,\n  \"version\": \"0.0.0\"\n}\n",
        )
        .map_err(|e| format!("初始化版本目录失败: {}", e))?;
    }
    let npm_cli = npm_cli_for(node_bin)?;
    let (name, registry) = pick_registry()?;
    report(
        "installing",
        &format!("正在从 {} 安装 dsh {}…", name, spec),
        0,
    );
    log.info(&format!(
        "[env] npm install --prefix {} @deepseek-ai/dsh@{} (reg {})",
        prefix.display(),
        spec,
        registry
    ));

    let mut child = Command::new(node_bin)
        .arg(npm_cli.to_str().unwrap_or(""))
        .args([
            "install",
            "--prefix",
            prefix.to_str().unwrap_or(""),
            &format!("@deepseek-ai/dsh@{}", spec),
            "--registry",
            registry,
            "--loglevel",
            "info",
            "--no-fund",
            "--no-audit",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn()
        .map_err(|e| format!("npm 安装 dsh 失败: {}", e))?;

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
    let bin = dsh_bin_for(prefix);
    if !bin.exists() {
        return Err(format!("安装完成但未找到 dsh 入口: {}", bin.display()));
    }
    let actual = read_actual_version(prefix).unwrap_or_else(|| spec.to_string());
    log.info(&format!("[env] dsh {} 安装完成：{}", actual, bin.display()));
    Ok(actual)
}
