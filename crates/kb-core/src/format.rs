use crate::types::{ExpertiseRecord, Outcome};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimeFormat {
    Markdown,
    Xml,
    Plain,
}

// ── Helpers ────────────────────────────────────────────────────────────────

pub fn format_time_ago(recorded_at: &str) -> String {
    let parsed = match chrono::DateTime::parse_from_rfc3339(recorded_at) {
        Ok(dt) => dt.with_timezone(&chrono::Utc),
        Err(_) => return "unknown".to_string(),
    };
    let now = chrono::Utc::now();
    let diff = now.signed_duration_since(parsed);

    let mins = diff.num_minutes();
    let hours = diff.num_hours();
    let days = diff.num_days();

    if mins < 1 {
        "just now".to_string()
    } else if mins < 60 {
        format!("{mins}m ago")
    } else if hours < 24 {
        format!("{hours}h ago")
    } else {
        format!("{days}d ago")
    }
}

fn format_evidence(record: &ExpertiseRecord) -> String {
    let evidence = match record.evidence() {
        Some(e) => e,
        None => return String::new(),
    };
    let mut parts = Vec::new();
    if let Some(ref c) = evidence.commit {
        parts.push(format!("commit: {c}"));
    }
    if let Some(ref d) = evidence.date {
        parts.push(format!("date: {d}"));
    }
    if let Some(ref i) = evidence.issue {
        parts.push(format!("issue: {i}"));
    }
    if let Some(ref f) = evidence.file {
        parts.push(format!("file: {f}"));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" [{}]", parts.join(", "))
    }
}

fn format_outcome(outcomes: Option<&[Outcome]>) -> String {
    let outcomes = match outcomes {
        Some(o) if !o.is_empty() => o,
        _ => return String::new(),
    };
    let latest = &outcomes[outcomes.len() - 1];
    let symbol = match latest.status {
        crate::types::OutcomeStatus::Success => "\u{2713}",
        crate::types::OutcomeStatus::Partial => "~",
        crate::types::OutcomeStatus::Failure => "\u{2717}",
    };
    let mut parts = vec![symbol.to_string()];
    if let Some(d) = latest.duration {
        parts.push(format!("{d}ms"));
    }
    if let Some(ref a) = latest.agent {
        parts.push(format!("@{a}"));
    }
    if outcomes.len() > 1 {
        parts.push(format!("({}x)", outcomes.len()));
    }
    format!(" [{}]", parts.join(" "))
}

fn format_links(record: &ExpertiseRecord) -> String {
    let mut parts = Vec::new();
    if let Some(relates) = record.relates_to() {
        if !relates.is_empty() {
            parts.push(format!("relates to: {}", relates.join(", ")));
        }
    }
    if let Some(supersedes) = record.supersedes() {
        if !supersedes.is_empty() {
            parts.push(format!("supersedes: {}", supersedes.join(", ")));
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" [{}]", parts.join("; "))
    }
}

fn format_record_meta(record: &ExpertiseRecord, full: bool) -> String {
    if !full {
        return format_links(record);
    }
    let mut parts = vec![format!(
        "({}){}",
        record.classification(),
        format_evidence(record)
    )];
    if let Some(tags) = record.tags() {
        if !tags.is_empty() {
            parts.push(format!("[tags: {}]", tags.join(", ")));
        }
    }
    format!(" {}{}", parts.join(" "), format_links(record))
}

fn id_tag(record: &ExpertiseRecord) -> String {
    match record.id() {
        Some(id) => format!("[{id}] "),
        None => String::new(),
    }
}

fn truncate(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        return text.to_string();
    }
    // Try to cut at first sentence boundary (punctuation followed by whitespace)
    let bytes = text.as_bytes();
    for i in 0..max_len {
        if (bytes[i] == b'.' || bytes[i] == b'!' || bytes[i] == b'?')
            && i + 1 < bytes.len()
            && bytes[i + 1].is_ascii_whitespace()
        {
            return text[..=i].to_string();
        }
    }
    format!("{}...", &text[..max_len])
}

