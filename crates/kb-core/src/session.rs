use std::fs;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{KbError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub label: Option<String>,
    pub started_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<DateTime<Utc>>,
}

fn sessions_dir(cwd: &Path) -> std::path::PathBuf {
    crate::config::get_kb_dir(cwd).join("sessions")
}

fn session_path(cwd: &Path, id: &str) -> std::path::PathBuf {
    sessions_dir(cwd).join(format!("{id}.json"))
}

fn generate_session_id() -> String {
    let now = Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let random: u64 = rand_bytes();
    let mut hasher = Sha256::new();
    hasher.update(now.to_le_bytes());
    hasher.update(random.to_le_bytes());
    let hash = hasher.finalize();
    let hex: String = hash.iter().take(3).map(|b| format!("{b:02x}")).collect();
    format!("kb-{hex}")
}

/// Poor-man's random: read 8 bytes from timestamp nanoseconds + pid.
fn rand_bytes() -> u64 {
    let pid = std::process::id() as u64;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    pid.wrapping_mul(6364136223846793005).wrapping_add(ts)
}

pub fn start_session(cwd: &Path, label: Option<&str>) -> Result<Session> {
    let dir = sessions_dir(cwd);
    fs::create_dir_all(&dir)?;

    let session = Session {
        id: generate_session_id(),
        label: label.map(|s| s.to_string()),
        started_at: Utc::now(),
        ended_at: None,
    };

    let json = serde_json::to_string_pretty(&session)?;
    fs::write(session_path(cwd, &session.id), json)?;
    Ok(session)
}

pub fn resume_session(cwd: &Path, id: &str) -> Result<Session> {
    let session = get_session(cwd, id)?;
    if session.ended_at.is_some() {
        return Err(KbError::ValidationError(format!(
            "Session \"{id}\" has already ended"
        )));
    }
    Ok(session)
}

pub fn end_session(cwd: &Path, id: &str) -> Result<()> {
    let mut session = get_session(cwd, id)?;
    if session.ended_at.is_some() {
        return Err(KbError::ValidationError(format!(
            "Session \"{id}\" has already ended"
        )));
    }
    session.ended_at = Some(Utc::now());
    let json = serde_json::to_string_pretty(&session)?;
    fs::write(session_path(cwd, id), json)?;
    Ok(())
}

pub fn list_sessions(cwd: &Path) -> Result<Vec<Session>> {
    let dir = sessions_dir(cwd);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            let content = fs::read_to_string(&path)?;
            let session: Session = serde_json::from_str(&content)?;
            sessions.push(session);
        }
    }
    sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(sessions)
}

pub fn get_session(cwd: &Path, id: &str) -> Result<Session> {
    let path = session_path(cwd, id);
    let content = fs::read_to_string(&path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            KbError::RecordNotFound(id.to_string())
        } else {
            KbError::Io(e)
        }
    })?;
    let session: Session = serde_json::from_str(&content)?;
    Ok(session)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_test_dir() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        crate::config::init_kb_dir(tmp.path()).unwrap();
        tmp
    }

    #[test]
    fn start_and_get_session() {
        let tmp = init_test_dir();
        let session = start_session(tmp.path(), Some("test label")).unwrap();
        assert!(session.id.starts_with("kb-"));
        assert_eq!(session.label.as_deref(), Some("test label"));
        assert!(session.ended_at.is_none());

        let loaded = get_session(tmp.path(), &session.id).unwrap();
        assert_eq!(loaded.id, session.id);
    }

    #[test]
    fn resume_and_end_session() {
        let tmp = init_test_dir();
        let session = start_session(tmp.path(), None).unwrap();

        let resumed = resume_session(tmp.path(), &session.id).unwrap();
        assert_eq!(resumed.id, session.id);

        end_session(tmp.path(), &session.id).unwrap();

        // Can't resume ended session
        assert!(resume_session(tmp.path(), &session.id).is_err());
        // Can't end twice
        assert!(end_session(tmp.path(), &session.id).is_err());
    }

    #[test]
    fn list_sessions_returns_all() {
        let tmp = init_test_dir();
        start_session(tmp.path(), Some("first")).unwrap();
        start_session(tmp.path(), Some("second")).unwrap();

        let sessions = list_sessions(tmp.path()).unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn get_nonexistent_session_errors() {
        let tmp = init_test_dir();
        assert!(get_session(tmp.path(), "kb-nonexistent").is_err());
    }
}
