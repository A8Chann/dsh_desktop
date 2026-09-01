fn main() {
    // 编译期嵌入 git 信息（「关于」弹窗用）：发布版 exe 在用户机器上无 git 仓库，
    // 必须在构建时固化。取 7 位 hash 与工作区是否脏。
    let commit = git_output(&["rev-parse", "--short", "HEAD"])
        .unwrap_or_else(|| "unknown".to_string());
    let dirty = git_output(&["status", "--porcelain"])
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    println!("cargo:rustc-env=DST_GIT_COMMIT={}", commit);
    println!("cargo:rustc-env=DST_GIT_DIRTY={}", dirty);

    tauri_build::build()
}

/// 在仓库根执行 git 并取 stdout 首行（失败/非 git 仓库返回 None）。
fn git_output(args: &[&str]) -> Option<String> {
    // CARGO_MANIFEST_DIR = src-tauri；仓库根是它的父目录
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent()?;
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