/// Get a short summary string for a record.
pub fn get_record_summary(record: &ExpertiseRecord) -> String {
    match record {
        ExpertiseRecord::Convention { content, .. } => truncate(content, 60),
        ExpertiseRecord::Pattern { name, .. } => name.clone(),
        ExpertiseRecord::Failure { description, .. } => truncate(description, 60),
        ExpertiseRecord::Decision { title, .. } => title.clone(),
        ExpertiseRecord::Reference { name, .. } => name.clone(),
        ExpertiseRecord::Guide { name, .. } => name.clone(),
    }
}

// ── Compact format ─────────────────────────────────────────────────────────

fn compact_id(record: &ExpertiseRecord) -> String {
    match record.id() {
        Some(id) => format!(" ({id})"),
        None => String::new(),
    }
}

fn compact_line(record: &ExpertiseRecord) -> String {
    let links = format_links(record);
    let id = compact_id(record);
    let outcome = format_outcome(record.outcomes());

    match record {
        ExpertiseRecord::Convention { content, .. } => {
            format!(
                "- [convention] {}{id}{outcome}{links}",
                truncate(content, 100)
            )
        }
        ExpertiseRecord::Pattern {
            name,
            description,
            files,
            ..
        } => {
            let f = files
                .as_ref()
                .filter(|f| !f.is_empty())
                .map(|f| format!(" ({})", f.join(", ")))
                .unwrap_or_default();
            format!(
                "- [pattern] {name}: {}{f}{id}{outcome}{links}",
                truncate(description, 100)
            )
        }
        ExpertiseRecord::Failure {
            description,
            resolution,
            ..
        } => {
            format!(
                "- [failure] {} \u{2192} {}{id}{outcome}{links}",
                truncate(description, 100),
                truncate(resolution, 100)
            )
        }
        ExpertiseRecord::Decision {
            title, rationale, ..
        } => {
            format!(
                "- [decision] {title}: {}{id}{outcome}{links}",
                truncate(rationale, 100)
            )
        }
        ExpertiseRecord::Reference {
            name,
            description,
            files,
            ..
        } => {
            let detail = files
                .as_ref()
                .filter(|f| !f.is_empty())
                .map(|f| format!(": {}", f.join(", ")))
                .unwrap_or_else(|| format!(": {}", truncate(description, 100)));
            format!("- [reference] {name}{detail}{id}{outcome}{links}")
        }
        ExpertiseRecord::Guide {
            name, description, ..
        } => {
            format!(
                "- [guide] {name}: {}{id}{outcome}{links}",
                truncate(description, 100)
            )
        }
    }
}

// ── Markdown format ────────────────────────────────────────────────────────

fn format_type_section(
    header: &str,
    records: &[&ExpertiseRecord],
    full: bool,
    line_fn: impl Fn(&ExpertiseRecord, bool) -> String,
) -> String {
    if records.is_empty() {
        return String::new();
    }
    let mut lines = vec![format!("### {header}")];
    for r in records {
        lines.push(line_fn(r, full));
    }
    lines.join("\n")
}

