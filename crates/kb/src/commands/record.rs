use anyhow::{Context, Result, bail};
use std::io::Read as _;
use std::path::Path;

use crate::cli::RecordArgs;
use crate::context::RuntimeContext;
use crate::output::*;

use kb_core::types::*;
use kb_core::{config, filter, lock, storage};

// ── Helpers ─────────────────────────────────────────────────────────────────

fn parse_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn parse_classification(s: &str) -> Classification {
    match s {
        "foundational" => Classification::Foundational,
        "observational" => Classification::Observational,
        _ => Classification::Tactical,
    }
}

fn build_evidence(args: &RecordArgs) -> Option<Evidence> {
    if args.evidence_commit.is_none()
        && args.evidence_issue.is_none()
        && args.evidence_file.is_none()
        && args.evidence_bead.is_none()
    {
        return None;
    }
    Some(Evidence {
        commit: args.evidence_commit.clone(),
        date: None,
        issue: args.evidence_issue.clone(),
        file: args.evidence_file.clone(),
        bead: args.evidence_bead.clone(),
    })
}

fn build_outcomes(args: &RecordArgs) -> Option<Vec<Outcome>> {
    let status_str = args.outcome_status.as_deref()?;
    let status = match status_str {
        "success" => OutcomeStatus::Success,
        "failure" => OutcomeStatus::Failure,
        "partial" => OutcomeStatus::Partial,
        _ => return None,
    };
    Some(vec![Outcome {
        status,
        duration: args.outcome_duration,
        test_results: args.outcome_test_results.clone(),
        agent: args.outcome_agent.clone(),
        notes: None,
        recorded_at: None,
    }])
}

// ── Fill defaults on raw JSON before deserialization ─────────────────────────

fn fill_defaults(value: &mut serde_json::Value) {
    if let Some(obj) = value.as_object_mut() {
        if !obj.contains_key("recorded_at") {
            obj.insert(
                "recorded_at".to_string(),
                serde_json::Value::String(now_iso()),
            );
        }
        if !obj.contains_key("classification") {
            obj.insert(
                "classification".to_string(),
                serde_json::Value::String("tactical".to_string()),
            );
        }
    }
}

// ── Batch result tracking ───────────────────────────────────────────────────

struct BatchResult {
    created: usize,
    updated: usize,
    skipped: usize,
    errors: Vec<String>,
}

// ── Process bulk records (stdin or batch file) ──────────────────────────────

/// Parses JSON input (single object or array), validates each record via serde,
/// fills in defaults, deduplicates, and writes atomically with file locking.
fn process_bulk_records(
    file_path: &Path,
    input_data: &str,
    force: bool,
    dry_run: bool,
) -> Result<BatchResult> {
    let parsed: serde_json::Value =
        serde_json::from_str(input_data).context("Failed to parse JSON input")?;

    let raw_records: Vec<serde_json::Value> = if parsed.is_array() {
        parsed.as_array().cloned().unwrap_or_default()
    } else {
        vec![parsed]
    };

    let mut errors: Vec<String> = Vec::new();
    let mut valid_records: Vec<ExpertiseRecord> = Vec::new();

    for (i, mut raw) in raw_records.into_iter().enumerate() {
        fill_defaults(&mut raw);

        match serde_json::from_value::<ExpertiseRecord>(raw) {
            Ok(record) => valid_records.push(record),
            Err(e) => {
                errors.push(format!("Record {i}: {e}"));
            }
        }
    }

    if valid_records.is_empty() {
        return Ok(BatchResult {
            created: 0,
            updated: 0,
            skipped: 0,
            errors,
        });
    }

    let mut created: usize = 0;
    let mut updated: usize = 0;
    let mut skipped: usize = 0;

    if dry_run {
        // Dry-run: check for duplicates without writing
        let existing = storage::read_expertise_file(file_path)?;
        let mut current_records = existing;

        for record in &valid_records {
            let dup = filter::find_duplicate(&current_records, record);

            if dup.is_some() && !force {
                if record.is_named_type() {
                    updated += 1;
                } else {
                    skipped += 1;
                }
            } else {
                created += 1;
            }
            // Track what we would add for accurate intra-batch dedup
            current_records.push(record.clone());
        }
    } else {
        // Normal mode: write with file locking
        lock::with_file_lock(file_path, || {
            let existing = storage::read_expertise_file(file_path)?;
            let mut current_records = existing;

            for record in valid_records {
                let dup = filter::find_duplicate(&current_records, &record);

                if let Some((idx, _)) = dup {
                    if !force {
                        if record.is_named_type() {
                            // Upsert: replace in place
                            current_records[idx] = record;
                            updated += 1;
                        } else {
                            // Exact match on unnamed type: skip
                            skipped += 1;
                        }
                    } else {
                        current_records.push(record);
                        created += 1;
                    }
                } else {
                    current_records.push(record);
                    created += 1;
                }
            }

            // Write all changes at once
            if created > 0 || updated > 0 {
                storage::write_expertise_file(file_path, &mut current_records)?;
            }

            Ok(())
        })?;
    }

    Ok(BatchResult {
        created,
        updated,
        skipped,
        errors,
    })
}

