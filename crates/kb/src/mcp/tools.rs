use std::collections::HashMap;
use std::path::Path;

use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};
use rust_mcp_sdk::schema::{CallToolResult, TextContent, schema_utils::CallToolError};
use rust_mcp_sdk::tool_box;

use kb_core::types::*;
use kb_core::{
    access_log, changelog, check, config, filter, format, lock, resolve, search, session, storage,
};

// ── Helper ───────────────────────────────────────────────────────────────────

fn text_result(s: impl Into<String>) -> Result<CallToolResult, CallToolError> {
    Ok(CallToolResult::text_content(vec![TextContent::new(
        s.into(),
        None,
        None,
    )]))
}

fn json_result(v: &serde_json::Value) -> Result<CallToolResult, CallToolError> {
    text_result(serde_json::to_string_pretty(v).unwrap_or_default())
}

fn tool_err(msg: impl Into<String>) -> CallToolError {
    CallToolError::from_message(msg.into())
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn parse_record_type(s: &str) -> Option<RecordType> {
    match s {
        "convention" => Some(RecordType::Convention),
        "pattern" => Some(RecordType::Pattern),
        "failure" => Some(RecordType::Failure),
        "decision" => Some(RecordType::Decision),
        "reference" => Some(RecordType::Reference),
        "guide" => Some(RecordType::Guide),
        _ => None,
    }
}

/// Map anyhow/kb-core errors into CallToolError
fn map_err(e: impl std::fmt::Display) -> CallToolError {
    CallToolError::from_message(format!("{e:#}"))
}

/// Log an access if we have a session
fn log_access(
    cwd: &Path,
    session_id: Option<&str>,
    tool: &str,
    domain: Option<&str>,
    query: Option<&str>,
    entry_id: Option<&str>,
    result_count: Option<usize>,
) {
    let Some(sid) = session_id else { return };
    let _ = access_log::append(
        cwd,
        &access_log::AccessLogEntry {
            session_id: sid.to_string(),
            timestamp: chrono::Utc::now(),
            tool: tool.to_string(),
            domain: domain.map(|s| s.to_string()),
            query: query.map(|s| s.to_string()),
            entry_id: entry_id.map(|s| s.to_string()),
            result_count,
            signal: None,
        },
    );
}

/// Log a mutation to the changelog
fn log_change(
    cwd: &Path,
    session_id: Option<&str>,
    action: &str,
    domain: &str,
    entry_id: &str,
    summary: Option<&str>,
    diff: Option<HashMap<String, (String, String)>>,
) {
    let _ = changelog::append(
        cwd,
        &changelog::ChangelogEntry {
            session_id: session_id.map(|s| s.to_string()),
            timestamp: chrono::Utc::now(),
            action: action.to_string(),
            domain: domain.to_string(),
            entry_id: entry_id.to_string(),
            summary: summary.map(|s| s.to_string()),
            diff,
        },
    );
}

// ── Tool definitions ─────────────────────────────────────────────────────────

#[mcp_tool(
    name = "kb_prime",
    description = "Start a session and prime context. Returns session ID and primed knowledge."
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct KbPrimeTool {
    /// Optional label for this session
    pub label: Option<String>,
    /// Domains to prime (omit for all)
    pub domains: Option<Vec<String>>,
}

impl KbPrimeTool {
    pub fn call_tool(&self, cwd: &Path) -> Result<(CallToolResult, String), CallToolError> {
        let cwd_buf = cwd.to_path_buf();
        config::ensure_kb_dir(&cwd_buf).map_err(map_err)?;
        let cfg = config::read_config(&cwd_buf).map_err(map_err)?;

        let target_domains: Vec<String> = if let Some(ref domains) = self.domains {
            for d in domains {
                config::ensure_domain_exists(&cfg, d).map_err(map_err)?;
            }
            domains.clone()
        } else {
            cfg.domains.clone()
        };

        let mut domain_data: Vec<(String, usize, Vec<ExpertiseRecord>)> = Vec::new();
        for domain in &target_domains {
            let file_path = config::get_expertise_path(domain, &cwd_buf).map_err(map_err)?;
            let records = storage::read_expertise_file(&file_path).map_err(map_err)?;
            let count = records.len();
            domain_data.push((domain.clone(), count, records));
        }

        let mcp_input: Vec<(String, usize, &[ExpertiseRecord])> = domain_data
            .iter()
            .map(|(d, c, recs)| (d.clone(), *c, recs.as_slice()))
            .collect();
        let primed = format::format_mcp_output(&mcp_input);

        let sess = session::start_session(cwd, self.label.as_deref()).map_err(map_err)?;
        let session_id = sess.id.clone();

        // Log the prime action
        access_log::append(
            cwd,
            &access_log::AccessLogEntry {
                session_id: session_id.clone(),
                timestamp: chrono::Utc::now(),
                tool: "prime".to_string(),
                domain: None,
                query: None,
                entry_id: None,
                result_count: Some(domain_data.iter().map(|(_, c, _)| c).sum()),
                signal: None,
            },
        )
        .map_err(map_err)?;

        let result = serde_json::json!({
            "session_id": session_id,
            "label": self.label,
            "domains": target_domains,
            "content": primed,
        });
        Ok((json_result(&result)?, session_id))
    }
}

#[mcp_tool(
    name = "kb_session_resume",
    description = "Resume a previous session by ID."
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct KbSessionResumeTool {
    /// Session ID to resume
    pub session_id: String,
}

impl KbSessionResumeTool {
    pub fn call_tool(&self, cwd: &Path) -> Result<(CallToolResult, String), CallToolError> {
        let sess = session::resume_session(cwd, &self.session_id).map_err(map_err)?;
        let session_id = sess.id.clone();

        access_log::append(
            cwd,
            &access_log::AccessLogEntry {
                session_id: session_id.clone(),
                timestamp: chrono::Utc::now(),
                tool: "session_resume".to_string(),
                domain: None,
                query: None,
                entry_id: None,
                result_count: None,
                signal: None,
            },
        )
        .map_err(map_err)?;

        let result = serde_json::json!({
            "session_id": session_id,
            "label": sess.label,
            "started_at": sess.started_at.to_rfc3339(),
        });
        Ok((json_result(&result)?, session_id))
    }
}

#[mcp_tool(name = "kb_session_end", description = "End the current session.")]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct KbSessionEndTool {}

impl KbSessionEndTool {
    /// Takes current session ID from handler context
    pub fn call_tool(&self, cwd: &Path, session_id: &str) -> Result<CallToolResult, CallToolError> {
        session::end_session(cwd, session_id).map_err(map_err)?;

        access_log::append(
            cwd,
            &access_log::AccessLogEntry {
                session_id: session_id.to_string(),
                timestamp: chrono::Utc::now(),
                tool: "session_end".to_string(),
                domain: None,
                query: None,
                entry_id: None,
                result_count: None,
                signal: None,
            },
        )
        .map_err(map_err)?;

        text_result(format!("Session {session_id} ended."))
    }
}

#[mcp_tool(
    name = "kb_query",
    description = "Query knowledge by domain with optional type filter."
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct KbQueryTool {
    /// Domain to query
    pub domain: String,
    /// Optional record type filter
    pub record_type: Option<String>,
}

impl KbQueryTool {
    pub fn call_tool(
        &self,
        cwd: &Path,
        session_id: Option<&str>,
    ) -> Result<CallToolResult, CallToolError> {
        let cwd_buf = cwd.to_path_buf();
        config::ensure_kb_dir(&cwd_buf).map_err(map_err)?;
        let cfg = config::read_config(&cwd_buf).map_err(map_err)?;
        config::ensure_domain_exists(&cfg, &self.domain).map_err(map_err)?;

        let file_path = config::get_expertise_path(&self.domain, &cwd_buf).map_err(map_err)?;
        let records = storage::read_expertise_file(&file_path).map_err(map_err)?;

        let filtered: Vec<&ExpertiseRecord> = if let Some(ref rt) = self.record_type {
            match parse_record_type(rt) {
                Some(record_type) => filter::filter_by_type(&records, record_type),
                None => return Err(tool_err(format!("Unknown record type: {rt}"))),
            }
        } else {
            records.iter().collect()
        };

        log_access(
            cwd,
            session_id,
            "query",
            Some(&self.domain),
            self.record_type.as_deref(),
            None,
            Some(filtered.len()),
        );

        let result = serde_json::json!({
            "domain": self.domain,
            "count": filtered.len(),
            "records": filtered,
        });
        json_result(&result)
    }
}

#[mcp_tool(name = "kb_query_all", description = "Query all domains.")]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct KbQueryAllTool {}

impl KbQueryAllTool {
    pub fn call_tool(
        &self,
        cwd: &Path,
        session_id: Option<&str>,
    ) -> Result<CallToolResult, CallToolError> {
        let cwd_buf = cwd.to_path_buf();
        config::ensure_kb_dir(&cwd_buf).map_err(map_err)?;
        let cfg = config::read_config(&cwd_buf).map_err(map_err)?;

        let mut domains_json: Vec<serde_json::Value> = Vec::new();
        for domain in &cfg.domains {
            let file_path = config::get_expertise_path(domain, &cwd_buf).map_err(map_err)?;
            let records = storage::read_expertise_file(&file_path).map_err(map_err)?;
            domains_json.push(serde_json::json!({
                "domain": domain,
                "count": records.len(),
                "records": records,
            }));
        }

        let total_count: usize = domains_json
            .iter()
            .filter_map(|d| d.get("count").and_then(|c| c.as_u64()))
            .sum::<u64>() as usize;
        log_access(
            cwd,
            session_id,
            "query_all",
            None,
            None,
            None,
            Some(total_count),
        );

        json_result(&serde_json::json!({ "domains": domains_json }))
    }
}

#[mcp_tool(
    name = "kb_search",
    description = "Full-text search across knowledge base."
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct KbSearchTool {
    /// Search query
    pub query: String,
    /// Limit to specific domain
    pub domain: Option<String>,
    /// Filter by record type
    pub record_type: Option<String>,
}

impl KbSearchTool {
    pub fn call_tool(
        &self,
        cwd: &Path,
        session_id: Option<&str>,
    ) -> Result<CallToolResult, CallToolError> {
        let cwd_buf = cwd.to_path_buf();
        config::ensure_kb_dir(&cwd_buf).map_err(map_err)?;
        let cfg = config::read_config(&cwd_buf).map_err(map_err)?;

        let domains: Vec<String> = if let Some(ref d) = self.domain {
            config::ensure_domain_exists(&cfg, d).map_err(map_err)?;
            vec![d.clone()]
        } else {
            cfg.domains.clone()
        };

        let mut results_json: Vec<serde_json::Value> = Vec::new();
        let mut total: usize = 0;

        for domain in &domains {
            let file_path = config::get_expertise_path(domain, &cwd_buf).map_err(map_err)?;
            let mut records = storage::read_expertise_file(&file_path).map_err(map_err)?;

            if let Some(ref rt) = self.record_type {
                match parse_record_type(rt) {
                    Some(record_type) => records.retain(|r| r.record_type() == record_type),
                    None => return Err(tool_err(format!("Unknown record type: {rt}"))),
                }
            }

            let matches: Vec<&ExpertiseRecord> = search::search_records(&records, &self.query);
            if !matches.is_empty() {
                total += matches.len();
                results_json.push(serde_json::json!({
                    "domain": domain,
                    "matches": matches,
                }));
            }
        }

        log_access(
            cwd,
            session_id,
            "search",
            self.domain.as_deref(),
            Some(&self.query),
            None,
            Some(total),
        );

        json_result(&serde_json::json!({
            "query": self.query,
            "total": total,
            "domains": results_json,
        }))
    }
}

#[mcp_tool(
    name = "kb_record",
    description = "Create a new knowledge entry. Record preferences and rationale, not implementation details discoverable from code. Use stable references, not hardcoded paths."
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct KbRecordTool {
    /// Domain to record in
    pub domain: String,
    /// Record type (convention, pattern, failure, decision, reference, guide)
    pub record_type: String,
    /// Why this approach is preferred, not how it works in code
    pub description: String,
    /// Name (for pattern, reference, guide)
    pub name: Option<String>,
    /// Title (for decision)
    pub title: Option<String>,
    /// Rationale (for decision)
    pub rationale: Option<String>,
    /// Tags
    pub tags: Option<Vec<String>>,
}

impl KbRecordTool {
    pub fn call_tool(
        &self,
        cwd: &Path,
        session_id: Option<&str>,
    ) -> Result<CallToolResult, CallToolError> {
        let cwd_buf = cwd.to_path_buf();
        config::ensure_kb_dir(&cwd_buf).map_err(map_err)?;
        let cfg = config::read_config(&cwd_buf).map_err(map_err)?;
        config::ensure_domain_exists(&cfg, &self.domain).map_err(map_err)?;

        let recorded_at = now_iso();
        let classification = Classification::Tactical;
        let tags = self.tags.clone();

        let record = match self.record_type.as_str() {
            "convention" => ExpertiseRecord::Convention {
                id: None,
                content: self.description.clone(),
                classification,
                recorded_at,
                evidence: None,
                tags,
                relates_to: None,
                supersedes: None,
                outcomes: None,
            },
            "pattern" => {
                let name = self
                    .name
                    .as_deref()
                    .ok_or_else(|| tool_err("Pattern requires 'name'"))?;
                ExpertiseRecord::Pattern {
                    id: None,
                    name: name.to_string(),
                    description: self.description.clone(),
                    files: None,
                    classification,
                    recorded_at,
                    evidence: None,
                    tags,
                    relates_to: None,
                    supersedes: None,
                    outcomes: None,
                }
            }
            "failure" => ExpertiseRecord::Failure {
                id: None,
                description: self.description.clone(),
                resolution: self
                    .rationale
                    .clone()
                    .unwrap_or_else(|| "unresolved".to_string()),
                classification,
                recorded_at,
                evidence: None,
                tags,
                relates_to: None,
                supersedes: None,
                outcomes: None,
            },
            "decision" => {
                let title = self
                    .title
                    .as_deref()
                    .ok_or_else(|| tool_err("Decision requires 'title'"))?;
                let rationale = self
                    .rationale
                    .as_deref()
                    .ok_or_else(|| tool_err("Decision requires 'rationale'"))?;
                ExpertiseRecord::Decision {
                    id: None,
                    title: title.to_string(),
                    rationale: rationale.to_string(),
                    date: None,
                    classification,
                    recorded_at,
                    evidence: None,
                    tags,
                    relates_to: None,
                    supersedes: None,
                    outcomes: None,
                }
            }
            "reference" => {
                let name = self
                    .name
                    .as_deref()
                    .ok_or_else(|| tool_err("Reference requires 'name'"))?;
                ExpertiseRecord::Reference {
                    id: None,
                    name: name.to_string(),
                    description: self.description.clone(),
                    files: None,
                    classification,
                    recorded_at,
                    evidence: None,
                    tags,
                    relates_to: None,
                    supersedes: None,
                    outcomes: None,
                }
            }
            "guide" => {
                let name = self
                    .name
                    .as_deref()
                    .ok_or_else(|| tool_err("Guide requires 'name'"))?;
                ExpertiseRecord::Guide {
                    id: None,
                    name: name.to_string(),
                    description: self.description.clone(),
                    classification,
                    recorded_at,
                    evidence: None,
                    tags,
                    relates_to: None,
                    supersedes: None,
                    outcomes: None,
                }
            }
            other => return Err(tool_err(format!("Unknown record type: {other}"))),
        };

        let file_path = config::get_expertise_path(&self.domain, &cwd_buf).map_err(map_err)?;

        let mut record = record;
        lock::with_file_lock(&file_path, || {
            let existing = storage::read_expertise_file(&file_path)?;
            let dup = filter::find_duplicate(&existing, &record);
            if let Some((idx, _)) = dup {
                if record.is_named_type() {
                    let mut records = existing;
                    records[idx] = record.clone();
                    storage::write_expertise_file(&file_path, &mut records)?;
                    return Ok(());
                }
                return Ok(());
            }
            storage::append_record(&file_path, &mut record)?;
            Ok(())
        })
        .map_err(map_err)?;

        let record_id = record.id().unwrap_or("unknown").to_string();
        log_access(
            cwd,
            session_id,
            "record",
            Some(&self.domain),
            None,
            Some(&record_id),
            None,
        );
        log_change(
            cwd,
            session_id,
            "record",
            &self.domain,
            &record_id,
            Some(&self.description),
            None,
        );

        json_result(&serde_json::json!({
            "success": true,
            "domain": self.domain,
            "type": self.record_type,
            "record": record,
        }))
    }
}

#[mcp_tool(name = "kb_edit", description = "Edit an existing knowledge entry.")]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct KbEditTool {
    /// Domain containing the entry
    pub domain: String,
    /// Record ID (full or prefix)
    pub entry_id: String,
    /// Field updates as key-value pairs
    pub updates: HashMap<String, String>,
}

