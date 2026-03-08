use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub action: String,
    pub domain: String,
    pub entry_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<HashMap<String, (String, String)>>,
}

#[derive(Debug, Clone, Default)]
pub struct ChangelogFilter {
    pub session_id: Option<String>,
    pub domain: Option<String>,
    pub action: Option<String>,
}

fn changelog_path(cwd: &Path) -> std::path::PathBuf {
    crate::config::get_kb_dir(cwd).join("changelog.jsonl")
}

pub fn append(cwd: &Path, entry: &ChangelogEntry) -> Result<()> {
    let path = changelog_path(cwd);
    let mut line = serde_json::to_string(entry)?;
    line.push('\n');

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(line.as_bytes())?;
    Ok(())
}

pub fn query_changelog(cwd: &Path, filters: &ChangelogFilter) -> Result<Vec<ChangelogEntry>> {
    let path = changelog_path(cwd);
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };

    let mut entries = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let entry: ChangelogEntry = serde_json::from_str(trimmed)?;

        if let Some(ref sid) = filters.session_id {
            if entry.session_id.as_ref() != Some(sid) {
                continue;
            }
        }
        if let Some(ref domain) = filters.domain {
            if &entry.domain != domain {
                continue;
            }
        }
        if let Some(ref action) = filters.action {
            if &entry.action != action {
                continue;
            }
        }

        entries.push(entry);
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_test_dir() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        crate::config::init_kb_dir(tmp.path()).unwrap();
        tmp
    }

    fn make_entry(session_id: Option<&str>, action: &str, domain: &str) -> ChangelogEntry {
        ChangelogEntry {
            session_id: session_id.map(|s| s.to_string()),
            timestamp: Utc::now(),
            action: action.to_string(),
            domain: domain.to_string(),
            entry_id: "mx-abc123".to_string(),
            summary: Some("test record".to_string()),
            diff: None,
        }
    }

    #[test]
    fn append_and_query() {
        let tmp = init_test_dir();
        append(tmp.path(), &make_entry(Some("s1"), "record", "rust")).unwrap();
        append(tmp.path(), &make_entry(Some("s1"), "edit", "rust")).unwrap();
        append(tmp.path(), &make_entry(None, "delete", "go")).unwrap();

        let all = query_changelog(tmp.path(), &ChangelogFilter::default()).unwrap();
        assert_eq!(all.len(), 3);

        let by_action = query_changelog(
            tmp.path(),
            &ChangelogFilter {
                action: Some("record".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(by_action.len(), 1);

        let by_domain = query_changelog(
            tmp.path(),
            &ChangelogFilter {
                domain: Some("go".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(by_domain.len(), 1);
    }

    #[test]
    fn query_empty_changelog() {
        let tmp = init_test_dir();
        let entries = query_changelog(tmp.path(), &ChangelogFilter::default()).unwrap();
        assert!(entries.is_empty());
    }
}
