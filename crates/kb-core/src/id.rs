use sha2::{Digest, Sha256};

use crate::types::ExpertiseRecord;

/// Generate a deterministic record ID from the record's type and unique field.
/// Format: `mx-{first 6 hex chars of SHA256}`.
pub fn generate_record_id(record: &ExpertiseRecord) -> String {
    let key = match record {
        ExpertiseRecord::Convention { content, .. } => format!("convention:{content}"),
        ExpertiseRecord::Pattern { name, .. } => format!("pattern:{name}"),
        ExpertiseRecord::Failure { description, .. } => format!("failure:{description}"),
        ExpertiseRecord::Decision { title, .. } => format!("decision:{title}"),
        ExpertiseRecord::Reference { name, .. } => format!("reference:{name}"),
        ExpertiseRecord::Guide { name, .. } => format!("guide:{name}"),
    };

    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let hash = hasher.finalize();
    let hex = hex_encode(&hash);
    format!("mx-{}", &hex[..6])
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Classification, ExpertiseRecord};

    #[test]
    fn convention_id_is_deterministic() {
        let record = ExpertiseRecord::Convention {
            id: None,
            content: "Always use snake_case".to_string(),
            classification: Classification::Foundational,
            recorded_at: "2024-01-01T00:00:00.000Z".to_string(),
            evidence: None,
            tags: None,
            relates_to: None,
            supersedes: None,
            outcomes: None,
        };
        let id1 = generate_record_id(&record);
        let id2 = generate_record_id(&record);
        assert_eq!(id1, id2);
        assert!(id1.starts_with("mx-"));
        assert_eq!(id1.len(), 9); // "mx-" + 6 hex chars
    }

    #[test]
    fn different_records_different_ids() {
        let r1 = ExpertiseRecord::Convention {
            id: None,
            content: "Use snake_case".to_string(),
            classification: Classification::Foundational,
            recorded_at: "2024-01-01T00:00:00.000Z".to_string(),
            evidence: None,
            tags: None,
            relates_to: None,
            supersedes: None,
            outcomes: None,
        };
        let r2 = ExpertiseRecord::Convention {
            id: None,
            content: "Use camelCase".to_string(),
            classification: Classification::Foundational,
            recorded_at: "2024-01-01T00:00:00.000Z".to_string(),
            evidence: None,
            tags: None,
            relates_to: None,
            supersedes: None,
            outcomes: None,
        };
        assert_ne!(generate_record_id(&r1), generate_record_id(&r2));
    }
}
