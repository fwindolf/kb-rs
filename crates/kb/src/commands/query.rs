use anyhow::{Result, bail};

use crate::cli::QueryArgs;
use crate::context::RuntimeContext;
use crate::output::*;
use kb_core::types::*;
use kb_core::{config, filter, format, scoring, storage};

pub fn run(ctx: &RuntimeContext, args: &QueryArgs) -> Result<()> {
    config::ensure_kb_dir(&ctx.cwd)?;
    let cfg = config::read_config(&ctx.cwd)?;

    let domains: Vec<String> = if let Some(ref domain) = args.domain {
        config::ensure_domain_exists(&cfg, domain)?;
        vec![domain.clone()]
    } else if args.all || cfg.domains.len() == 1 {
        cfg.domains.clone()
    } else {
        bail!("Specify a domain or use --all to query all domains.");
    };

    let mut json_domains: Vec<serde_json::Value> = Vec::new();
    let mut sections: Vec<String> = Vec::new();

    for domain in &domains {
        let file_path = config::get_expertise_path(domain, &ctx.cwd)?;
        let records = storage::read_expertise_file(&file_path)?;

        // Apply filters
        let mut filtered: Vec<&ExpertiseRecord> = records.iter().collect();

        if let Some(ref rt) = args.record_type {
            let record_type = match rt.as_str() {
                "convention" => RecordType::Convention,
                "pattern" => RecordType::Pattern,
                "failure" => RecordType::Failure,
                "decision" => RecordType::Decision,
                "reference" => RecordType::Reference,
                "guide" => RecordType::Guide,
                other => bail!("Unknown record type: {other}"),
            };
            filtered = filter::filter_by_type(&records, record_type);
        }

        if let Some(ref cls_str) = args.classification {
            let cls = match cls_str.as_str() {
                "foundational" => Classification::Foundational,
                "tactical" => Classification::Tactical,
                "observational" => Classification::Observational,
                other => bail!("Unknown classification: {other}"),
            };
            filtered.retain(|r| r.classification() == cls);
        }

        if let Some(ref file) = args.file {
            let file_matches = filter::filter_by_file(&records, file);
            let file_ids: std::collections::HashSet<Option<&str>> =
                file_matches.iter().map(|r| r.id()).collect();
            filtered.retain(|r| file_ids.contains(&r.id()));
        }

        if let Some(ref os) = args.outcome_status {
            let target_status = match os.as_str() {
                "success" => OutcomeStatus::Success,
                "failure" => OutcomeStatus::Failure,
                "partial" => OutcomeStatus::Partial,
                other => bail!("Unknown outcome status: {other}"),
            };
            filtered.retain(|r| {
                r.outcomes()
                    .map(|outcomes| outcomes.iter().any(|o| o.status == target_status))
                    .unwrap_or(false)
            });
        }

        if args.sort_by_score {
            scoring::sort_by_confirmation_score(&mut filtered);
        }

        if ctx.json {
            let record_values: Vec<&ExpertiseRecord> = filtered.to_vec();
            json_domains.push(serde_json::json!({
                "domain": domain,
                "records": record_values,
            }));
        } else {
            let last_updated = records.iter().map(|r| r.recorded_at().to_string()).max();
            let owned: Vec<ExpertiseRecord> = filtered.into_iter().cloned().collect();
            let output =
                format::format_domain_expertise(domain, &owned, last_updated.as_deref(), false);
            sections.push(output);
        }
    }

    if ctx.json {
        output_json(&serde_json::json!({
            "success": true,
            "command": "query",
            "domains": json_domains,
        }));
    } else if !sections.is_empty() {
        println!("{}", sections.join("\n\n"));
    }

    Ok(())
}
