use crate::error::{KbError, Result};
use crate::types::ExpertiseRecord;

/// Resolve an identifier to a record within a list.
/// Accepts: full ID (mx-abc123), bare hash (abc123), or prefix (abc / mx-abc).
/// Returns (index, record) or error if not found / ambiguous.
pub fn resolve_record_id<'a>(
    records: &'a [ExpertiseRecord],
    identifier: &str,
) -> Result<(usize, &'a ExpertiseRecord)> {
    // Normalize: strip mx- prefix to get the hash part
    let hash = identifier.strip_prefix("mx-").unwrap_or(identifier);

    // Try exact match first
    let full_id = format!("mx-{hash}");
    if let Some((i, record)) = records
        .iter()
        .enumerate()
        .find(|(_, r)| r.id() == Some(full_id.as_str()))
    {
        return Ok((i, record));
    }

    // Try prefix match
    let prefix = format!("mx-{hash}");
    let matches: Vec<(usize, &ExpertiseRecord)> = records
        .iter()
        .enumerate()
        .filter(|(_, r)| r.id().map(|id| id.starts_with(&prefix)).unwrap_or(false))
        .collect();

    match matches.len() {
        1 => Ok(matches[0]),
        0 => Err(KbError::RecordNotFound(identifier.to_string())),
        n => {
            let ids: Vec<&str> = matches.iter().filter_map(|(_, r)| r.id()).collect();
            Err(KbError::AmbiguousId {
                id: identifier.to_string(),
                count: n,
                ids: ids.join(", "),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Classification, ExpertiseRecord};

    fn record_with_id(id: &str) -> ExpertiseRecord {
        ExpertiseRecord::Convention {
            id: Some(id.to_string()),
            content: format!("content for {id}"),
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
    fn exact_match() {
        let records = vec![record_with_id("mx-abc123")];
        let (i, _) = resolve_record_id(&records, "mx-abc123").unwrap();
        assert_eq!(i, 0);
    }

    #[test]
    fn bare_hash_match() {
        let records = vec![record_with_id("mx-abc123")];
        let (i, _) = resolve_record_id(&records, "abc123").unwrap();
        assert_eq!(i, 0);
    }

    #[test]
    fn prefix_match() {
        let records = vec![record_with_id("mx-abc123")];
        let (i, _) = resolve_record_id(&records, "abc").unwrap();
        assert_eq!(i, 0);
    }

    #[test]
    fn not_found() {
        let records = vec![record_with_id("mx-abc123")];
        assert!(resolve_record_id(&records, "xyz").is_err());
    }

    #[test]
    fn ambiguous() {
        let records = vec![record_with_id("mx-abc123"), record_with_id("mx-abc456")];
        let err = resolve_record_id(&records, "abc").unwrap_err();
        assert!(matches!(err, KbError::AmbiguousId { .. }));
    }
}
