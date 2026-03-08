use anyhow::Result;

use crate::cli::PruneArgs;
use crate::context::RuntimeContext;
use crate::output::*;
use kb_core::{config, health, lock, storage};

pub fn run(ctx: &RuntimeContext, args: &PruneArgs) -> Result<()> {
    config::ensure_kb_dir(&ctx.cwd)?;
    let cfg = config::read_config(&ctx.cwd)?;

    let now = chrono::Utc::now();
    let shelf_life = &cfg.classification_defaults.shelf_life;
    let mut total_pruned = 0usize;
    let mut json_domains: Vec<serde_json::Value> = Vec::new();

    for domain in &cfg.domains {
        let file_path = config::get_expertise_path(domain, &ctx.cwd)?;
        let records = storage::read_expertise_file(&file_path)?;

        let stale_indices: Vec<usize> = records
            .iter()
            .enumerate()
            .filter(|(_, r)| health::is_record_stale(r, now, shelf_life))
            .map(|(i, _)| i)
            .collect();

        if stale_indices.is_empty() {
            continue;
        }

        let stale_count = stale_indices.len();

        if args.dry_run {
            if ctx.json {
                let stale_ids: Vec<&str> = stale_indices
                    .iter()
                    .filter_map(|&i| records[i].id())
                    .collect();
                json_domains.push(serde_json::json!({
                    "domain": domain,
                    "stale_count": stale_count,
                    "stale_ids": stale_ids,
                }));
            } else {
                println!(
                    "  {}: {} stale record(s) would be pruned",
                    domain, stale_count
                );
                for &i in &stale_indices {
                    let r = &records[i];
                    let id = r.id().unwrap_or("?");
                    let summary = kb_core::format::get_record_summary(r);
                    println!("    {} {} ({})", id, r.record_type(), summary);
                }
            }
        } else {
            lock::with_file_lock(&file_path, || {
                let mut records = storage::read_expertise_file(&file_path)?;
                // Remove in reverse order to preserve indices
                let mut indices: Vec<usize> = records
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| health::is_record_stale(r, now, shelf_life))
                    .map(|(i, _)| i)
                    .collect();
                indices.sort_unstable_by(|a, b| b.cmp(a));
                for i in indices {
                    records.remove(i);
                }
                storage::write_expertise_file(&file_path, &mut records)?;
                Ok(())
            })?;

            if ctx.json {
                json_domains.push(serde_json::json!({
                    "domain": domain,
                    "pruned": stale_count,
                }));
            } else {
                println!("  {}: pruned {} stale record(s)", domain, stale_count);
            }
        }

        total_pruned += stale_count;
    }

    if ctx.json {
        output_json(&serde_json::json!({
            "success": true,
            "command": "prune",
            "dry_run": args.dry_run,
            "total_pruned": total_pruned,
            "domains": json_domains,
        }));
    } else if total_pruned == 0 {
        print_success("No stale records found.");
    } else if args.dry_run {
        print_warning(&format!(
            "{} stale record(s) would be pruned. Run without --dry-run to remove.",
            total_pruned
        ));
    } else {
        print_success(&format!("Pruned {} stale record(s).", total_pruned));
    }

    Ok(())
}