// ── Output helpers for batch/stdin results ──────────────────────────────────

fn output_bulk_errors(ctx: &RuntimeContext, result: &BatchResult) {
    if result.errors.is_empty() {
        return;
    }
    if ctx.json {
        output_json_error(
            "record",
            &format!("Validation errors: {}", result.errors.join("; ")),
        );
    } else {
        print_error("Validation errors:");
        for error in &result.errors {
            print_error(&format!("  {error}"));
        }
    }
}

fn output_bulk_json(domain: &str, action: &str, result: &BatchResult) {
    output_json(&serde_json::json!({
        "success": result.errors.is_empty() || (result.created + result.updated) > 0,
        "command": "record",
        "action": action,
        "domain": domain,
        "created": result.created,
        "updated": result.updated,
        "skipped": result.skipped,
        "errors": result.errors,
    }));
}

fn output_bulk_text_dry_run(domain: &str, result: &BatchResult) {
    let total = result.created + result.updated;
    if total > 0 || result.skipped > 0 {
        print_success(&format!(
            "Dry-run complete. Would process {total} record(s) in {domain}:"
        ));
        if result.created > 0 {
            println!("  Create: {}", result.created);
        }
        if result.updated > 0 {
            println!("  Update: {}", result.updated);
        }
        if result.skipped > 0 {
            println!("  Skip: {}", result.skipped);
        }
        println!("  Run without --dry-run to apply changes.");
    } else {
        print_warning("No records would be processed.");
    }
}

fn output_bulk_text(domain: &str, result: &BatchResult) {
    if result.created > 0 {
        print_success(&format!("Created {} record(s) in {domain}", result.created));
    }
    if result.updated > 0 {
        print_success(&format!("Updated {} record(s) in {domain}", result.updated));
    }
    if result.skipped > 0 {
        print_warning(&format!(
            "Skipped {} duplicate(s) in {domain}",
            result.skipped
        ));
    }
}

// ── Main entry point ────────────────────────────────────────────────────────

pub fn run(ctx: &RuntimeContext, args: &RecordArgs) -> Result<()> {
    // Handle --batch mode
    if let Some(ref batch_file) = args.batch {
        return run_batch(ctx, args, batch_file);
    }

    // Handle --stdin mode
    if args.stdin {
        return run_stdin(ctx, args);
    }

    // Handle CLI args mode
    run_cli(ctx, args)
}

// ── --batch mode ────────────────────────────────────────────────────────────

fn run_batch(ctx: &RuntimeContext, args: &RecordArgs, batch_file: &str) -> Result<()> {
    if !Path::new(batch_file).exists() {
        if ctx.json {
            output_json_error("record", &format!("Batch file not found: {batch_file}"));
            return Ok(());
        }
        bail!("Batch file not found: {batch_file}");
    }

    config::ensure_kb_dir(&ctx.cwd)?;
    let cfg = config::read_config(&ctx.cwd)?;
    config::ensure_domain_exists(&cfg, &args.domain)?;

    let file_path = config::get_expertise_path(&args.domain, &ctx.cwd)?;
    let file_content = std::fs::read_to_string(batch_file)
        .with_context(|| format!("Failed to read batch file: {batch_file}"))?;

    let result = process_bulk_records(&file_path, &file_content, args.force, args.dry_run)?;

    output_bulk_errors(ctx, &result);

    if ctx.json {
        let action = if args.dry_run { "dry-run" } else { "batch" };
        output_bulk_json(&args.domain, action, &result);
    } else if args.dry_run {
        output_bulk_text_dry_run(&args.domain, &result);
    } else {
        output_bulk_text(&args.domain, &result);
    }

    if !result.errors.is_empty() && result.created + result.updated == 0 {
        bail!("All records failed validation");
    }

    Ok(())
}

// ── --stdin mode ────────────────────────────────────────────────────────────

fn run_stdin(ctx: &RuntimeContext, args: &RecordArgs) -> Result<()> {
    config::ensure_kb_dir(&ctx.cwd)?;
    let cfg = config::read_config(&ctx.cwd)?;
    config::ensure_domain_exists(&cfg, &args.domain)?;

    let file_path = config::get_expertise_path(&args.domain, &ctx.cwd)?;

    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .context("Failed to read from stdin")?;

    let result = process_bulk_records(&file_path, &input, args.force, args.dry_run)?;

    output_bulk_errors(ctx, &result);

    if ctx.json {
        let action = if args.dry_run { "dry-run" } else { "stdin" };
        output_bulk_json(&args.domain, action, &result);
    } else if args.dry_run {
        output_bulk_text_dry_run(&args.domain, &result);
    } else {
        output_bulk_text(&args.domain, &result);
    }

    if !result.errors.is_empty() && result.created + result.updated == 0 {
        bail!("All records failed validation");
    }

    Ok(())
}

