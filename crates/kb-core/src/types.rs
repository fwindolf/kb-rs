use serde::{Deserialize, Serialize};

// ── Enums ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordType {
    Convention,
    Pattern,
    Failure,
    Decision,
    Reference,
    Guide,
}

impl RecordType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Convention => "convention",
            Self::Pattern => "pattern",
            Self::Failure => "failure",
            Self::Decision => "decision",
            Self::Reference => "reference",
            Self::Guide => "guide",
        }
    }
}

impl std::fmt::Display for RecordType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Classification {
    Foundational,
    Tactical,
    Observational,
}

impl Classification {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Foundational => "foundational",
            Self::Tactical => "tactical",
            Self::Observational => "observational",
        }
    }
}

impl std::fmt::Display for Classification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutcomeStatus {
    Success,
    Failure,
    Partial,
}

impl OutcomeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
            Self::Partial => "partial",
        }
    }
}

impl std::fmt::Display for OutcomeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Supporting Types ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Evidence {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bead: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outcome {
    pub status: OutcomeStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_results: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_at: Option<String>,
}

// ── Expertise Record (tagged enum) ─────────────────────────────────────────

/// The main record type. Tagged by the `"type"` field in JSON for JSONL compat.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ExpertiseRecord {
    Convention {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        content: String,
        classification: Classification,
        recorded_at: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        evidence: Option<Evidence>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tags: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        relates_to: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        supersedes: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        outcomes: Option<Vec<Outcome>>,
    },
    Pattern {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        name: String,
        description: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        files: Option<Vec<String>>,
        classification: Classification,
        recorded_at: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        evidence: Option<Evidence>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tags: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        relates_to: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        supersedes: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        outcomes: Option<Vec<Outcome>>,
    },
    Failure {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        description: String,
        resolution: String,
        classification: Classification,
        recorded_at: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        evidence: Option<Evidence>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tags: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        relates_to: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        supersedes: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        outcomes: Option<Vec<Outcome>>,
    },
    Decision {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        title: String,
        rationale: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        date: Option<String>,
        classification: Classification,
        recorded_at: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        evidence: Option<Evidence>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tags: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        relates_to: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        supersedes: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        outcomes: Option<Vec<Outcome>>,
    },
    Reference {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        name: String,
        description: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        files: Option<Vec<String>>,
        classification: Classification,
        recorded_at: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        evidence: Option<Evidence>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tags: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        relates_to: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        supersedes: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        outcomes: Option<Vec<Outcome>>,
    },
    Guide {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        name: String,
        description: String,
        classification: Classification,
        recorded_at: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        evidence: Option<Evidence>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tags: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        relates_to: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        supersedes: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        outcomes: Option<Vec<Outcome>>,
    },
}

