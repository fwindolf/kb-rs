use crate::scoring::compute_confirmation_score;
use crate::types::{Classification, ExpertiseRecord, RecordType};

pub const DEFAULT_BUDGET: usize = 4000;

/// Priority order for record types (lower index = higher priority).
const TYPE_PRIORITY: [RecordType; 6] = [
    RecordType::Convention,
    RecordType::Decision,
    RecordType::Pattern,
    RecordType::Guide,
    RecordType::Failure,
    RecordType::Reference,
];

/// Priority order for classifications.
const CLASSIFICATION_PRIORITY: [Classification; 3] = [
    Classification::Foundational,
    Classification::Tactical,
    Classification::Observational,
];

pub struct DomainRecords<'a> {
    pub domain: String,
    pub records: Vec<&'a ExpertiseRecord>,
}

pub struct BudgetResult<'a> {
    pub kept: Vec<DomainRecords<'a>>,
    pub dropped_count: usize,
    pub dropped_domain_count: usize,
}

/// Estimate token count from character count (chars / 4).
pub fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}

fn type_priority_index(rt: RecordType) -> usize {
    TYPE_PRIORITY
        .iter()
        .position(|&t| t == rt)
        .unwrap_or(TYPE_PRIORITY.len())
}

fn classification_priority_index(c: Classification) -> usize {
    CLASSIFICATION_PRIORITY
        .iter()
        .position(|&cl| cl == c)
        .unwrap_or(CLASSIFICATION_PRIORITY.len())
}

fn record_sort_key(r: &ExpertiseRecord) -> (usize, usize, i64, i64) {
    let type_idx = type_priority_index(r.record_type());
    let class_idx = classification_priority_index(r.classification());
    let cs = compute_confirmation_score(r);
    let time = chrono::DateTime::parse_from_rfc3339(r.recorded_at())
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(0);
    (type_idx, class_idx, -(cs * 1000.0) as i64, -time)
}

/// Apply a token budget to records across multiple domains.
/// Records are prioritized by type, classification, confirmation score, then recency.
pub fn apply_budget<'a, F>(
    domains: &[DomainRecords<'a>],
    budget: usize,
    format_record: F,
) -> BudgetResult<'a>
where
    F: Fn(&ExpertiseRecord, &str) -> String,
{
    // Flatten and tag
    let mut tagged: Vec<(&str, &'a ExpertiseRecord)> = Vec::new();
    for d in domains {
        for r in &d.records {
            tagged.push((d.domain.as_str(), r));
        }
    }

    // Sort by priority
    tagged.sort_by(|a, b| {
        let ka = record_sort_key(a.1);
        let kb = record_sort_key(b.1);
        ka.cmp(&kb)
    });

    let total_records = tagged.len();
    let mut used_tokens = 0;
    let mut kept_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();

    for (i, (domain, record)) in tagged.iter().enumerate() {
        let formatted = format_record(record, domain);
        let cost = estimate_tokens(&formatted);
        if used_tokens + cost <= budget {
            used_tokens += cost;
            kept_indices.insert(i);
        }
    }

    // Rebuild domain groups preserving original order
    let mut result = Vec::new();
    let mut dropped_domains = std::collections::HashSet::new();

    for domain_rec in domains {
        let mut kept_records = Vec::new();
        for record in &domain_rec.records {
            let idx = tagged
                .iter()
                .position(|(d, r)| *d == domain_rec.domain.as_str() && std::ptr::eq(*r, *record));
            if let Some(idx) = idx {
                if kept_indices.contains(&idx) {
                    kept_records.push(*record);
                } else {
                    dropped_domains.insert(domain_rec.domain.clone());
                }
            }
        }
        if !kept_records.is_empty() {
            result.push(DomainRecords {
                domain: domain_rec.domain.clone(),
                records: kept_records,
            });
        } else if !domain_rec.records.is_empty() {
            dropped_domains.insert(domain_rec.domain.clone());
        }
    }

    BudgetResult {
        kept: result,
        dropped_count: total_records - kept_indices.len(),
        dropped_domain_count: dropped_domains.len(),
    }
}

/// Format the truncation summary line.
pub fn format_budget_summary(dropped_count: usize, dropped_domain_count: usize) -> String {
    let domain_part = if dropped_domain_count > 0 {
        format!(
            " across {} domain{}",
            dropped_domain_count,
            if dropped_domain_count == 1 { "" } else { "s" }
        )
    } else {
        String::new()
    };
    format!(
        "... and {} more record{}{} (use --budget <n> to show more)",
        dropped_count,
        if dropped_count == 1 { "" } else { "s" },
        domain_part
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_tokens_basic() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
        assert_eq!(estimate_tokens("abcdefgh"), 2);
    }

    #[test]
    fn format_budget_summary_singular() {
        let s = format_budget_summary(1, 1);
        assert!(s.contains("1 more record"));
        assert!(s.contains("1 domain"));
    }

    #[test]
    fn format_budget_summary_plural() {
        let s = format_budget_summary(5, 2);
        assert!(s.contains("5 more records"));
        assert!(s.contains("2 domains"));
    }
}
