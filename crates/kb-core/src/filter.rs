use crate::types::{Classification, ExpertiseRecord, RecordType};

pub fn filter_by_type(
    records: &[ExpertiseRecord],
    record_type: RecordType,
) -> Vec<&ExpertiseRecord> {
    records
        .iter()
        .filter(|r| r.record_type() == record_type)
        .collect()
}

pub fn filter_by_classification(
    records: &[ExpertiseRecord],
    classification: Classification,
) -> Vec<&ExpertiseRecord> {
    records
        .iter()
        .filter(|r| r.classification() == classification)
        .collect()
}

pub fn filter_by_file<'a>(records: &'a [ExpertiseRecord], file: &str) -> Vec<&'a ExpertiseRecord> {
    let file_lower = file.to_lowercase();
    records
        .iter()
        .filter(|r| {
            r.files()
                .map(|files| files.iter().any(|f| f.to_lowercase().contains(&file_lower)))
                .unwrap_or(false)
        })
        .collect()
}

/// Find a duplicate record in existing records.
/// Returns the index and a reference to the duplicate if found.
pub fn find_duplicate<'a>(
    existing: &'a [ExpertiseRecord],
    new_record: &ExpertiseRecord,
) -> Option<(usize, &'a ExpertiseRecord)> {
    for (i, record) in existing.iter().enumerate() {
        if record.record_type() != new_record.record_type() {
            continue;
        }
        let is_dup = match (record, new_record) {
            (
                ExpertiseRecord::Convention { content: a, .. },
                ExpertiseRecord::Convention { content: b, .. },
            ) => a == b,
            (
                ExpertiseRecord::Pattern { name: a, .. },
                ExpertiseRecord::Pattern { name: b, .. },
            ) => a == b,
            (
                ExpertiseRecord::Failure { description: a, .. },
                ExpertiseRecord::Failure { description: b, .. },
            ) => a == b,
            (
                ExpertiseRecord::Decision { title: a, .. },
                ExpertiseRecord::Decision { title: b, .. },
            ) => a == b,
            (
                ExpertiseRecord::Reference { name: a, .. },
                ExpertiseRecord::Reference { name: b, .. },
            ) => a == b,
            (ExpertiseRecord::Guide { name: a, .. }, ExpertiseRecord::Guide { name: b, .. }) => {
                a == b
            }
            _ => false,
        };
        if is_dup {
            return Some((i, record));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Classification;

    fn convention(content: &str) -> ExpertiseRecord {
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

    fn pattern(name: &str) -> ExpertiseRecord {
        ExpertiseRecord::Pattern {
            id: None,
            name: name.to_string(),
            description: "desc".to_string(),
            files: Some(vec!["src/main.rs".to_string()]),
            classification: Classification::Tactical,
            recorded_at: "2024-01-01T00:00:00.000Z".to_string(),
            evidence: None,
            tags: None,
            relates_to: None,
            supersedes: None,
            outcomes: None,
        }
    }

    #[test]
    fn filter_by_type_works() {
        let records = vec![convention("a"), pattern("b"), convention("c")];
        let filtered = filter_by_type(&records, RecordType::Convention);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn filter_by_file_works() {
        let records = vec![convention("a"), pattern("b")];
        let filtered = filter_by_file(&records, "main.rs");
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn find_duplicate_convention() {
        let existing = vec![convention("test")];
        let new = convention("test");
        assert!(find_duplicate(&existing, &new).is_some());

        let different = convention("other");
        assert!(find_duplicate(&existing, &different).is_none());
    }

    #[test]
    fn find_duplicate_named_type() {
        let existing = vec![pattern("Error Handling")];
        let new = pattern("Error Handling");
        assert!(find_duplicate(&existing, &new).is_some());

        let different = pattern("Logging");
        assert!(find_duplicate(&existing, &different).is_none());
    }
}