/// Helper methods for accessing common fields across all variants.
impl ExpertiseRecord {
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Convention { id, .. }
            | Self::Pattern { id, .. }
            | Self::Failure { id, .. }
            | Self::Decision { id, .. }
            | Self::Reference { id, .. }
            | Self::Guide { id, .. } => id.as_deref(),
        }
    }

    pub fn set_id(&mut self, new_id: String) {
        match self {
            Self::Convention { id, .. }
            | Self::Pattern { id, .. }
            | Self::Failure { id, .. }
            | Self::Decision { id, .. }
            | Self::Reference { id, .. }
            | Self::Guide { id, .. } => *id = Some(new_id),
        }
    }

    pub fn record_type(&self) -> RecordType {
        match self {
            Self::Convention { .. } => RecordType::Convention,
            Self::Pattern { .. } => RecordType::Pattern,
            Self::Failure { .. } => RecordType::Failure,
            Self::Decision { .. } => RecordType::Decision,
            Self::Reference { .. } => RecordType::Reference,
            Self::Guide { .. } => RecordType::Guide,
        }
    }

    pub fn classification(&self) -> Classification {
        match self {
            Self::Convention { classification, .. }
            | Self::Pattern { classification, .. }
            | Self::Failure { classification, .. }
            | Self::Decision { classification, .. }
            | Self::Reference { classification, .. }
            | Self::Guide { classification, .. } => *classification,
        }
    }

    pub fn set_classification(&mut self, new_cls: Classification) {
        match self {
            Self::Convention { classification, .. }
            | Self::Pattern { classification, .. }
            | Self::Failure { classification, .. }
            | Self::Decision { classification, .. }
            | Self::Reference { classification, .. }
            | Self::Guide { classification, .. } => *classification = new_cls,
        }
    }

    pub fn recorded_at(&self) -> &str {
        match self {
            Self::Convention { recorded_at, .. }
            | Self::Pattern { recorded_at, .. }
            | Self::Failure { recorded_at, .. }
            | Self::Decision { recorded_at, .. }
            | Self::Reference { recorded_at, .. }
            | Self::Guide { recorded_at, .. } => recorded_at,
        }
    }

    pub fn evidence(&self) -> Option<&Evidence> {
        match self {
            Self::Convention { evidence, .. }
            | Self::Pattern { evidence, .. }
            | Self::Failure { evidence, .. }
            | Self::Decision { evidence, .. }
            | Self::Reference { evidence, .. }
            | Self::Guide { evidence, .. } => evidence.as_ref(),
        }
    }

    pub fn tags(&self) -> Option<&[String]> {
        match self {
            Self::Convention { tags, .. }
            | Self::Pattern { tags, .. }
            | Self::Failure { tags, .. }
            | Self::Decision { tags, .. }
            | Self::Reference { tags, .. }
            | Self::Guide { tags, .. } => tags.as_deref(),
        }
    }

    pub fn set_tags(&mut self, new_tags: Option<Vec<String>>) {
        match self {
            Self::Convention { tags, .. }
            | Self::Pattern { tags, .. }
            | Self::Failure { tags, .. }
            | Self::Decision { tags, .. }
            | Self::Reference { tags, .. }
            | Self::Guide { tags, .. } => *tags = new_tags,
        }
    }

    pub fn relates_to(&self) -> Option<&[String]> {
        match self {
            Self::Convention { relates_to, .. }
            | Self::Pattern { relates_to, .. }
            | Self::Failure { relates_to, .. }
            | Self::Decision { relates_to, .. }
            | Self::Reference { relates_to, .. }
            | Self::Guide { relates_to, .. } => relates_to.as_deref(),
        }
    }

    pub fn set_relates_to(&mut self, new_val: Option<Vec<String>>) {
        match self {
            Self::Convention { relates_to, .. }
            | Self::Pattern { relates_to, .. }
            | Self::Failure { relates_to, .. }
            | Self::Decision { relates_to, .. }
            | Self::Reference { relates_to, .. }
            | Self::Guide { relates_to, .. } => *relates_to = new_val,
        }
    }

    pub fn supersedes(&self) -> Option<&[String]> {
        match self {
            Self::Convention { supersedes, .. }
            | Self::Pattern { supersedes, .. }
            | Self::Failure { supersedes, .. }
            | Self::Decision { supersedes, .. }
            | Self::Reference { supersedes, .. }
            | Self::Guide { supersedes, .. } => supersedes.as_deref(),
        }
    }

    pub fn set_supersedes(&mut self, new_val: Option<Vec<String>>) {
        match self {
            Self::Convention { supersedes, .. }
            | Self::Pattern { supersedes, .. }
            | Self::Failure { supersedes, .. }
            | Self::Decision { supersedes, .. }
            | Self::Reference { supersedes, .. }
            | Self::Guide { supersedes, .. } => *supersedes = new_val,
        }
    }

    pub fn outcomes(&self) -> Option<&[Outcome]> {
        match self {
            Self::Convention { outcomes, .. }
            | Self::Pattern { outcomes, .. }
            | Self::Failure { outcomes, .. }
            | Self::Decision { outcomes, .. }
            | Self::Reference { outcomes, .. }
            | Self::Guide { outcomes, .. } => outcomes.as_deref(),
        }
    }

    pub fn set_outcomes(&mut self, new_val: Option<Vec<Outcome>>) {
        match self {
            Self::Convention { outcomes, .. }
            | Self::Pattern { outcomes, .. }
            | Self::Failure { outcomes, .. }
            | Self::Decision { outcomes, .. }
            | Self::Reference { outcomes, .. }
            | Self::Guide { outcomes, .. } => *outcomes = new_val,
        }
    }

    pub fn files(&self) -> Option<&[String]> {
        match self {
            Self::Pattern { files, .. } | Self::Reference { files, .. } => files.as_deref(),
            _ => None,
        }
    }

    /// Returns true if this is a "named" type that supports upsert on duplicate.
    pub fn is_named_type(&self) -> bool {
        matches!(
            self,
            Self::Pattern { .. }
                | Self::Decision { .. }
                | Self::Reference { .. }
                | Self::Guide { .. }
        )
    }
}

// ── Config ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShelfLife {
    pub tactical: u32,
    pub observational: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationDefaults {
    pub shelf_life: ShelfLife,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Governance {
    pub max_entries: u32,
    pub warn_entries: u32,
    pub hard_limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KbConfig {
    pub version: String,
    pub domains: Vec<String>,
    pub governance: Governance,
    pub classification_defaults: ClassificationDefaults,
}

impl Default for KbConfig {
    fn default() -> Self {
        Self {
            version: "1".to_string(),
            domains: Vec::new(),
            governance: Governance {
                max_entries: 100,
                warn_entries: 150,
                hard_limit: 200,
            },
            classification_defaults: ClassificationDefaults {
                shelf_life: ShelfLife {
                    tactical: 14,
                    observational: 30,
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convention_round_trip() {
        let json = r#"{"type":"convention","content":"Always use snake_case","classification":"foundational","recorded_at":"2024-01-01T00:00:00.000Z"}"#;
        let record: ExpertiseRecord = serde_json::from_str(json).unwrap();
        assert_eq!(record.record_type(), RecordType::Convention);
        assert_eq!(record.classification(), Classification::Foundational);

        let serialized = serde_json::to_string(&record).unwrap();
        let re_parsed: ExpertiseRecord = serde_json::from_str(&serialized).unwrap();
        assert_eq!(re_parsed.record_type(), RecordType::Convention);
    }

    #[test]
    fn pattern_with_all_fields() {
        let json = r#"{"type":"pattern","id":"mx-abc123","name":"Error Handling","description":"Always wrap in try/catch","files":["src/utils.ts"],"classification":"tactical","recorded_at":"2024-01-01T00:00:00.000Z","tags":["rust","error"],"outcomes":[{"status":"success","agent":"claude"}]}"#;
        let record: ExpertiseRecord = serde_json::from_str(json).unwrap();
        assert_eq!(record.id(), Some("mx-abc123"));
        assert_eq!(record.record_type(), RecordType::Pattern);
        assert!(record.is_named_type());
        assert_eq!(record.tags().unwrap().len(), 2);
        assert_eq!(record.outcomes().unwrap().len(), 1);
    }

    #[test]
    fn config_default() {
        let config = KbConfig::default();
        assert_eq!(config.version, "1");
        assert!(config.domains.is_empty());
        assert_eq!(config.governance.max_entries, 100);
        assert_eq!(config.classification_defaults.shelf_life.tactical, 14);
    }

    #[test]
    fn config_yaml_round_trip() {
        let config = KbConfig::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: KbConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.version, config.version);
        assert_eq!(parsed.governance.max_entries, config.governance.max_entries);
    }
}