impl KbEditTool {
    pub fn call_tool(
        &self,
        cwd: &Path,
        session_id: Option<&str>,
    ) -> Result<CallToolResult, CallToolError> {
        let cwd_buf = cwd.to_path_buf();
        config::ensure_kb_dir(&cwd_buf).map_err(map_err)?;
        let cfg = config::read_config(&cwd_buf).map_err(map_err)?;
        config::ensure_domain_exists(&cfg, &self.domain).map_err(map_err)?;

        let file_path = config::get_expertise_path(&self.domain, &cwd_buf).map_err(map_err)?;

        let updated_record = lock::with_file_lock(&file_path, || {
            let mut records = storage::read_expertise_file(&file_path)?;
            let (idx, _) = resolve::resolve_record_id(&records, &self.entry_id)?;

            let record = &mut records[idx];

            for (key, value) in &self.updates {
                match key.as_str() {
                    "content" => {
                        if let ExpertiseRecord::Convention { content, .. } = record {
                            *content = value.clone();
                        }
                    }
                    "name" => match record {
                        ExpertiseRecord::Pattern { name, .. }
                        | ExpertiseRecord::Reference { name, .. }
                        | ExpertiseRecord::Guide { name, .. } => {
                            *name = value.clone();
                        }
                        _ => {}
                    },
                    "description" => match record {
                        ExpertiseRecord::Pattern { description, .. }
                        | ExpertiseRecord::Failure { description, .. }
                        | ExpertiseRecord::Reference { description, .. }
                        | ExpertiseRecord::Guide { description, .. } => {
                            *description = value.clone();
                        }
                        _ => {}
                    },
                    "resolution" => {
                        if let ExpertiseRecord::Failure { resolution, .. } = record {
                            *resolution = value.clone();
                        }
                    }
                    "title" => {
                        if let ExpertiseRecord::Decision { title, .. } = record {
                            *title = value.clone();
                        }
                    }
                    "rationale" => {
                        if let ExpertiseRecord::Decision { rationale, .. } = record {
                            *rationale = value.clone();
                        }
                    }
                    _ => {}
                }
            }

            let updated = records[idx].clone();
            storage::write_expertise_file(&file_path, &mut records)?;
            Ok(updated)
        })
        .map_err(map_err)?;

        let diff: HashMap<String, (String, String)> = self
            .updates
            .iter()
            .map(|(k, v)| (k.clone(), ("...".to_string(), v.clone())))
            .collect();
        log_access(
            cwd,
            session_id,
            "edit",
            Some(&self.domain),
            None,
            Some(&self.entry_id),
            None,
        );
        log_change(
            cwd,
            session_id,
            "edit",
            &self.domain,
            &self.entry_id,
            None,
            Some(diff),
        );

        json_result(&serde_json::json!({
            "success": true,
            "domain": self.domain,
            "id": self.entry_id,
            "record": updated_record,
        }))
    }
}

