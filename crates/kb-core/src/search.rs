use std::collections::{HashMap, HashSet};

use crate::types::ExpertiseRecord;

/// BM25 tuning parameters.
pub struct Bm25Params {
    /// Term frequency saturation (typical: 1.2-2.0).
    pub k1: f64,
    /// Document length normalization (0 = none, 1 = full).
    pub b: f64,
}

impl Default for Bm25Params {
    fn default() -> Self {
        Self { k1: 1.5, b: 0.75 }
    }
}

/// A search result with score and matched field names.
pub struct Bm25Result<'a> {
    pub record: &'a ExpertiseRecord,
    pub score: f64,
    pub matched_fields: Vec<String>,
}

/// Tokenize text: lowercase, replace punctuation with spaces, split on whitespace.
pub fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c.is_whitespace() {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect()
}

/// Extract searchable text from a record, organized by field.
fn extract_record_text(record: &ExpertiseRecord) -> (String, HashMap<String, String>) {
    let mut field_texts: HashMap<String, String> = HashMap::new();
    let mut all_parts: Vec<String> = Vec::new();

    let add_field = |name: &str,
                     value: &str,
                     field_texts: &mut HashMap<String, String>,
                     all_parts: &mut Vec<String>| {
        if !value.trim().is_empty() {
            field_texts.insert(name.to_string(), value.to_string());
            all_parts.push(value.to_string());
        }
    };

    let add_array_field = |name: &str,
                           values: &[String],
                           field_texts: &mut HashMap<String, String>,
                           all_parts: &mut Vec<String>| {
        let text = values.join(" ");
        if !text.trim().is_empty() {
            field_texts.insert(name.to_string(), text.clone());
            all_parts.push(text);
        }
    };

    match record {
        ExpertiseRecord::Pattern {
            name,
            description,
            files,
            ..
        } => {
            add_field("name", name, &mut field_texts, &mut all_parts);
            add_field("description", description, &mut field_texts, &mut all_parts);
            if let Some(files) = files {
                add_array_field("files", files, &mut field_texts, &mut all_parts);
            }
        }
        ExpertiseRecord::Convention { content, .. } => {
            add_field("content", content, &mut field_texts, &mut all_parts);
        }
        ExpertiseRecord::Failure {
            description,
            resolution,
            ..
        } => {
            add_field("description", description, &mut field_texts, &mut all_parts);
            add_field("resolution", resolution, &mut field_texts, &mut all_parts);
        }
        ExpertiseRecord::Decision {
            title, rationale, ..
        } => {
            add_field("title", title, &mut field_texts, &mut all_parts);
            add_field("rationale", rationale, &mut field_texts, &mut all_parts);
        }
        ExpertiseRecord::Reference {
            name,
            description,
            files,
            ..
        } => {
            add_field("name", name, &mut field_texts, &mut all_parts);
            add_field("description", description, &mut field_texts, &mut all_parts);
            if let Some(files) = files {
                add_array_field("files", files, &mut field_texts, &mut all_parts);
            }
        }
        ExpertiseRecord::Guide {
            name, description, ..
        } => {
            add_field("name", name, &mut field_texts, &mut all_parts);
            add_field("description", description, &mut field_texts, &mut all_parts);
        }
    }

    // Add tags
    if let Some(tags) = record.tags() {
        add_array_field("tags", tags, &mut field_texts, &mut all_parts);
    }

    (all_parts.join(" "), field_texts)
}

/// Calculate term frequency for a list of tokens.
fn calculate_tf(tokens: &[String]) -> HashMap<&str, usize> {
    let mut tf: HashMap<&str, usize> = HashMap::new();
    for token in tokens {
        *tf.entry(token.as_str()).or_default() += 1;
    }
    tf
}

/// Calculate inverse document frequency for all terms in the corpus.
fn calculate_idf(corpus: &[Vec<String>]) -> HashMap<String, f64> {
    let doc_count = corpus.len() as f64;
    let mut doc_freq: HashMap<String, usize> = HashMap::new();

    for doc_tokens in corpus {
        let unique: HashSet<&str> = doc_tokens.iter().map(|s| s.as_str()).collect();
        for term in unique {
            *doc_freq.entry(term.to_string()).or_default() += 1;
        }
    }

    let mut idf = HashMap::new();
    for (term, freq) in &doc_freq {
        let f = *freq as f64;
        idf.insert(term.clone(), ((doc_count - f + 0.5) / (f + 0.5) + 1.0).ln());
    }
    idf
}

