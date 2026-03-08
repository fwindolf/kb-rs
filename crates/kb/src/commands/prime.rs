use std::collections::HashSet;
use std::fs;

use anyhow::{Context, Result, bail};

use crate::cli::PrimeArgs;
use crate::context::RuntimeContext;
use crate::output::*;
use kb_core::format::PrimeFormat;
use kb_core::{budget, config, format, git, storage, types::*};

/// Produce a rough text representation of a record for token estimation.
/// Uses a simple format similar to compact lines.
fn estimate_record_text(record: &ExpertiseRecord) -> String {
    match record {
        ExpertiseRecord::Convention { content, .. } => {
            format!("[convention] {content}")
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
            format!("[pattern] {name}: {description}{f}")
        }
        ExpertiseRecord::Failure {
            description,
            resolution,
            ..
        } => {
            format!("[failure] {description} -> {resolution}")
        }
        ExpertiseRecord::Decision {
            title, rationale, ..
        } => {
            format!("[decision] {title}: {rationale}")
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
                .unwrap_or_else(|| format!(": {description}"));
            format!("[reference] {name}{detail}")
        }
        ExpertiseRecord::Guide {
            name, description, ..
        } => {
            format!("[guide] {name}: {description}")
        }
    }
}

/// Get the last-modified time of a file as an RFC 3339 string, or None.
fn get_file_mod_time(path: &std::path::Path) -> Option<String> {
    let meta = fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?;
    let dt: chrono::DateTime<chrono::Utc> = modified.into();
    Some(dt.to_rfc3339())
}

