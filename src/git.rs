// SPDX-License-Identifier: GPL-3.0-or-later
// oxid - Git status integration for footer

use std::path::Path;
use std::process::Command;

/// Git status: Clean or Dirty (has changes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitStatus {
    Clean,
    Dirty,
    /// No .git directory or git not available.
    Unknown,
}

/// Check git status for the given directory.
/// Runs `git status -s` and returns Dirty if there is any output.
pub fn get_git_status(dir: &Path) -> GitStatus {
    let git_dir = dir.join(".git");
    if !git_dir.exists() {
        return GitStatus::Unknown;
    }

    let output = Command::new("git")
        .arg("status")
        .arg("-s")
        .current_dir(dir)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            if out.stdout.is_empty() {
                GitStatus::Clean
            } else {
                GitStatus::Dirty
            }
        }
        _ => GitStatus::Unknown,
    }
}
