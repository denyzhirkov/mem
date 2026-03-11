use std::path::Path;
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("Git command IO failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Git error ({code}): {stderr}")]
    Git { code: i32, stderr: String },
    #[error("Not a git repository")]
    NotARepo,
}

fn run_git(path: &Path, args: &[&str]) -> Result<String, SyncError> {
    let output = Command::new("git")
        .current_dir(path)
        .args(args)
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(SyncError::Git {
            code: output.status.code().unwrap_or(-1),
            stderr,
        })
    }
}

pub fn is_git_repo(path: &Path) -> bool {
    run_git(path, &["rev-parse", "--is-inside-work-tree"]).is_ok()
}

pub fn sync_status(path: &Path) -> Result<String, SyncError> {
    if !is_git_repo(path) {
        return Err(SyncError::NotARepo);
    }
    run_git(path, &["status", "--short"])
}

pub fn commit_all(path: &Path, message: &str) -> Result<(), SyncError> {
    if !is_git_repo(path) {
        return Err(SyncError::NotARepo);
    }
    run_git(path, &["add", "."])?;
    // Git commit might fail if there's nothing to commit.
    match run_git(path, &["commit", "-m", message]) {
        Ok(_) => Ok(()),
        Err(SyncError::Git { stderr, .. }) if stderr.contains("nothing to commit") || stderr.contains("working tree clean") => {
            Ok(()) // Clean tree is fine
        }
        Err(e) => Err(e),
    }
}

pub fn pull(path: &Path) -> Result<String, SyncError> {
    if !is_git_repo(path) {
        return Err(SyncError::NotARepo);
    }
    run_git(path, &["pull", "--rebase"])
}

pub fn push(path: &Path) -> Result<String, SyncError> {
    if !is_git_repo(path) {
        return Err(SyncError::NotARepo);
    }
    run_git(path, &["push"])
}

pub fn check_conflicts(path: &Path) -> Result<bool, SyncError> {
    if !is_git_repo(path) {
        return Err(SyncError::NotARepo);
    }
    // Any file with 'U' in git status indicates unmerged paths which means conflicts
    let status = sync_status(path)?;
    Ok(status.lines().any(|line| line.starts_with('U') || line.chars().nth(1) == Some('U')))
}
