use std::process::Command;
use std::path::Path;

pub struct GitStatus {
    pub branch: String,
    pub is_dirty: bool,
}

pub fn get_git_status(path: &Path) -> Option<GitStatus> {
    if !path.exists() {
        return None;
    }

    let branch = Command::new("git")
        .current_dir(path)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })?;

    let is_dirty = Command::new("git")
        .current_dir(path)
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .map(|output| !output.stdout.is_empty())
        .unwrap_or(false);

    Some(GitStatus { branch, is_dirty })
}