/// Calculate BM25 score for a single document against a query.
fn calculate_bm25_score(
    query_tokens: &[String],
    doc_tokens: &[String],
    avg_doc_length: f64,
    idf: &HashMap<String, f64>,
    params: &Bm25Params,
) -> f64 {
    let tf = calculate_tf(doc_tokens);
    let doc_length = doc_tokens.len() as f64;
    let mut score = 0.0;

    for qt in query_tokens {
        let term_freq = *tf.get(qt.as_str()).unwrap_or(&0) as f64;
        let term_idf = *idf.get(qt.as_str()).unwrap_or(&0.0);

        let numerator = term_freq * (params.k1 + 1.0);
        let denominator =
            term_freq + params.k1 * (1.0 - params.b + params.b * (doc_length / avg_doc_length));

        score += term_idf * (numerator / denominator);
    }

    score
}

/// Search records using BM25 ranking. Returns results sorted by score (highest first).
pub fn search_bm25<'a>(
    records: &'a [ExpertiseRecord],
    query: &str,
    params: &Bm25Params,
) -> Vec<Bm25Result<'a>> {
    if records.is_empty() || query.trim().is_empty() {
        return Vec::new();
    }

    let query_tokens = tokenize(query);
    if query_tokens.is_empty() {
        return Vec::new();
    }

    // Extract and tokenize all documents
    let docs: Vec<(String, HashMap<String, String>, Vec<String>)> = records
        .iter()
        .map(|r| {
            let (all_text, field_texts) = extract_record_text(r);
            let tokens = tokenize(&all_text);
            (all_text, field_texts, tokens)
        })
        .collect();

    // Average document length
    let total_length: usize = docs.iter().map(|(_, _, t)| t.len()).sum();
    let avg_doc_length = total_length as f64 / docs.len() as f64;

    // IDF
    let corpus: Vec<Vec<String>> = docs.iter().map(|(_, _, t)| t.clone()).collect();
    let idf = calculate_idf(&corpus);

    // Score each document
    let mut results = Vec::new();
    for (i, (_, field_texts, tokens)) in docs.iter().enumerate() {
        let score = calculate_bm25_score(&query_tokens, tokens, avg_doc_length, &idf, params);

        if score > 0.0 {
            let matched_fields: Vec<String> = field_texts
                .iter()
                .filter(|(_, text)| {
                    let field_tokens = tokenize(text);
                    query_tokens
                        .iter()
                        .any(|qt| field_tokens.iter().any(|ft| ft == qt))
                })
                .map(|(name, _)| name.clone())
                .collect();

            results.push(Bm25Result {
                record: &records[i],
                score,
                matched_fields,
            });
        }
    }

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

/// Search records using default BM25 params. Returns records sorted by relevance.
pub fn search_records<'a>(records: &'a [ExpertiseRecord], query: &str) -> Vec<&'a ExpertiseRecord> {
    search_bm25(records, query, &Bm25Params::default())
        .into_iter()
        .map(|r| r.record)
        .collect()
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

    fn pattern(name: &str, desc: &str) -> ExpertiseRecord {
        ExpertiseRecord::Pattern {
            id: None,
            name: name.to_string(),
            description: desc.to_string(),
            files: None,
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
    fn tokenize_basic() {
        let tokens = tokenize("Hello, World! foo-bar");
        assert_eq!(tokens, vec!["hello", "world", "foo-bar"]);
    }

    #[test]
    fn empty_query_returns_empty() {
        let records = vec![convention("test")];
        let results = search_bm25(&records, "", &Bm25Params::default());
        assert!(results.is_empty());
    }

    #[test]
    fn empty_records_returns_empty() {
        let results = search_bm25(&[], "test", &Bm25Params::default());
        assert!(results.is_empty());
    }

    #[test]
    fn finds_matching_records() {
        let records = vec![
            convention("Always use snake_case for variables"),
            convention("Write tests for all new features"),
            pattern(
                "Error Handling",
                "Use Result type for error handling in Rust",
            ),
        ];
        let results = search_bm25(&records, "error handling", &Bm25Params::default());
        assert!(!results.is_empty());
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn results_sorted_by_score() {
        let records = vec![
            convention("unrelated content here"),
            convention("error error error handling"),
            convention("some error handling approach"),
        ];
        let results = search_bm25(&records, "error handling", &Bm25Params::default());
        for window in results.windows(2) {
            assert!(window[0].score >= window[1].score);
        }
    }
}
