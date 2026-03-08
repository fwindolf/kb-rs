use crate::types::ExpertiseRecord;

/// Count successful outcomes.
pub fn get_success_count(record: &ExpertiseRecord) -> usize {
    record
        .outcomes()
        .map(|outcomes| {
            outcomes
                .iter()
                .filter(|o| o.status == crate::types::OutcomeStatus::Success)
                .count()
        })
        .unwrap_or(0)
}

/// Count failed outcomes.
pub fn get_failure_count(record: &ExpertiseRecord) -> usize {
    record
        .outcomes()
        .map(|outcomes| {
            outcomes
                .iter()
                .filter(|o| o.status == crate::types::OutcomeStatus::Failure)
                .count()
        })
        .unwrap_or(0)
}

/// Total number of recorded applications.
pub fn get_total_applications(record: &ExpertiseRecord) -> usize {
    record.outcomes().map(|o| o.len()).unwrap_or(0)
}

/// Success rate (0.0-1.0). Partial outcomes count as 0.5.
pub fn get_success_rate(record: &ExpertiseRecord) -> f64 {
    let total = get_total_applications(record);
    if total == 0 {
        return 0.0;
    }
    let success = get_success_count(record) as f64;
    let partial = record
        .outcomes()
        .map(|outcomes| {
            outcomes
                .iter()
                .filter(|o| o.status == crate::types::OutcomeStatus::Partial)
                .count()
        })
        .unwrap_or(0) as f64;
    (success + partial * 0.5) / total as f64
}

/// Confirmation-frequency score: success count + 0.5 * partial count.
pub fn compute_confirmation_score(record: &ExpertiseRecord) -> f64 {
    let outcomes = match record.outcomes() {
        Some(o) if !o.is_empty() => o,
        _ => return 0.0,
    };
    let success = outcomes
        .iter()
        .filter(|o| o.status == crate::types::OutcomeStatus::Success)
        .count() as f64;
    let partial = outcomes
        .iter()
        .filter(|o| o.status == crate::types::OutcomeStatus::Partial)
        .count() as f64;
    success + partial * 0.5
}

/// Apply confirmation-frequency boost to a base score.
pub fn apply_confirmation_boost(
    base_score: f64,
    record: &ExpertiseRecord,
    boost_factor: f64,
) -> f64 {
    let cs = compute_confirmation_score(record);
    if cs == 0.0 {
        return base_score;
    }
    base_score * (1.0 + boost_factor * cs)
}

/// Sort records by confirmation score (highest first). Stable sort.
pub fn sort_by_confirmation_score(records: &mut [&ExpertiseRecord]) {
    records.sort_by(|a, b| {
        compute_confirmation_score(b)
            .partial_cmp(&compute_confirmation_score(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Classification, ExpertiseRecord, Outcome, OutcomeStatus};

    fn record_with_outcomes(outcomes: Vec<Outcome>) -> ExpertiseRecord {
        ExpertiseRecord::Convention {
            id: None,
            content: "test".to_string(),
            classification: Classification::Tactical,
            recorded_at: "2024-01-01T00:00:00.000Z".to_string(),
            evidence: None,
            tags: None,
            relates_to: None,
            supersedes: None,
            outcomes: Some(outcomes),
        }
    }

    #[test]
    fn no_outcomes_score_zero() {
        let r = ExpertiseRecord::Convention {
            id: None,
            content: "test".to_string(),
            classification: Classification::Tactical,
            recorded_at: "2024-01-01T00:00:00.000Z".to_string(),
            evidence: None,
            tags: None,
            relates_to: None,
            supersedes: None,
            outcomes: None,
        };
        assert_eq!(compute_confirmation_score(&r), 0.0);
    }

    #[test]
    fn success_and_partial() {
        let r = record_with_outcomes(vec![
            Outcome {
                status: OutcomeStatus::Success,
                duration: None,
                test_results: None,
                agent: None,
                notes: None,
                recorded_at: None,
            },
            Outcome {
                status: OutcomeStatus::Partial,
                duration: None,
                test_results: None,
                agent: None,
                notes: None,
                recorded_at: None,
            },
            Outcome {
                status: OutcomeStatus::Failure,
                duration: None,
                test_results: None,
                agent: None,
                notes: None,
                recorded_at: None,
            },
        ]);
        assert_eq!(compute_confirmation_score(&r), 1.5); // 1 + 0.5
        assert_eq!(get_success_count(&r), 1);
        assert_eq!(get_failure_count(&r), 1);
        assert_eq!(get_total_applications(&r), 3);
    }

    #[test]
    fn boost_with_zero_score() {
        let r = ExpertiseRecord::Convention {
            id: None,
            content: "test".to_string(),
            classification: Classification::Tactical,
            recorded_at: "2024-01-01T00:00:00.000Z".to_string(),
            evidence: None,
            tags: None,
            relates_to: None,
            supersedes: None,
            outcomes: None,
        };
        assert_eq!(apply_confirmation_boost(10.0, &r, 0.1), 10.0);
    }
}