// ── CLI args mode ───────────────────────────────────────────────────────────

fn run_cli(ctx: &RuntimeContext, args: &RecordArgs) -> Result<()> {
    config::ensure_kb_dir(&ctx.cwd)?;
    let cfg = config::read_config(&ctx.cwd)?;
    config::ensure_domain_exists(&cfg, &args.domain)?;

    // --type is required in CLI mode
    let type_str = match args.record_type.as_deref() {
        Some(t) => t,
        None => {
            if ctx.json {
                output_json_error(
                    "record",
                    "--type is required (convention, pattern, failure, decision, reference, guide)",
                );
                return Ok(());
            }
            bail!("--type is required (convention, pattern, failure, decision, reference, guide)");
        }
    };

    let classification = parse_classification(&args.classification);
    let recorded_at = now_iso();
    let evidence = build_evidence(args);
    let tags = args.tags.as_deref().map(parse_csv);
    let relates_to = args.relates_to.as_deref().map(parse_csv);
    let supersedes = args.supersedes.as_deref().map(parse_csv);
    let outcomes = build_outcomes(args);
    let files = args.files.as_deref().map(parse_csv);

    // Build record from CLI args based on type
    let record = match type_str {
        "convention" => {
            let content = args.content.as_deref().or(args.description.as_deref());
            let content = match content {
                Some(c) => c.to_string(),
                None => {
                    if ctx.json {
                        output_json_error(
                            "record",
                            "Convention records require content (positional argument or --description).",
                        );
                        return Ok(());
                    }
                    bail!(
                        "Convention records require content (positional argument or --description)."
                    );
                }
            };
            ExpertiseRecord::Convention {
                id: None,
                content,
                classification,
                recorded_at,
                evidence,
                tags,
                relates_to,
                supersedes,
                outcomes,
            }
        }

        "pattern" => {
            let name = args.name.as_deref();
            let description = args.description.as_deref().or(args.content.as_deref());
            match (name, description) {
                (Some(n), Some(d)) => ExpertiseRecord::Pattern {
                    id: None,
                    name: n.to_string(),
                    description: d.to_string(),
                    files,
                    classification,
                    recorded_at,
                    evidence,
                    tags,
                    relates_to,
                    supersedes,
                    outcomes,
                },
                _ => {
                    if ctx.json {
                        output_json_error(
                            "record",
                            "Pattern records require --name and --description (or positional content).",
                        );
                        return Ok(());
                    }
                    bail!(
                        "Pattern records require --name and --description (or positional content)."
                    );
                }
            }
        }

        "failure" => {
            let description = args.description.as_deref();
            let resolution = args.resolution.as_deref();
            match (description, resolution) {
                (Some(d), Some(r)) => ExpertiseRecord::Failure {
                    id: None,
                    description: d.to_string(),
                    resolution: r.to_string(),
                    classification,
                    recorded_at,
                    evidence,
                    tags,
                    relates_to,
                    supersedes,
                    outcomes,
                },
                _ => {
                    if ctx.json {
                        output_json_error(
                            "record",
                            "Failure records require --description and --resolution.",
                        );
                        return Ok(());
                    }
                    bail!("Failure records require --description and --resolution.");
                }
            }
        }

        "decision" => {
            let title = args.title.as_deref();
            let rationale = args.rationale.as_deref();
            match (title, rationale) {
                (Some(t), Some(r)) => ExpertiseRecord::Decision {
                    id: None,
                    title: t.to_string(),
                    rationale: r.to_string(),
                    date: None,
                    classification,
                    recorded_at,
                    evidence,
                    tags,
                    relates_to,
                    supersedes,
                    outcomes,
                },
                _ => {
                    if ctx.json {
                        output_json_error(
                            "record",
                            "Decision records require --title and --rationale.",
                        );
                        return Ok(());
                    }
                    bail!("Decision records require --title and --rationale.");
                }
            }
        }

        "reference" => {
            let name = args.name.as_deref();
            let description = args.description.as_deref().or(args.content.as_deref());
            match (name, description) {
                (Some(n), Some(d)) => ExpertiseRecord::Reference {
                    id: None,
                    name: n.to_string(),
                    description: d.to_string(),
                    files,
                    classification,
                    recorded_at,
                    evidence,
                    tags,
                    relates_to,
                    supersedes,
                    outcomes,
                },
                _ => {
                    if ctx.json {
                        output_json_error(
                            "record",
                            "Reference records require --name and --description (or positional content).",
                        );
                        return Ok(());
                    }
                    bail!(
                        "Reference records require --name and --description (or positional content)."
                    );
                }
            }
        }

        "guide" => {
            let name = args.name.as_deref();
            let description = args.description.as_deref().or(args.content.as_deref());
            match (name, description) {
                (Some(n), Some(d)) => ExpertiseRecord::Guide {
                    id: None,
                    name: n.to_string(),
                    description: d.to_string(),
                    classification,
                    recorded_at,
                    evidence,
                    tags,
                    relates_to,
                    supersedes,
                    outcomes,
                },
                _ => {
                    if ctx.json {
                        output_json_error(
                            "record",
                            "Guide records require --name and --description (or positional content).",
                        );
                        return Ok(());
                    }
                    bail!(
                        "Guide records require --name and --description (or positional content)."
                    );
                }
            }
        }

        other => bail!("Unknown record type: {other}"),
    };

    let file_path = config::get_expertise_path(&args.domain, &ctx.cwd)?;
    let record_type = record.record_type();

    if args.dry_run {
        run_cli_dry_run(
            ctx,
            &args.domain,
            &file_path,
            &record,
            record_type,
            args.force,
        )
    } else {
        run_cli_write(
            ctx,
            &args.domain,
            &file_path,
            record,
            record_type,
            args.force,
        )
    }
}

