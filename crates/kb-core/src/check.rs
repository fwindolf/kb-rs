use std::path::Path;

use regex::Regex;
use serde::Serialize;

use crate::config;
use crate::error::Result;
use crate::storage;
use crate::types::ExpertiseRecord;

#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    pub domain: String,
    pub entry_id: String,
    pub entry_summary: String,
    pub broken_refs: Vec<String>,
}

/// Extract file path-like strings from text.
/// Matches patterns like `src/foo/bar.rs`, `*.ts`, `crates/kb/Cargo.toml`.
fn extract_paths(text: &str) -> Vec<String> {
    let re = Regex::new(
        r#"(?:^|[\s`"',(])([a-zA-Z0-9_.*][\w.*/-]*\.[a-zA-Z0-9]+)(?:[\s`"',):]|$)"#
    )
    .unwrap();

    let mut paths = Vec::new();
    for cap in re.captures_iter(text) {
        let path = cap[1].to_string();
        // Skip things that look like URLs, versions, or numbers
        if path.contains("://") || path.starts_with("0.") || path.starts_with("1.") {
            continue;
        }
        // Must contain a slash or be a glob pattern to be a path reference
        if path.contains('/') || path.contains('*') {
            paths.push(path);
        }
    }
    paths
}

/// Get all searchable text from a record.
fn record_text(record: &ExpertiseRecord) -> String {
    let mut parts = Vec::new();

    match record {
        ExpertiseRecord::Convention { content, .. } => parts.push(content.as_str()),
        ExpertiseRecord::Pattern {
            name,
            description,
            files,
            ..
        } => {
            parts.push(name.as_str());
            parts.push(description.as_str());
            if let Some(f) = files {
                for file in f {
                    parts.push(file.as_str());
                }
            }
        }
        ExpertiseRecord::Failure {
            description,
            resolution,
            ..
        } => {
            parts.push(description.as_str());
            parts.push(resolution.as_str());
        }
        ExpertiseRecord::Decision {
            title, rationale, ..
        } => {
            parts.push(title.as_str());
            parts.push(rationale.as_str());
        }
        ExpertiseRecord::Reference {
            name,
            description,
            files,
            ..
        } => {
            parts.push(name.as_str());
            parts.push(description.as_str());
            if let Some(f) = files {
                for file in f {
                    parts.push(file.as_str());
                }
            }
        }
        ExpertiseRecord::Guide {
            name, description, ..
        } => {
            parts.push(name.as_str());
            parts.push(description.as_str());
        }
    }

    // Include evidence file if present
    if let Some(ev) = record.evidence() {
        if let Some(ref file) = ev.file {
            parts.push(file.as_str());
        }
    }

    parts.join(" ")
}

fn record_summary(record: &ExpertiseRecord) -> String {
    let text = match record {
        ExpertiseRecord::Convention { content, .. } => content.clone(),
        ExpertiseRecord::Pattern { name, .. }
        | ExpertiseRecord::Reference { name, .. }
        | ExpertiseRecord::Guide { name, .. } => name.clone(),
        ExpertiseRecord::Failure { description, .. } => description.clone(),
        ExpertiseRecord::Decision { title, .. } => title.clone(),
    };
    if text.len() > 80 {
        format!("{}...", &text[..77])
    } else {
        text
    }
}

pub fn check_references(cwd: &Path, domain: Option<&str>) -> Result<Vec<CheckResult>> {
    config::ensure_kb_dir(cwd)?;
    let cfg = config::read_config(cwd)?;

    let domains: Vec<&str> = if let Some(d) = domain {
        config::ensure_domain_exists(&cfg, d)?;
        vec![d]
    } else {
        cfg.domains.iter().map(|s| s.as_str()).collect()
    };

    let mut results = Vec::new();

    for domain_name in domains {
        let file_path = config::get_expertise_path(domain_name, cwd)?;
        let records = storage::read_expertise_file(&file_path)?;

        for record in &records {
            let text = record_text(record);
            let paths = extract_paths(&text);

            let mut broken = Vec::new();
            for path_ref in &paths {
                // Skip glob patterns — they're not checkable as literal paths
                if path_ref.contains('*') {
                    continue;
                }
                let full_path = cwd.join(path_ref);
                if !full_path.exists() {
                    broken.push(path_ref.clone());
                }
            }

            if !broken.is_empty() {
                results.push(CheckResult {
                    domain: domain_name.to_string(),
                    entry_id: record.id().unwrap_or("(no id)").to_string(),
                    entry_summary: record_summary(record),
                    broken_refs: broken,
                });
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_paths_from_text() {
        let text = "Use the pattern in src/utils/helpers.rs and check crates/kb/Cargo.toml";
        let paths = extract_paths(text);
        assert!(paths.contains(&"src/utils/helpers.rs".to_string()));
        assert!(paths.contains(&"crates/kb/Cargo.toml".to_string()));
    }

    #[test]
    fn extract_paths_skips_non_paths() {
        let text = "version 1.0.0 and some word.thing without slashes";
        let paths = extract_paths(text);
        assert!(paths.is_empty());
    }

    #[test]
    fn extract_glob_patterns() {
        let text = "matches *.ts files and src/**/*.rs patterns";
        let paths = extract_paths(text);
        assert!(paths.contains(&"*.ts".to_string()));
        assert!(paths.contains(&"src/**/*.rs".to_string()));
    }

    #[test]
    fn check_references_with_broken_paths() {
        use crate::types::{Classification, ExpertiseRecord};

        let tmp = tempfile::tempdir().unwrap();
        crate::config::init_kb_dir(tmp.path()).unwrap();

        // Add a domain
        let mut cfg = config::read_config(tmp.path()).unwrap();
        cfg.domains.push("test".to_string());
        config::write_config(&cfg, tmp.path()).unwrap();

        // Create expertise file with a record referencing a nonexistent path
        let file_path = config::get_expertise_path("test", tmp.path()).unwrap();
        let mut record = ExpertiseRecord::Convention {
            id: None,
            content: "Always check src/nonexistent/file.rs before deploying".to_string(),
            classification: Classification::Tactical,
            recorded_at: "2024-01-01T00:00:00.000Z".to_string(),
            evidence: None,
            tags: None,
            relates_to: None,
            supersedes: None,
            outcomes: None,
        };
        storage::append_record(&file_path, &mut record).unwrap();

        let results = check_references(tmp.path(), Some("test")).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0]
            .broken_refs
            .contains(&"src/nonexistent/file.rs".to_string()));
    }
}