pub fn format_domain_expertise(
    domain: &str,
    records: &[ExpertiseRecord],
    last_updated: Option<&str>,
    full: bool,
) -> String {
    let updated_str = last_updated
        .map(|ts| format!(", updated {}", format_time_ago(ts)))
        .unwrap_or_default();
    let mut lines = vec![format!(
        "## {domain} ({} records{updated_str})",
        records.len()
    )];
    lines.push(String::new());

    let sections: Vec<String> = [
        format_type_section(
            "Conventions",
            &records
                .iter()
                .filter(|r| matches!(r, ExpertiseRecord::Convention { .. }))
                .collect::<Vec<_>>(),
            full,
            |r, full| {
                if let ExpertiseRecord::Convention { content, .. } = r {
                    format!("- {}{content}{}", id_tag(r), format_record_meta(r, full))
                } else {
                    String::new()
                }
            },
        ),
        format_type_section(
            "Patterns",
            &records
                .iter()
                .filter(|r| matches!(r, ExpertiseRecord::Pattern { .. }))
                .collect::<Vec<_>>(),
            full,
            |r, full| {
                if let ExpertiseRecord::Pattern {
                    name,
                    description,
                    files,
                    ..
                } = r
                {
                    let f = files
                        .as_ref()
                        .filter(|f| !f.is_empty())
                        .map(|f| format!(" ({})", f.join(", ")))
                        .unwrap_or_default();
                    format!(
                        "- {}**{name}**: {description}{f}{}",
                        id_tag(r),
                        format_record_meta(r, full)
                    )
                } else {
                    String::new()
                }
            },
        ),
        format_type_section(
            "Known Failures",
            &records
                .iter()
                .filter(|r| matches!(r, ExpertiseRecord::Failure { .. }))
                .collect::<Vec<_>>(),
            full,
            |r, full| {
                if let ExpertiseRecord::Failure {
                    description,
                    resolution,
                    ..
                } = r
                {
                    format!(
                        "- {}{description}{}\n  \u{2192} {resolution}",
                        id_tag(r),
                        format_record_meta(r, full)
                    )
                } else {
                    String::new()
                }
            },
        ),
        format_type_section(
            "Decisions",
            &records
                .iter()
                .filter(|r| matches!(r, ExpertiseRecord::Decision { .. }))
                .collect::<Vec<_>>(),
            full,
            |r, full| {
                if let ExpertiseRecord::Decision {
                    title, rationale, ..
                } = r
                {
                    format!(
                        "- {}**{title}**: {rationale}{}",
                        id_tag(r),
                        format_record_meta(r, full)
                    )
                } else {
                    String::new()
                }
            },
        ),
        format_type_section(
            "References",
            &records
                .iter()
                .filter(|r| matches!(r, ExpertiseRecord::Reference { .. }))
                .collect::<Vec<_>>(),
            full,
            |r, full| {
                if let ExpertiseRecord::Reference {
                    name,
                    description,
                    files,
                    ..
                } = r
                {
                    let f = files
                        .as_ref()
                        .filter(|f| !f.is_empty())
                        .map(|f| format!(" ({})", f.join(", ")))
                        .unwrap_or_default();
                    format!(
                        "- {}**{name}**: {description}{f}{}",
                        id_tag(r),
                        format_record_meta(r, full)
                    )
                } else {
                    String::new()
                }
            },
        ),
        format_type_section(
            "Guides",
            &records
                .iter()
                .filter(|r| matches!(r, ExpertiseRecord::Guide { .. }))
                .collect::<Vec<_>>(),
            full,
            |r, full| {
                if let ExpertiseRecord::Guide {
                    name, description, ..
                } = r
                {
                    format!(
                        "- {}**{name}**: {description}{}",
                        id_tag(r),
                        format_record_meta(r, full)
                    )
                } else {
                    String::new()
                }
            },
        ),
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect();

    lines.push(sections.join("\n\n"));
    lines.join("\n")
}

pub fn format_domain_expertise_compact(
    domain: &str,
    records: &[ExpertiseRecord],
    last_updated: Option<&str>,
) -> String {
    let updated_str = last_updated
        .map(|ts| format!(", updated {}", format_time_ago(ts)))
        .unwrap_or_default();
    let mut lines = vec![format!(
        "## {domain} ({} records{updated_str})",
        records.len()
    )];
    for r in records {
        lines.push(compact_line(r));
    }
    lines.join("\n")
}

pub fn format_prime_output(domain_sections: &[String]) -> String {
    let mut lines = vec![
        "# Project Expertise (via KB)".to_string(),
        String::new(),
        "> **Context Recovery**: Run `kb prime` after compaction, clear, or new session".to_string(),
        String::new(),
        "## Rules".to_string(),
        String::new(),
        "- **Record learnings**: When you discover a pattern, fix a bug, or make a design decision — record it with `kb record`".to_string(),
        "- **Check expertise first**: Before implementing, check if relevant expertise exists with `kb search` or `kb prime --context`".to_string(),
        "- **Targeted priming**: Use `kb prime --files src/foo.ts` to load only records relevant to specific files".to_string(),
        "- **Do NOT** store expertise in code comments, markdown files, or memory tools — use `kb record`".to_string(),
        "- **Do NOT** record implementation details discoverable from code — record which approach is preferred and why".to_string(),
        "- **Do NOT** hardcode file paths or line numbers that will change — use stable references (doc files, module names, config keys)".to_string(),
        "- Run `kb doctor` if you are unsure whether records are healthy".to_string(),
        String::new(),
    ];

    if domain_sections.is_empty() {
        lines.push("No expertise recorded yet. Use `kb add <domain>` to create a domain, then `kb record` to add records.".to_string());
        lines.push(String::new());
    } else {
        lines.push(domain_sections.join("\n\n"));
        lines.push(String::new());
    }

    lines.push(String::new());
    lines.push("## Recording New Learnings".to_string());
    lines.push(String::new());
    lines.push(
        "When you discover a pattern, convention, failure, or make an architectural decision:"
            .to_string(),
    );
    lines.push(String::new());
    lines.push("```bash".to_string());
    lines.push("kb record <domain> --type convention \"description\"".to_string());
    lines.push(
        "kb record <domain> --type failure --description \"...\" --resolution \"...\"".to_string(),
    );
    lines
        .push("kb record <domain> --type decision --title \"...\" --rationale \"...\"".to_string());
    lines.push(
        "kb record <domain> --type pattern --name \"...\" --description \"...\" --files \"...\""
            .to_string(),
    );
    lines.push(
        "kb record <domain> --type reference --name \"...\" --description \"...\" --files \"...\""
            .to_string(),
    );
    lines.push("kb record <domain> --type guide --name \"...\" --description \"...\"".to_string());
    lines.push("```".to_string());
    lines.push(String::new());
    lines.push("**Link evidence** to records when available:".to_string());
    lines.push(String::new());
    lines.push("```bash".to_string());
    lines.push("kb record <domain> --type pattern --name \"...\" --description \"...\" --evidence-commit abc123".to_string());
    lines.push("kb record <domain> --type decision --title \"...\" --rationale \"...\" --evidence-bead beads-xxx".to_string());
    lines.push("```".to_string());
    lines.push(String::new());
    lines.push("**Batch record** multiple records at once:".to_string());
    lines.push(String::new());
    lines.push("```bash".to_string());
    lines.push("kb record <domain> --batch records.json  # from file".to_string());
    lines.push("echo '[{\"type\":\"convention\",\"content\":\"...\"}]' | kb record <domain> --stdin  # from stdin".to_string());
    lines.push("```".to_string());
    lines.push(String::new());
    lines.push("## Searching Expertise".to_string());
    lines.push(String::new());
    lines.push("Use `kb search` to find relevant records across all domains. Results are ranked by relevance (BM25):".to_string());
    lines.push(String::new());
    lines.push("```bash".to_string());
    lines.push(
        "kb search \"file locking\"              # multi-word queries ranked by relevance"
            .to_string(),
    );
    lines.push("kb search \"atomic\" --domain cli        # limit to a specific domain".to_string());
    lines.push("kb search \"ESM\" --type convention      # filter by record type".to_string());
    lines.push("kb search \"concurrency\" --tag safety   # filter by tag".to_string());
    lines.push("```".to_string());
    lines.push(String::new());
    lines.push(
        "Search before implementing — existing expertise may already cover your use case."
            .to_string(),
    );
    lines.push(String::new());
    lines.push("## Domain Maintenance".to_string());
    lines.push(String::new());
    lines.push("When a domain grows large, compact it to keep expertise focused:".to_string());
    lines.push(String::new());
    lines.push("```bash".to_string());
    lines.push("kb compact --auto --dry-run     # preview what would be merged".to_string());
    lines.push("kb compact --auto               # merge same-type record groups".to_string());
    lines.push("```".to_string());
    lines.push(String::new());
    lines.push("Use `kb diff` to review what expertise changed:".to_string());
    lines.push(String::new());
    lines.push("```bash".to_string());
    lines.push(
        "kb diff HEAD~3                  # see record changes over last 3 commits".to_string(),
    );
    lines.push("```".to_string());
    lines.push(String::new());
    lines.push("## Session End".to_string());
    lines.push(String::new());
    lines.push(
        "**IMPORTANT**: Before ending your session, record what you learned and sync:".to_string(),
    );
    lines.push(String::new());
    lines.push("```".to_string());
    lines
        .push("[ ] kb learn          # see what files changed — decide what to record".to_string());
    lines.push("[ ] kb record ...     # record learnings (see above)".to_string());
    lines.push("[ ] kb sync           # validate, stage, and commit .kb/ changes".to_string());
    lines.push("```".to_string());
    lines.push(String::new());
    lines.push("Do NOT skip this. Unrecorded learnings are lost for the next session.".to_string());

    lines.join("\n")
}

pub fn format_prime_output_compact(domain_sections: &[String]) -> String {
    let mut lines = Vec::new();
    lines.push("# Project Expertise (via KB)".to_string());
    lines.push(String::new());
    if domain_sections.is_empty() {
        lines.push("No expertise recorded yet. Use `kb add <domain>` to create a domain, then `kb record` to add records.".to_string());
    } else {
        lines.push(domain_sections.join("\n\n"));
    }
    lines.push(String::new());
    lines.push("## Quick Reference".to_string());
    lines.push(String::new());
    lines.push(
        "- `kb search \"query\"` \u{2014} find relevant records before implementing".to_string(),
    );
    lines.push(
        "- `kb prime --files src/foo.ts` \u{2014} load records for specific files".to_string(),
    );
    lines.push("- `kb prime --context` \u{2014} load records for git-changed files".to_string());
    lines.push("- `kb record <domain> --type <type> --description \"...\"`".to_string());
    lines.push(
        "  - Types: `convention`, `pattern`, `failure`, `decision`, `reference`, `guide`"
            .to_string(),
    );
    lines.push("  - Evidence: `--evidence-commit <sha>`, `--evidence-bead <id>`".to_string());
    lines.push("- `kb doctor` \u{2014} check record health".to_string());
    lines.join("\n")
}

// ── XML format ─────────────────────────────────────────────────────────────

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub fn format_domain_expertise_xml(
    domain: &str,
    records: &[ExpertiseRecord],
    last_updated: Option<&str>,
) -> String {
    let updated_attr = last_updated
        .map(|ts| format!(" updated=\"{}\"", format_time_ago(ts)))
        .unwrap_or_default();
    let mut lines = vec![format!(
        "<domain name=\"{}\" entries=\"{}\"{updated_attr}>",
        xml_escape(domain),
        records.len()
    )];

    for r in records {
        let id_attr = r
            .id()
            .map(|id| format!(" id=\"{}\"", xml_escape(id)))
            .unwrap_or_default();
        let type_str = r.record_type().as_str();
        lines.push(format!(
            "  <{type_str}{id_attr} classification=\"{}\">",
            r.classification()
        ));

        match r {
            ExpertiseRecord::Convention { content, .. } => {
                lines.push(format!("    {}", xml_escape(content)));
            }
            ExpertiseRecord::Pattern {
                name,
                description,
                files,
                ..
            } => {
                lines.push(format!("    <name>{}</name>", xml_escape(name)));
                lines.push(format!(
                    "    <description>{}</description>",
                    xml_escape(description)
                ));
                if let Some(files) = files.as_ref().filter(|f| !f.is_empty()) {
                    lines.push(format!(
                        "    <files>{}</files>",
                        files
                            .iter()
                            .map(|f| xml_escape(f))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
            }
            ExpertiseRecord::Failure {
                description,
                resolution,
                ..
            } => {
                lines.push(format!(
                    "    <description>{}</description>",
                    xml_escape(description)
                ));
                lines.push(format!(
                    "    <resolution>{}</resolution>",
                    xml_escape(resolution)
                ));
            }
            ExpertiseRecord::Decision {
                title, rationale, ..
            } => {
                lines.push(format!("    <title>{}</title>", xml_escape(title)));
                lines.push(format!(
                    "    <rationale>{}</rationale>",
                    xml_escape(rationale)
                ));
            }
            ExpertiseRecord::Reference {
                name,
                description,
                files,
                ..
            } => {
                lines.push(format!("    <name>{}</name>", xml_escape(name)));
                lines.push(format!(
                    "    <description>{}</description>",
                    xml_escape(description)
                ));
                if let Some(files) = files.as_ref().filter(|f| !f.is_empty()) {
                    lines.push(format!(
                        "    <files>{}</files>",
                        files
                            .iter()
                            .map(|f| xml_escape(f))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
            }
            ExpertiseRecord::Guide {
                name, description, ..
            } => {
                lines.push(format!("    <name>{}</name>", xml_escape(name)));
                lines.push(format!(
                    "    <description>{}</description>",
                    xml_escape(description)
                ));
            }
        }

        if let Some(tags) = r.tags() {
            if !tags.is_empty() {
                lines.push(format!(
                    "    <tags>{}</tags>",
                    tags.iter()
                        .map(|t| xml_escape(t))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
        if let Some(relates) = r.relates_to() {
            if !relates.is_empty() {
                lines.push(format!(
                    "    <relates_to>{}</relates_to>",
                    relates.join(", ")
                ));
            }
        }
        if let Some(supersedes) = r.supersedes() {
            if !supersedes.is_empty() {
                lines.push(format!(
                    "    <supersedes>{}</supersedes>",
                    supersedes.join(", ")
                ));
            }
        }
        if let Some(outcomes) = r.outcomes() {
            for o in outcomes {
                let dur = o
                    .duration
                    .map(|d| format!(" duration=\"{d}\""))
                    .unwrap_or_default();
                let agent = o
                    .agent
                    .as_ref()
                    .map(|a| format!(" agent=\"{}\"", xml_escape(a)))
                    .unwrap_or_default();
                let content = o
                    .test_results
                    .as_ref()
                    .map(|t| xml_escape(t))
                    .unwrap_or_default();
                lines.push(format!(
                    "    <outcome status=\"{}\"{dur}{agent}>{content}</outcome>",
                    o.status
                ));
            }
        }
        lines.push(format!("  </{type_str}>"));
    }

    lines.push("</domain>".to_string());
    lines.join("\n")
}

pub fn format_prime_output_xml(domain_sections: &[String]) -> String {
    let mut lines = vec!["<expertise>".to_string()];
    if domain_sections.is_empty() {
        lines.push(
            "  <empty>No expertise recorded yet. Use kb add and kb record to get started.</empty>"
                .to_string(),
        );
    } else {
        lines.push(domain_sections.join("\n"));
    }
    lines.push("</expertise>".to_string());
    lines.join("\n")
}

// ── Plain text format ──────────────────────────────────────────────────────

pub fn format_domain_expertise_plain(
    domain: &str,
    records: &[ExpertiseRecord],
    last_updated: Option<&str>,
) -> String {
    let updated_str = last_updated
        .map(|ts| format!(" (updated {})", format_time_ago(ts)))
        .unwrap_or_default();
    let mut lines = vec![format!("[{domain}] {} records{updated_str}", records.len())];
    lines.push(String::new());

    fn by_type(
        records: &[ExpertiseRecord],
        filter: fn(&ExpertiseRecord) -> bool,
    ) -> Vec<&ExpertiseRecord> {
        records.iter().filter(|r| filter(r)).collect()
    }

    let conventions = by_type(records, |r| matches!(r, ExpertiseRecord::Convention { .. }));
    let patterns = by_type(records, |r| matches!(r, ExpertiseRecord::Pattern { .. }));
    let failures = by_type(records, |r| matches!(r, ExpertiseRecord::Failure { .. }));
    let decisions = by_type(records, |r| matches!(r, ExpertiseRecord::Decision { .. }));
    let references = by_type(records, |r| matches!(r, ExpertiseRecord::Reference { .. }));
    let guides = by_type(records, |r| matches!(r, ExpertiseRecord::Guide { .. }));

    if !conventions.is_empty() {
        lines.push("Conventions:".to_string());
        for r in &conventions {
            if let ExpertiseRecord::Convention { content, .. } = r {
                lines.push(format!("  - {}{content}{}", id_tag(r), format_links(r)));
            }
        }
        lines.push(String::new());
    }
    if !patterns.is_empty() {
        lines.push("Patterns:".to_string());
        for r in &patterns {
            if let ExpertiseRecord::Pattern {
                name,
                description,
                files,
                ..
            } = r
            {
                let f = files
                    .as_ref()
                    .filter(|f| !f.is_empty())
                    .map(|f| format!(" ({})", f.join(", ")))
                    .unwrap_or_default();
                lines.push(format!(
                    "  - {}{name}: {description}{f}{}",
                    id_tag(r),
                    format_links(r)
                ));
            }
        }
        lines.push(String::new());
    }
    if !failures.is_empty() {
        lines.push("Known Failures:".to_string());
        for r in &failures {
            if let ExpertiseRecord::Failure {
                description,
                resolution,
                ..
            } = r
            {
                lines.push(format!("  - {}{description}{}", id_tag(r), format_links(r)));
                lines.push(format!("    Fix: {resolution}"));
            }
        }
        lines.push(String::new());
    }
    if !decisions.is_empty() {
        lines.push("Decisions:".to_string());
        for r in &decisions {
            if let ExpertiseRecord::Decision {
                title, rationale, ..
            } = r
            {
                lines.push(format!(
                    "  - {}{title}: {rationale}{}",
                    id_tag(r),
                    format_links(r)
                ));
            }
        }
        lines.push(String::new());
    }
    if !references.is_empty() {
        lines.push("References:".to_string());
        for r in &references {
            if let ExpertiseRecord::Reference {
                name,
                description,
                files,
                ..
            } = r
            {
                let f = files
                    .as_ref()
                    .filter(|f| !f.is_empty())
                    .map(|f| format!(" ({})", f.join(", ")))
                    .unwrap_or_default();
                lines.push(format!(
                    "  - {}{name}: {description}{f}{}",
                    id_tag(r),
                    format_links(r)
                ));
            }
        }
        lines.push(String::new());
    }
    if !guides.is_empty() {
        lines.push("Guides:".to_string());
        for r in &guides {
            if let ExpertiseRecord::Guide {
                name, description, ..
            } = r
            {
                lines.push(format!(
                    "  - {}{name}: {description}{}",
                    id_tag(r),
                    format_links(r)
                ));
            }
        }
        lines.push(String::new());
    }

    lines.join("\n").trim_end().to_string()
}

pub fn format_prime_output_plain(domain_sections: &[String]) -> String {
    let mut lines = Vec::new();
    lines.push("Project Expertise (via KB)".to_string());
    lines.push("============================".to_string());
    lines.push(String::new());
    if domain_sections.is_empty() {
        lines.push(
            "No expertise recorded yet. Use `kb add <domain>` and `kb record` to get started."
                .to_string(),
        );
    } else {
        lines.push(domain_sections.join("\n\n"));
    }
    lines.join("\n")
}

// ── MCP format ─────────────────────────────────────────────────────────────

pub fn format_mcp_output(domains: &[(String, usize, &[ExpertiseRecord])]) -> String {
    let domain_objs: Vec<serde_json::Value> = domains
        .iter()
        .map(|(domain, count, records)| {
            serde_json::json!({
                "domain": domain,
                "entry_count": count,
                "records": records,
            })
        })
        .collect();
    serde_json::json!({ "type": "expertise", "domains": domain_objs }).to_string()
}

// ── Session end reminder ───────────────────────────────────────────────────

pub fn get_session_end_reminder(format: PrimeFormat) -> String {
    match format {
        PrimeFormat::Xml => [
            "<session_close_protocol priority=\"critical\">",
            "  <instruction>Before saying done or complete, you MUST run this checklist:</instruction>",
            "  <checklist>",
            "    <step>kb learn — see what files changed, decide what to record</step>",
            "    <step>kb record &lt;domain&gt; --type &lt;type&gt; --description &quot;...&quot;</step>",
            "    <step>kb sync — validate, stage, and commit .kb/ changes</step>",
            "  </checklist>",
            "  <warning>NEVER skip this. Unrecorded learnings are lost for the next session.</warning>",
            "</session_close_protocol>",
        ].join("\n"),
        PrimeFormat::Plain => [
            "=== SESSION CLOSE PROTOCOL (CRITICAL) ===",
            "",
            "Before saying \"done\" or \"complete\", you MUST run this checklist:",
            "",
            "[ ] 1. kb learn              (see what files changed — decide what to record)",
            "[ ] 2. kb record <domain> --type <type> --description \"...\"",
            "[ ] 3. kb sync               (validate, stage, and commit .kb/ changes)",
            "",
            "NEVER skip this. Unrecorded learnings are lost for the next session.",
        ].join("\n"),
        PrimeFormat::Markdown => [
            "# \u{1F6A8} SESSION CLOSE PROTOCOL \u{1F6A8}",
            "",
            "**CRITICAL**: Before saying \"done\" or \"complete\", you MUST run this checklist:",
            "",
            "```",
            "[ ] 1. kb learn              # see what files changed — decide what to record",
            "[ ] 2. kb record <domain> --type <type> --description \"...\"",
            "[ ] 3. kb sync               # validate, stage, and commit .kb/ changes",
            "```",
            "",
            "**NEVER skip this.** Unrecorded learnings are lost for the next session.",
        ].join("\n"),
    }
}

// ── Status output ──────────────────────────────────────────────────────────

pub struct DomainStat {
    pub domain: String,
    pub count: usize,
    pub last_updated: Option<String>,
}

pub fn format_status_output(stats: &[DomainStat], governance: &crate::types::Governance) -> String {
    let mut lines = Vec::new();
    lines.push("KB Status".to_string());
    lines.push("============".to_string());
    lines.push(String::new());

    if stats.is_empty() {
        lines.push("No domains configured. Run `kb add <domain>` to get started.".to_string());
        return lines.join("\n");
    }

    for stat in stats {
        let updated = stat
            .last_updated
            .as_ref()
            .map(|ts| format_time_ago(ts))
            .unwrap_or_else(|| "never".to_string());
        let status = if stat.count >= governance.hard_limit as usize {
            " \u{26A0} OVER HARD LIMIT \u{2014} must decompose"
        } else if stat.count >= governance.warn_entries as usize {
            " \u{26A0} consider splitting domain"
        } else if stat.count >= governance.max_entries as usize {
            " \u{2014} approaching limit"
        } else {
            ""
        };
        lines.push(format!(
            "  {}: {} records (updated {updated}){status}",
            stat.domain, stat.count
        ));
    }

    lines.join("\n")
}
