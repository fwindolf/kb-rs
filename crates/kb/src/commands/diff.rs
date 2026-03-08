use anyhow::{Result, bail};
use std::collections::HashMap;
use std::process::Command;

use crate::cli::DiffArgs;
use crate::context::RuntimeContext;
use crate::output::*;
use kb_core::types::ExpertiseRecord;
use kb_core::{config, format, storage};

/// Read a JSONL file content from a git ref.
fn read_file_at_ref(cwd: &std::path::Path, git_ref: &str, rel_path: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["show", &format!("{git_ref}:{rel_path}")])
        .current_dir(cwd)
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

/// Parse JSONL content into records, skipping bad lines.
fn parse_jsonl(content: &str) -> Vec<ExpertiseRecord> {
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

pub fn run(ctx: &RuntimeContext, args: &DiffArgs) -> Result<()> {
    config::ensure_kb_dir(&ctx.cwd)?;

    if !kb_core::git::is_git_repo(&ctx.cwd) {
        if ctx.json {
            output_json_error("diff", "Not in a git repository.");
            return Ok(());
        }
        bail!("Not in a git repository. `kb diff` requires git.");
    }

    let cfg = config::read_config(&ctx.cwd)?;

    let mut added: Vec<(String, ExpertiseRecord)> = Vec::new();
    let mut removed: Vec<(String, ExpertiseRecord)> = Vec::new();

    for domain in &cfg.domains {
        let rel_path = format!(".kb/expertise/{domain}.jsonl");

        // Current state
        let file_path = config::get_expertise_path(domain, &ctx.cwd)?;
        let current_records = storage::read_expertise_file(&file_path)?;

        // Old state from git ref
        let old_records = read_file_at_ref(&ctx.cwd, &args.since, &rel_path)
            .map(|content| parse_jsonl(&content))
            .unwrap_or_default();

        // Build ID maps
        let current_ids: HashMap<String, &ExpertiseRecord> = current_records
            .iter()
            .filter_map(|r| r.id().map(|id| (id.to_string(), r)))
            .collect();

        let old_ids: HashMap<String, &ExpertiseRecord> = old_records
            .iter()
            .filter_map(|r| r.id().map(|id| (id.to_string(), r)))
            .collect();

        // Added: in current but not in old
        for (id, record) in &current_ids {
            if !old_ids.contains_key(id) {
                added.push((domain.clone(), (*record).clone()));
            }
        }

        // Removed: in old but not in current
        for (id, record) in &old_ids {
            if !current_ids.contains_key(id) {
                removed.push((domain.clone(), (*record).clone()));
            }
        }
    }

    if ctx.json {
        let added_json: Vec<serde_json::Value> = added
            .iter()
            .map(|(d, r)| serde_json::json!({"domain": d, "record": r}))
            .collect();
        let removed_json: Vec<serde_json::Value> = removed
            .iter()
            .map(|(d, r)| serde_json::json!({"domain": d, "record": r}))
            .collect();
        output_json(&serde_json::json!({
            "success": true,
            "command": "diff",
            "since": args.since,
            "added": added_json,
            "removed": removed_json,
        }));
    } else {
        println!("Expertise diff since {}:", args.since);
        println!();

        if added.is_empty() && removed.is_empty() {
            println!("No changes.");
            return Ok(());
        }

        if !added.is_empty() {
            println!("Added ({}):", added.len());
            for (domain, r) in &added {
                let id = r.id().unwrap_or("?");
                let summary = format::get_record_summary(r);
                println!("  + [{domain}] {id} {}: {summary}", r.record_type());
            }
            println!();
        }

        if !removed.is_empty() {
            println!("Removed ({}):", removed.len());
            for (domain, r) in &removed {
                let id = r.id().unwrap_or("?");
                let summary = format::get_record_summary(r);
                println!("  - [{domain}] {id} {}: {summary}", r.record_type());
            }
        }
    }

    Ok(())
}