#[mcp_tool(name = "kb_delete", description = "Delete a knowledge entry.")]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct KbDeleteTool {
    /// Domain containing the entry
    pub domain: String,
    /// Record ID to delete
    pub entry_id: String,
}

impl KbDeleteTool {
    pub fn call_tool(
        &self,
        cwd: &Path,
        session_id: Option<&str>,
    ) -> Result<CallToolResult, CallToolError> {
        let cwd_buf = cwd.to_path_buf();
        config::ensure_kb_dir(&cwd_buf).map_err(map_err)?;
        let cfg = config::read_config(&cwd_buf).map_err(map_err)?;
        config::ensure_domain_exists(&cfg, &self.domain).map_err(map_err)?;

        let file_path = config::get_expertise_path(&self.domain, &cwd_buf).map_err(map_err)?;

        let records = storage::read_expertise_file(&file_path).map_err(map_err)?;
        let (idx, matched) =
            resolve::resolve_record_id(&records, &self.entry_id).map_err(map_err)?;
        let record_id = matched.id().unwrap_or("unknown").to_string();
        let summary = format::get_record_summary(matched);

        lock::with_file_lock(&file_path, || {
            let mut records = storage::read_expertise_file(&file_path)?;
            records.remove(idx);
            storage::write_expertise_file(&file_path, &mut records)?;
            Ok(())
        })
        .map_err(map_err)?;

        log_access(
            cwd,
            session_id,
            "delete",
            Some(&self.domain),
            None,
            Some(&record_id),
            None,
        );
        log_change(
            cwd,
            session_id,
            "delete",
            &self.domain,
            &record_id,
            Some(&summary),
            None,
        );

        json_result(&serde_json::json!({
            "success": true,
            "domain": self.domain,
            "id": record_id,
            "summary": summary,
        }))
    }
}

