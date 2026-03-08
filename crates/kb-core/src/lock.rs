use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use crate::error::{KbError, Result};

const LOCK_STALE_MS: u64 = 30_000;
const LOCK_RETRY_INTERVAL_MS: u64 = 50;
const LOCK_TIMEOUT_MS: u64 = 5_000;

fn lock_path(file_path: &Path) -> PathBuf {
    let mut p = file_path.as_os_str().to_owned();
    p.push(".lock");
    PathBuf::from(p)
}

fn is_stale_lock(path: &Path) -> bool {
    match fs::metadata(path) {
        Ok(meta) => match meta.modified() {
            Ok(modified) => {
                let age = SystemTime::now()
                    .duration_since(modified)
                    .unwrap_or(Duration::ZERO);
                age.as_millis() > LOCK_STALE_MS as u128
            }
            Err(_) => false,
        },
        Err(_) => false,
    }
}

fn acquire_lock(path: &Path) -> Result<()> {
    let deadline = Instant::now() + Duration::from_millis(LOCK_TIMEOUT_MS);

    loop {
        match OpenOptions::new().write(true).create_new(true).open(path) {
            Ok(_) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                if is_stale_lock(path) {
                    let _ = fs::remove_file(path);
                    continue;
                }

                if Instant::now() >= deadline {
                    return Err(KbError::LockTimeout(path.display().to_string()));
                }

                thread::sleep(Duration::from_millis(LOCK_RETRY_INTERVAL_MS));
            }
            Err(e) => return Err(KbError::Io(e)),
        }
    }
}

fn release_lock(path: &Path) {
    let _ = fs::remove_file(path);
}

/// Execute a closure while holding an advisory file lock.
pub fn with_file_lock<T, F>(file_path: &Path, f: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    let lp = lock_path(file_path);
    acquire_lock(&lp)?;
    let result = f();
    release_lock(&lp);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_and_release() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("test.jsonl");
        fs::write(&file, "").unwrap();

        let result = with_file_lock(&file, || Ok(42));
        assert_eq!(result.unwrap(), 42);
        assert!(!lock_path(&file).exists());
    }

    #[test]
    fn lock_released_on_error() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("test.jsonl");
        fs::write(&file, "").unwrap();

        let result: Result<()> = with_file_lock(&file, || {
            Err(KbError::ValidationError("test".to_string()))
        });
        assert!(result.is_err());
        assert!(!lock_path(&file).exists());
    }
}