// ── CLI dry-run ─────────────────────────────────────────────────────────────

fn run_cli_dry_run(
    ctx: &RuntimeContext,
    domain: &str,
    file_path: &Path,
    record: &ExpertiseRecord,
    record_type: RecordType,
    force: bool,
) -> Result<()> {
    let existing = storage::read_expertise_file(file_path)?;
    let dup = filter::find_duplicate(&existing, record);

    let action = if dup.is_some() && !force {
        if record.is_named_type() {
            "updated"
        } else {
            "skipped"
        }
    } else {
        "created"
    };

    if ctx.json {
        output_json(&serde_json::json!({
            "success": true,
            "command": "record",
            "action": "dry-run",
            "wouldDo": action,
            "domain": domain,
            "type": record_type.as_str(),
            "record": record,
        }));
    } else {
        match action {
            "created" => {
                print_success(&format!("Dry-run: Would create {record_type} in {domain}"));
            }
            "updated" => {
                print_success(&format!(
                    "Dry-run: Would update existing {record_type} in {domain}"
                ));
            }
            _ => {
                print_warning(&format!(
                    "Dry-run: Duplicate {record_type} already exists in {domain}. Would skip."
                ));
            }
        }
        println!("  Run without --dry-run to apply changes.");
    }

    Ok(())
}

// ── CLI write (normal mode) ─────────────────────────────────────────────────

fn run_cli_write(
    ctx: &RuntimeContext,
    domain: &str,
    file_path: &Path,
    record: ExpertiseRecord,
    record_type: RecordType,
    force: bool,
) -> Result<()> {
    lock::with_file_lock(file_path, || {
        let mut existing = storage::read_expertise_file(file_path)?;
        let dup = filter::find_duplicate(&existing, &record);

        if let Some((idx, _)) = dup {
            if !force {
                if record.is_named_type() {
                    // Upsert: replace in place
                    existing[idx] = record.clone();
                    storage::write_expertise_file(file_path, &mut existing)?;

                    if ctx.json {
                        output_json(&serde_json::json!({
                            "success": true,
                            "command": "record",
                            "action": "updated",
                            "domain": domain,
                            "type": record_type.as_str(),
                            "index": idx + 1,
                            "record": record,
                        }));
                    } else {
                        print_success(&format!(
                            "Updated existing {record_type} in {domain} (record #{})",
                            idx + 1
                        ));
                    }
                } else {
                    // Exact match on unnamed type: skip
                    if ctx.json {
                        output_json(&serde_json::json!({
                            "success": true,
                            "command": "record",
                            "action": "skipped",
                            "domain": domain,
                            "type": record_type.as_str(),
                            "index": idx + 1,
                        }));
                    } else {
                        print_warning(&format!(
                            "Duplicate {record_type} already exists in {domain} (record #{}). Use --force to add anyway.",
                            idx + 1
                        ));
                    }
                }
                return Ok(());
            }
        }

        // New record (or --force): append
        let mut record = record;
        storage::append_record(file_path, &mut record)?;

        if ctx.json {
            output_json(&serde_json::json!({
                "success": true,
                "command": "record",
                "action": "created",
                "domain": domain,
                "type": record_type.as_str(),
                "record": record,
            }));
        } else {
            print_success(&format!("Recorded {record_type} in {domain}"));
        }

        Ok(())
    })?;

    Ok(())
}