#[mcp_tool(
    name = "kb_status",
    description = "Get domain listing with record counts."
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct KbStatusTool {}

impl KbStatusTool {
    pub fn call_tool(&self, cwd: &Path) -> Result<CallToolResult, CallToolError> {
        let cwd_buf = cwd.to_path_buf();
        config::ensure_kb_dir(&cwd_buf).map_err(map_err)?;
        let cfg = config::read_config(&cwd_buf).map_err(map_err)?;

        let mut domains: Vec<serde_json::Value> = Vec::new();
        for domain in &cfg.domains {
            let file_path = config::get_expertise_path(domain, &cwd_buf).map_err(map_err)?;
            let records = storage::read_expertise_file(&file_path).map_err(map_err)?;
            let last_updated = records.iter().map(|r| r.recorded_at().to_string()).max();
            domains.push(serde_json::json!({
                "domain": domain,
                "count": records.len(),
                "last_updated": last_updated,
            }));
        }

        json_result(&serde_json::json!({ "domains": domains }))
    }
}

#[mcp_tool(
    name = "kb_oracle",
    description = "Record a knowledge gap. Returns empty but logs what was asked."
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct KbOracleTool {
    /// The query that revealed a gap
    pub query: String,
    /// Domain the gap relates to
    pub domain: Option<String>,
}