/// Parse a comma-or-space separated `--files` value into individual paths.
fn parse_file_paths(raw: &str) -> Vec<String> {
    raw.split(',')
        .flat_map(|s| s.split_whitespace())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Parse the `--domain` / `--exclude-domain` flags which may contain comma-separated values.
fn parse_domain_flag(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn run(ctx: &RuntimeContext, args: &PrimeArgs) -> Result<()> {
    config::ensure_kb_dir(&ctx.cwd)?;
    let cfg = config::read_config(&ctx.cwd)?;

    let fmt = match args.format.as_str() {
        "xml" => PrimeFormat::Xml,
        "plain" => PrimeFormat::Plain,
        _ => PrimeFormat::Markdown,
    };

    // ── Determine target domains ────────────────────────────────────────

    // Merge positional args and --domain flag
    let mut requested: Vec<String> = args.domains.clone();
    if let Some(ref domain_flag) = args.domain {
        requested.extend(parse_domain_flag(domain_flag));
    }

    // Deduplicate while preserving order
    let mut seen = HashSet::new();
    let unique: Vec<String> = requested
        .into_iter()
        .filter(|d| seen.insert(d.clone()))
        .collect();

    // Validate all requested domains exist in config
    for d in &unique {
        if !cfg.domains.contains(d) {
            let available = cfg.domains.join(", ");
            let msg = format!("Domain \"{d}\" not found in config. Available domains: {available}");
            if ctx.json {
                output_json_error("prime", &msg);
                return Ok(());
            }
            bail!("{msg}");
        }
    }

    // Validate excluded domains
    let excluded: Vec<String> = args
        .exclude_domain
        .as_ref()
        .map(|e| parse_domain_flag(e))
        .unwrap_or_default();

    for d in &excluded {
        if !cfg.domains.contains(d) {
            let available = cfg.domains.join(", ");
            let msg = format!(
                "Excluded domain \"{d}\" not found in config. Available domains: {available}"
            );
            if ctx.json {
                output_json_error("prime", &msg);
                return Ok(());
            }
            bail!("{msg}");
        }
    }

    let target_domains: Vec<String> = if unique.is_empty() {
        cfg.domains.clone()
    } else {
        unique
    }
    .into_iter()
    .filter(|d| !excluded.contains(d))
    .collect();

    // ── Resolve file filter ─────────────────────────────────────────────

    let files_to_filter: Option<Vec<String>> = if args.context {
        if !git::is_git_repo(&ctx.cwd) {
            let msg = "Not in a git repository. --context requires git.";
            if ctx.json {
                output_json_error("prime", msg);
                return Ok(());
            }
            bail!("{msg}");
        }
        let changed = git::get_changed_files(&ctx.cwd, "HEAD~1");
        if changed.is_empty() {
            if ctx.json {
                output_json_error("prime", "No changed files found. Nothing to filter by.");
            } else {
                println!("No changed files found. Nothing to filter by.");
            }
            return Ok(());
        }
        Some(changed)
    } else if let Some(ref raw) = args.files {
        let paths = parse_file_paths(raw);
        if paths.is_empty() { None } else { Some(paths) }
    } else {
        None
    };

    // ── Budget settings ─────────────────────────────────────────────────

    let is_machine_output = args.mcp || ctx.json;
    let budget_enabled = !is_machine_output && !args.no_limit;
    let token_budget = args.budget.unwrap_or(budget::DEFAULT_BUDGET);

    // ── Generate output ─────────────────────────────────────────────────

    let output = if is_machine_output {
        // --json and --mcp produce structured JSON output — no budget
        let mut domains: Vec<(String, usize, Vec<ExpertiseRecord>)> = Vec::new();

        for domain in &target_domains {
            let file_path = config::get_expertise_path(domain, &ctx.cwd)?;
            let records = storage::read_expertise_file(&file_path)?;

            let filtered: Vec<ExpertiseRecord> = if let Some(ref filter_files) = files_to_filter {
                let refs = git::filter_by_context(&records, filter_files);
                refs.into_iter().cloned().collect()
            } else {
                records
            };

            // Include domain if no file filter is active, or if filtering produced results
            if files_to_filter.is_none() || !filtered.is_empty() {
                let count = filtered.len();
                domains.push((domain.clone(), count, filtered));
            }
        }

        // format_mcp_output takes &[(String, usize, &[ExpertiseRecord])]
        let mcp_input: Vec<(String, usize, &[ExpertiseRecord])> = domains
            .iter()
            .map(|(d, c, recs)| (d.clone(), *c, recs.as_slice()))
            .collect();
        format::format_mcp_output(&mcp_input)
    } else {
        // Human-readable output

        // Load all records per domain
        struct DomainData {
            domain: String,
            records: Vec<ExpertiseRecord>,
            last_updated: Option<String>,
        }

        let mut all_domains: Vec<DomainData> = Vec::new();

        for domain in &target_domains {
            let file_path = config::get_expertise_path(domain, &ctx.cwd)?;
            let records = storage::read_expertise_file(&file_path)?;

            let filtered: Vec<ExpertiseRecord> = if let Some(ref filter_files) = files_to_filter {
                let refs = git::filter_by_context(&records, filter_files);
                if refs.is_empty() {
                    continue;
                }
                refs.into_iter().cloned().collect()
            } else {
                records
            };

            let last_updated = get_file_mod_time(&file_path);

            all_domains.push(DomainData {
                domain: domain.clone(),
                records: filtered,
                last_updated,
            });
        }

        // Build DomainRecords for budget calculation (references into our owned data)
        let domain_records: Vec<budget::DomainRecords<'_>> = all_domains
            .iter()
            .map(|dd| budget::DomainRecords {
                domain: dd.domain.clone(),
                records: dd.records.iter().collect(),
            })
            .collect();

        // Apply budget filtering
        let (records_to_format, dropped_count, dropped_domain_count) = if budget_enabled {
            let result = budget::apply_budget(&domain_records, token_budget, |record, _domain| {
                estimate_record_text(record)
            });
            (
                result.kept,
                result.dropped_count,
                result.dropped_domain_count,
            )
        } else {
            (domain_records, 0, 0)
        };

        // Build a lookup for last_updated by domain name
        let mod_times: std::collections::HashMap<&str, Option<&str>> = all_domains
            .iter()
            .map(|dd| (dd.domain.as_str(), dd.last_updated.as_deref()))
            .collect();

        // Determine whether to use full/verbose or compact formatting
        let use_full = args.verbose || args.full || fmt != PrimeFormat::Markdown;

        // Format each domain section
        let domain_sections: Vec<String> = records_to_format
            .iter()
            .map(|dr| {
                let last_updated = mod_times.get(dr.domain.as_str()).copied().flatten();

                // Collect owned records for formatting functions that take &[ExpertiseRecord]
                let owned: Vec<ExpertiseRecord> = dr.records.iter().map(|r| (*r).clone()).collect();

                if use_full {
                    match fmt {
                        PrimeFormat::Xml => {
                            format::format_domain_expertise_xml(&dr.domain, &owned, last_updated)
                        }
                        PrimeFormat::Plain => {
                            format::format_domain_expertise_plain(&dr.domain, &owned, last_updated)
                        }
                        PrimeFormat::Markdown => format::format_domain_expertise(
                            &dr.domain,
                            &owned,
                            last_updated,
                            args.full,
                        ),
                    }
                } else {
                    format::format_domain_expertise_compact(&dr.domain, &owned, last_updated)
                }
            })
            .collect();

        // Wrap in prime output format
        let mut out = if use_full {
            match fmt {
                PrimeFormat::Xml => format::format_prime_output_xml(&domain_sections),
                PrimeFormat::Plain => format::format_prime_output_plain(&domain_sections),
                PrimeFormat::Markdown => format::format_prime_output(&domain_sections),
            }
        } else {
            format::format_prime_output_compact(&domain_sections)
        };

        // Append truncation summary if records were dropped
        if dropped_count > 0 {
            out.push_str("\n\n");
            out.push_str(&budget::format_budget_summary(
                dropped_count,
                dropped_domain_count,
            ));
        }

        // Append session-end reminder
        out.push_str("\n\n");
        out.push_str(&format::get_session_end_reminder(fmt));

        out
    };

    // ── Export or print ──────────────────────────────────────────────────

    if let Some(ref export_path) = args.export {
        fs::write(export_path, format!("{output}\n"))
            .with_context(|| format!("Failed to write to {export_path}"))?;
        if !ctx.json {
            print_success(&format!("Exported to {export_path}"));
        }
    } else {
        println!("{output}");
    }

    Ok(())
}
