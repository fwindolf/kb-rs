use std::fs;
use std::io::Write;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessLogEntry {
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub tool: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AccessLogFilter {
    pub session_id: Option<String>,
    pub domain: Option<String>,
    pub tool: Option<String>,
}

fn log_path(cwd: &Path) -> std::path::PathBuf {
    crate::config::get_kb_dir(cwd).join("access.jsonl")
}

pub fn append(cwd: &Path, entry: &AccessLogEntry) -> Result<()> {
    let path = log_path(cwd);
    let mut line = serde_json::to_string(entry)?;
    line.push('\n');

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(line.as_bytes())?;
    Ok(())
}

pub fn query_log(cwd: &Path, filters: &AccessLogFilter) -> Result<Vec<AccessLogEntry>> {
    let path = log_path(cwd);
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
        let entry: AccessLogEntry = serde_json::from_str(trimmed)?;

        if let Some(ref sid) = filters.session_id {
            if &entry.session_id != sid {
                continue;
            }
        }
        if let Some(ref domain) = filters.domain {
            if entry.domain.as_ref() != Some(domain) {
                continue;
            }
        }
        if let Some(ref tool) = filters.tool {
            if &entry.tool != tool {
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

    fn make_entry(session_id: &str, tool: &str, domain: Option<&str>) -> AccessLogEntry {
        AccessLogEntry {
            session_id: session_id.to_string(),
            timestamp: Utc::now(),
            tool: tool.to_string(),
            domain: domain.map(|s| s.to_string()),
            query: None,
            entry_id: None,
            result_count: None,
            signal: None,
        }
    }

    #[test]
    fn append_and_query() {
        let tmp = init_test_dir();
        append(tmp.path(), &make_entry("s1", "query", Some("rust"))).unwrap();
        append(tmp.path(), &make_entry("s1", "search", Some("go"))).unwrap();
        append(tmp.path(), &make_entry("s2", "query", Some("rust"))).unwrap();

        let all = query_log(tmp.path(), &AccessLogFilter::default()).unwrap();
        assert_eq!(all.len(), 3);

        let by_session = query_log(
            tmp.path(),
            &AccessLogFilter {
                session_id: Some("s1".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(by_session.len(), 2);

        let by_tool = query_log(
            tmp.path(),
            &AccessLogFilter {
                tool: Some("query".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(by_tool.len(), 2);

        let by_domain = query_log(
            tmp.path(),
            &AccessLogFilter {
                domain: Some("go".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(by_domain.len(), 1);
    }

    #[test]
    fn query_empty_log() {
        let tmp = init_test_dir();
        let entries = query_log(tmp.path(), &AccessLogFilter::default()).unwrap();
        assert!(entries.is_empty());
    }
}
