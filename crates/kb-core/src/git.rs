use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use crate::types::ExpertiseRecord;

/// Check if the given directory is inside a git repository.
pub fn is_git_repo(cwd: &Path) -> bool {
    Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(cwd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Get all changed files (committed since ref + staged + unstaged).
pub fn get_changed_files(cwd: &Path, since: &str) -> Vec<String> {
    let mut files = BTreeSet::new();

    // Committed changes since ref
    if let Ok(output) = Command::new("git")
        .args(["diff", "--name-only", since])
        .current_dir(cwd)
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    files.insert(trimmed.to_string());
                }
            }
        }
    }

    // Staged but uncommitted
    if let Ok(output) = Command::new("git")
        .args(["diff", "--name-only", "--cached"])
        .current_dir(cwd)
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    files.insert(trimmed.to_string());
                }
            }
        }
    }

    // Unstaged working tree changes
    if let Ok(output) = Command::new("git")
        .args(["diff", "--name-only"])
        .current_dir(cwd)
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    files.insert(trimmed.to_string());
                }
            }
        }
    }

    files.into_iter().collect()
}

/// Check if a file matches any of the changed files (exact or substring).
pub fn file_matches_any(file: &str, changed_files: &[String]) -> bool {
    changed_files.iter().any(|changed| {
        changed == file || changed.ends_with(file) || file.ends_with(changed.as_str())
    })
}

/// Filter records to only those relevant to changed files.
/// Records without a `files` field are always included.
pub fn filter_by_context<'a>(
    records: &'a [ExpertiseRecord],
    changed_files: &[String],
) -> Vec<&'a ExpertiseRecord> {
    records
        .iter()
        .filter(|r| {
            match r.files() {
                Some(files) if !files.is_empty() => {
                    files.iter().any(|f| file_matches_any(f, changed_files))
                }
                _ => true, // No files field → always relevant
            }
        })
        .collect()
}