impl KbOracleTool {
    pub fn call_tool(
        &self,
        cwd: &Path,
        session_id: Option<&str>,
    ) -> Result<CallToolResult, CallToolError> {
        log_access(
            cwd,
            session_id,
            "oracle",
            self.domain.as_deref(),
            Some(&self.query),
            None,
            Some(0),
        );
        text_result("Knowledge gap recorded.")
    }
}

#[mcp_tool(
    name = "kb_feedback",
    description = "Submit helpful/not-helpful signal on a knowledge entry."
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct KbFeedbackTool {
    /// Record ID to provide feedback on
    pub entry_id: String,
    /// Signal: "helpful" or "not-helpful"
    pub signal: String,
    /// Domain (optional, searches all if omitted)
    pub domain: Option<String>,
}

impl KbFeedbackTool {
    pub fn call_tool(
        &self,
        cwd: &Path,
        session_id: Option<&str>,
    ) -> Result<CallToolResult, CallToolError> {
        if let Some(sid) = session_id {
            let _ = access_log::append(
                cwd,
                &access_log::AccessLogEntry {
                    session_id: sid.to_string(),
                    timestamp: chrono::Utc::now(),
                    tool: "feedback".to_string(),
                    domain: self.domain.clone(),
                    query: None,
                    entry_id: Some(self.entry_id.clone()),
                    result_count: None,
                    signal: Some(self.signal.clone()),
                },
            );
        }
        text_result(format!(
            "Feedback '{}' recorded for entry {}.",
            self.signal, self.entry_id
        ))
    }
}

