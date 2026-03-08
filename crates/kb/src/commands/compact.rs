use anyhow::Result;
use std::collections::HashMap;

use crate::cli::CompactArgs;
use crate::context::RuntimeContext;
use crate::output::*;
use kb_core::types::*;
use kb_core::{config, lock, storage};

/// Group records by type for potential merging.
fn find_compact_groups(records: &[ExpertiseRecord]) -> HashMap<RecordType, Vec<usize>> {
    let mut groups: HashMap<RecordType, Vec<usize>> = HashMap::new();
    for (i, r) in records.iter().enumerate() {
        groups.entry(r.record_type()).or_default().push(i);
    }
    // Only keep groups with more than 1 record (candidates for compaction)
    groups.retain(|_, indices| indices.len() > 1);
    groups
}

pub fn run(ctx: &RuntimeContext, args: &CompactArgs) -> Result<()> {
    config::ensure_kb_dir(&ctx.cwd)?;
    let cfg = config::read_config(&ctx.cwd)?;

    let mut total_merged = 0usize;
    let mut json_domains: Vec<serde_json::Value> = Vec::new();

    for domain in &cfg.domains {
        let file_path = config::get_expertise_path(domain, &ctx.cwd)?;
        let records = storage::read_expertise_file(&file_path)?;
        let groups = find_compact_groups(&records);

        if groups.is_empty() {
            continue;
        }

        let mut domain_merged = 0usize;

        if args.dry_run {
            for (record_type, indices) in &groups {
                if ctx.json {
                    json_domains.push(serde_json::json!({
                        "domain": domain,
                        "type": record_type.as_str(),
                        "count": indices.len(),
                        "action": "would_compact",
                    }));
                } else {
                    println!(
                        "  {}: {} {} records could be compacted",
                        domain,
                        indices.len(),
                        record_type
                    );
                }
            }
        } else if args.auto {
            // Auto-compact: for same-type records, keep the newest of each "name" group
            // For conventions, deduplicate exact content matches
            lock::with_file_lock(&file_path, || {
                let mut records = storage::read_expertise_file(&file_path)?;
                let original_len = records.len();

                // Deduplicate by content/name identity, keeping the newest
                let mut seen: HashMap<String, usize> = HashMap::new();
                let mut to_remove: Vec<usize> = Vec::new();

                for (i, r) in records.iter().enumerate() {
                    let key = match r {
                        ExpertiseRecord::Convention { content, .. } => {
                            format!("convention:{content}")
                        }
                        ExpertiseRecord::Pattern { name, .. } => format!("pattern:{name}"),
                        ExpertiseRecord::Failure { description, .. } => {
                            format!("failure:{description}")
                        }
                        ExpertiseRecord::Decision { title, .. } => format!("decision:{title}"),
                        ExpertiseRecord::Reference { name, .. } => format!("reference:{name}"),
                        ExpertiseRecord::Guide { name, .. } => format!("guide:{name}"),
                    };

                    if let Some(&prev_idx) = seen.get(&key) {
                        // Keep the one with the newer recorded_at
                        let prev_ts = records[prev_idx].recorded_at();
                        let curr_ts = r.recorded_at();
                        if curr_ts >= prev_ts {
                            to_remove.push(prev_idx);
                            seen.insert(key, i);
                        } else {
                            to_remove.push(i);
                        }
                    } else {
                        seen.insert(key, i);
                    }
                }

                to_remove.sort_unstable_by(|a, b| b.cmp(a));
                to_remove.dedup();
                for i in &to_remove {
                    records.remove(*i);
                }

                domain_merged = original_len - records.len();
                if domain_merged > 0 {
                    storage::write_expertise_file(&file_path, &mut records)?;
                }
                Ok(())
            })?;

            if domain_merged > 0 {
                total_merged += domain_merged;
                if ctx.json {
                    json_domains.push(serde_json::json!({
                        "domain": domain,
                        "merged": domain_merged,
                    }));
                } else {
                    println!(
                        "  {}: compacted {} duplicate record(s)",
                        domain, domain_merged
                    );
                }
            }
        }
    }

    if ctx.json {
        output_json(&serde_json::json!({
            "success": true,
            "command": "compact",
            "dry_run": args.dry_run,
            "total_merged": total_merged,
            "domains": json_domains,
        }));
    } else if args.dry_run {
        if total_merged == 0 && json_domains.is_empty() {
            print_success("No records to compact.");
        } else {
            print_warning("Dry run complete. Run without --dry-run to apply.");
        }
    } else if total_merged == 0 {
        print_success("No duplicate records found to compact.");
    } else {
        print_success(&format!("Compacted {} record(s) total.", total_merged));
    }

    Ok(())
}
