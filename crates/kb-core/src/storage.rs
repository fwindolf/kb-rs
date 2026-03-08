use std::fs;
use std::io::Write;
use std::path::Path;

use crate::error::Result;
use crate::id::generate_record_id;
use crate::types::ExpertiseRecord;

/// Read all records from a JSONL expertise file.
/// Returns an empty vec if the file doesn't exist.
/// Handles legacy `outcome` (singular) → `outcomes` (array) migration.
pub fn read_expertise_file(file_path: &Path) -> Result<Vec<ExpertiseRecord>> {
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };

    let mut records = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Handle legacy `outcome` → `outcomes` migration
        let mut raw: serde_json::Value = serde_json::from_str(trimmed)?;
        if let Some(obj) = raw.as_object_mut() {
            if obj.contains_key("outcome") && !obj.contains_key("outcomes") {
                if let Some(outcome) = obj.remove("outcome") {
                    obj.insert(
                        "outcomes".to_string(),
                        serde_json::Value::Array(vec![outcome]),
                    );
                }
            }
        }

        let record: ExpertiseRecord = serde_json::from_value(raw)?;
        records.push(record);
    }

    Ok(records)
}

/// Append a single record to a JSONL file. Generates an ID if missing.
pub fn append_record(file_path: &Path, record: &mut ExpertiseRecord) -> Result<()> {
    if record.id().is_none() {
        record.set_id(generate_record_id(record));
    }
    let mut line = serde_json::to_string(record)?;
    line.push('\n');

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)?;
    file.write_all(line.as_bytes())?;
    Ok(())
}

/// Atomically write all records to a JSONL file (temp file + rename).
pub fn write_expertise_file(file_path: &Path, records: &mut [ExpertiseRecord]) -> Result<()> {
    for r in records.iter_mut() {
        if r.id().is_none() {
            r.set_id(generate_record_id(r));
        }
    }

    let dir = file_path.parent().unwrap_or(Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;

    for r in records.iter() {
        let line = serde_json::to_string(r)?;
        writeln!(tmp, "{line}")?;
    }

    tmp.flush()?;

    // Persist atomically (rename)
    tmp.persist(file_path).map_err(std::io::Error::other)?;

    Ok(())
}

/// Create an empty expertise file.
pub fn create_expertise_file(file_path: &Path) -> Result<()> {
    fs::write(file_path, "")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Classification, ExpertiseRecord};

    fn make_convention(content: &str) -> ExpertiseRecord {
        ExpertiseRecord::Convention {
            id: None,
            content: content.to_string(),
            classification: Classification::Foundational,
            recorded_at: "2024-01-01T00:00:00.000Z".to_string(),
            evidence: None,
            tags: None,
            relates_to: None,
            supersedes: None,
            outcomes: None,
        }
    }

    #[test]
    fn read_empty_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.jsonl");
        fs::write(&path, "").unwrap();
        let records = read_expertise_file(&path).unwrap();
        assert!(records.is_empty());
    }

    #[test]
    fn read_nonexistent_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nonexistent.jsonl");
        let records = read_expertise_file(&path).unwrap();
        assert!(records.is_empty());
    }

    #[test]
    fn append_and_read() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.jsonl");
        create_expertise_file(&path).unwrap();

        let mut r1 = make_convention("Use snake_case");
        append_record(&path, &mut r1).unwrap();

        let mut r2 = make_convention("Use descriptive names");
        append_record(&path, &mut r2).unwrap();

        let records = read_expertise_file(&path).unwrap();
        assert_eq!(records.len(), 2);
        assert!(records[0].id().is_some());
        assert!(records[1].id().is_some());
    }

    #[test]
    fn atomic_write() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.jsonl");

        let mut records = vec![make_convention("First"), make_convention("Second")];
        write_expertise_file(&path, &mut records).unwrap();

        let read_back = read_expertise_file(&path).unwrap();
        assert_eq!(read_back.len(), 2);
    }

    #[test]
    fn legacy_outcome_migration() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.jsonl");
        // Legacy format with singular "outcome" field
        let legacy = r#"{"type":"convention","content":"test","classification":"tactical","recorded_at":"2024-01-01T00:00:00.000Z","outcome":{"status":"success"}}"#;
        fs::write(&path, format!("{legacy}\n")).unwrap();

        let records = read_expertise_file(&path).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].outcomes().unwrap().len(), 1);
    }
}
