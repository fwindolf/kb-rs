use anyhow::{Result, bail};

use crate::cli::ReadyArgs;
use crate::context::RuntimeContext;
use crate::output::*;
use kb_core::{config, format, storage};

/// Parse a human-friendly duration string like "24h", "7d", "2w" into a chrono Duration.
fn parse_duration(s: &str) -> Result<chrono::Duration> {
    let s = s.trim();
    if s.is_empty() {
        bail!("Empty duration string");
    }

    let (num_str, unit) = s.split_at(s.len() - 1);
    let num: i64 = num_str
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid duration number: \"{}\"", num_str))?;

    match unit {
        "h" => Ok(chrono::Duration::hours(num)),
        "d" => Ok(chrono::Duration::days(num)),
        "w" => Ok(chrono::Duration::weeks(num)),
        "m" => Ok(chrono::Duration::minutes(num)),
        _ => bail!(
            "Unknown duration unit \"{}\". Use h (hours), d (days), w (weeks), m (minutes).",
            unit
        ),
    }
}

pub fn run(ctx: &RuntimeContext, args: &ReadyArgs) -> Result<()> {
    config::ensure_kb_dir(&ctx.cwd)?;
    let cfg = config::read_config(&ctx.cwd)?;

    let domains: Vec<String> = if let Some(ref domain) = args.domain {
        config::ensure_domain_exists(&cfg, domain)?;
        vec![domain.clone()]
    } else {
        cfg.domains.clone()
    };

    // Collect all records with domain info, sorted by recorded_at descending
    let mut all_records: Vec<(String, kb_core::types::ExpertiseRecord)> = Vec::new();

    for domain in &domains {
        let file_path = config::get_expertise_path(domain, &ctx.cwd)?;
        let records = storage::read_expertise_file(&file_path)?;
        for r in records {
            all_records.push((domain.clone(), r));
        }
    }

    // Sort by recorded_at descending (newest first)
    all_records.sort_by(|a, b| b.1.recorded_at().cmp(a.1.recorded_at()));

    // Apply --since filter
    if let Some(ref since_str) = args.since {
        let dur = parse_duration(since_str)?;
        let cutoff = chrono::Utc::now() - dur;
        let cutoff_str = cutoff.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        all_records.retain(|(_, r)| r.recorded_at() >= cutoff_str.as_str());
    }

    // Limit
    all_records.truncate(args.limit);

    if ctx.json {
        let json_records: Vec<serde_json::Value> = all_records
            .iter()
            .map(|(domain, r)| {
                serde_json::json!({
                    "domain": domain,
                    "record": r,
                })
            })
            .collect();
        output_json(&serde_json::json!({
            "success": true,
            "command": "ready",
            "count": json_records.len(),
            "records": json_records,
        }));
    } else if all_records.is_empty() {
        println!("No recent records found.");
    } else {
        println!("Recent records ({}):", all_records.len());
        println!();
        for (domain, r) in &all_records {
            let id = r.id().unwrap_or("?");
            let summary = format::get_record_summary(r);
            let time_ago = format::format_time_ago(r.recorded_at());
            println!(
                "  [{domain}] {id} {} - {} ({time_ago})",
                r.record_type(),
                summary,
            );
        }
    }

    Ok(())
}
