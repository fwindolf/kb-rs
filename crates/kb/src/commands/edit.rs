use anyhow::Result;

use crate::cli::EditArgs;
use crate::context::RuntimeContext;
use crate::output::*;

use kb_core::types::*;
use kb_core::{config, lock, resolve, storage};

// ── Helpers ─────────────────────────────────────────────────────────────────

fn parse_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

fn parse_classification(s: &str) -> std::result::Result<Classification, kb_core::error::KbError> {
    match s {
        "foundational" => Ok(Classification::Foundational),
        "tactical" => Ok(Classification::Tactical),
        "observational" => Ok(Classification::Observational),
        other => Err(kb_core::error::KbError::ValidationError(format!(
            "Invalid classification: {other}"
        ))),
    }
}

fn parse_outcome_status(s: &str) -> std::result::Result<OutcomeStatus, kb_core::error::KbError> {
    match s {
        "success" => Ok(OutcomeStatus::Success),
        "failure" => Ok(OutcomeStatus::Failure),
        "partial" => Ok(OutcomeStatus::Partial),
        other => Err(kb_core::error::KbError::ValidationError(format!(
            "Invalid outcome status: {other}"
        ))),
    }
}

// ── Main entry point ────────────────────────────────────────────────────────

pub fn run(ctx: &RuntimeContext, args: &EditArgs) -> Result<()> {
    // Check for .kb directory
    if let Err(e) = config::ensure_kb_dir(&ctx.cwd) {
        let msg = format!("{e}");
        if ctx.json {
            output_json_error("edit", &msg);
            return Ok(());
        }
        return Err(e.into());
    }

    let cfg = match config::read_config(&ctx.cwd) {
        Ok(c) => c,
        Err(e) => {
            if ctx.json {
                output_json_error("edit", &format!("{e}"));
                return Ok(());
            }
            return Err(e.into());
        }
    };

    if let Err(e) = config::ensure_domain_exists(&cfg, &args.domain) {
        if ctx.json {
            output_json_error("edit", &format!("{e}"));
            return Ok(());
        }
        return Err(e.into());
    }

    let file_path = config::get_expertise_path(&args.domain, &ctx.cwd)?;

    // All reads and writes happen inside the file lock for consistency
    lock::with_file_lock(&file_path, || {
        let mut records = storage::read_expertise_file(&file_path)?;

        // Resolve the record ID
        let (target_index, _) = match resolve::resolve_record_id(&records, &args.id) {
            Ok(resolved) => resolved,
            Err(e) => {
                if ctx.json {
                    output_json_error("edit", &format!("{e}"));
                    return Ok(());
                }
                return Err(e);
            }
        };

        let record = &mut records[target_index];

        // ── Apply common field updates ──────────────────────────────────

        if let Some(ref cls_str) = args.classification {
            let cls = parse_classification(cls_str)?;
            record.set_classification(cls);
        }

        if let Some(ref rt) = args.relates_to {
            record.set_relates_to(Some(parse_csv(rt)));
        }

        if let Some(ref ss) = args.supersedes {
            record.set_supersedes(Some(parse_csv(ss)));
        }

        // ── Append outcome if provided ──────────────────────────────────

        if let Some(ref status_str) = args.outcome_status {
            let status = parse_outcome_status(status_str)?;
            let new_outcome = Outcome {
                status,
                duration: args.outcome_duration,
                test_results: args.outcome_test_results.clone(),
                agent: args.outcome_agent.clone(),
                notes: None,
                recorded_at: None,
            };
            let mut existing_outcomes = record.outcomes().map(|o| o.to_vec()).unwrap_or_default();
            existing_outcomes.push(new_outcome);
            record.set_outcomes(Some(existing_outcomes));
        }

        // ── Apply type-specific field updates ───────────────────────────

        match record {
            ExpertiseRecord::Convention { content, .. } => {
                if let Some(ref new_content) = args.content {
                    *content = new_content.clone();
                }
            }
            ExpertiseRecord::Pattern {
                name,
                description,
                files,
                ..
            } => {
                if let Some(ref new_name) = args.name {
                    *name = new_name.clone();
                }
                if let Some(ref new_desc) = args.description {
                    *description = new_desc.clone();
                }
                if let Some(ref new_files) = args.files {
                    *files = Some(parse_csv(new_files));
                }
            }
            ExpertiseRecord::Failure {
                description,
                resolution,
                ..
            } => {
                if let Some(ref new_desc) = args.description {
                    *description = new_desc.clone();
                }
                if let Some(ref new_res) = args.resolution {
                    *resolution = new_res.clone();
                }
            }
            ExpertiseRecord::Decision {
                title, rationale, ..
            } => {
                if let Some(ref new_title) = args.title {
                    *title = new_title.clone();
                }
                if let Some(ref new_rationale) = args.rationale {
                    *rationale = new_rationale.clone();
                }
            }
            ExpertiseRecord::Reference {
                name,
                description,
                files,
                ..
            } => {
                if let Some(ref new_name) = args.name {
                    *name = new_name.clone();
                }
                if let Some(ref new_desc) = args.description {
                    *description = new_desc.clone();
                }
                if let Some(ref new_files) = args.files {
                    *files = Some(parse_csv(new_files));
                }
            }
            ExpertiseRecord::Guide {
                name, description, ..
            } => {
                if let Some(ref new_name) = args.name {
                    *name = new_name.clone();
                }
                if let Some(ref new_desc) = args.description {
                    *description = new_desc.clone();
                }
            }
        }

        // ── Write back ──────────────────────────────────────────────────

        storage::write_expertise_file(&file_path, &mut records)?;

        // ── Output ──────────────────────────────────────────────────────

        let record = &records[target_index];
        let record_id = record.id().unwrap_or_default();
        let record_type = record.record_type();

        if ctx.json {
            output_json(&serde_json::json!({
                "success": true,
                "command": "edit",
                "domain": args.domain,
                "id": record_id,
                "type": record_type.as_str(),
                "record": record,
            }));
        } else {
            print_success(&format!(
                "Updated {record_type} {record_id} in {}",
                args.domain
            ));
        }

        Ok(())
    })?;

    Ok(())
}