#[mcp_tool(
    name = "kb_check",
    description = "Validate path/pattern references against working tree."
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct KbCheckTool {
    /// Domain to check (omit for all)
    pub domain: Option<String>,
}

impl KbCheckTool {
    pub fn call_tool(
        &self,
        cwd: &Path,
        session_id: Option<&str>,
    ) -> Result<CallToolResult, CallToolError> {
        let results = check::check_references(cwd, self.domain.as_deref()).map_err(map_err)?;
        log_access(
            cwd,
            session_id,
            "check",
            self.domain.as_deref(),
            None,
            None,
            Some(results.len()),
        );
        json_result(&serde_json::json!({
            "broken_count": results.len(),
            "results": results.iter().map(|r| serde_json::json!({
                "domain": r.domain,
                "entry_id": r.entry_id,
                "summary": r.entry_summary,
                "broken_refs": r.broken_refs,
            })).collect::<Vec<_>>(),
        }))
    }
}

// ── Toolbox ──────────────────────────────────────────────────────────────────

tool_box!(
    KbTools,
    [
        KbPrimeTool,
        KbSessionResumeTool,
        KbSessionEndTool,
        KbQueryTool,
        KbQueryAllTool,
        KbSearchTool,
        KbRecordTool,
        KbEditTool,
        KbDeleteTool,
        KbStatusTool,
        KbOracleTool,
        KbFeedbackTool,
        KbCheckTool
    ]
);
