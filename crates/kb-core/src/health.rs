use std::collections::HashMap;

use crate::types::{Classification, ExpertiseRecord, RecordType, ShelfLife};

#[derive(Debug)]
pub struct DomainHealth {
    pub governance_utilization: u32,
    pub stale_count: usize,
    pub type_distribution: HashMap<RecordType, usize>,
    pub classification_distribution: HashMap<Classification, usize>,
    pub oldest_timestamp: Option<String>,
    pub newest_timestamp: Option<String>,
}

/// Check if a record is stale based on classification and shelf life.
pub fn is_record_stale(
    record: &ExpertiseRecord,
    now: chrono::DateTime<chrono::Utc>,
    shelf_life: &ShelfLife,
) -> bool {
    let classification = record.classification();

    if classification == Classification::Foundational {
        return false;
    }

    let recorded_at = match chrono::DateTime::parse_from_rfc3339(record.recorded_at()) {
        Ok(dt) => dt.with_timezone(&chrono::Utc),
        Err(_) => return false,
    };

    let age_days = (now - recorded_at).num_days();

    match classification {
        Classification::Tactical => age_days > shelf_life.tactical as i64,
        Classification::Observational => age_days > shelf_life.observational as i64,
        Classification::Foundational => false,
    }
}

/// Calculate comprehensive health metrics for a domain.
pub fn calculate_domain_health(
    records: &[ExpertiseRecord],
    max_entries: u32,
    shelf_life: &ShelfLife,
) -> DomainHealth {
    let now = chrono::Utc::now();

    let mut type_dist: HashMap<RecordType, usize> = HashMap::new();
    let mut class_dist: HashMap<Classification, usize> = HashMap::new();
    let mut stale_count = 0;
    let mut oldest: Option<String> = None;
    let mut newest: Option<String> = None;

    for record in records {
        *type_dist.entry(record.record_type()).or_default() += 1;
        *class_dist.entry(record.classification()).or_default() += 1;

        if is_record_stale(record, now, shelf_life) {
            stale_count += 1;
        }

        let ts = record.recorded_at().to_string();
        if oldest.as_ref().is_none_or(|o| ts < *o) {
            oldest = Some(ts.clone());
        }
        if newest.as_ref().is_none_or(|n| ts > *n) {
            newest = Some(ts);
        }
    }

    let utilization = if max_entries > 0 {
        ((records.len() as f64 / max_entries as f64) * 100.0).round() as u32
    } else {
        0
    };

    DomainHealth {
        governance_utilization: utilization,
        stale_count,
        type_distribution: type_dist,
        classification_distribution: class_dist,
        oldest_timestamp: oldest,
        newest_timestamp: newest,
    }
}
