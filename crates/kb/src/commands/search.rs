use anyhow::{Result, bail};

use crate::cli::SearchArgs;
use crate::context::RuntimeContext;
use crate::output::*;
use kb_core::types::*;
use kb_core::{config, format, scoring, search, storage};

pub fn run(ctx: &RuntimeContext, args: &SearchArgs) -> Result<()> {
    config::ensure_kb_dir(&ctx.cwd)?;
    let cfg = config::read_config(&ctx.cwd)?;

    let query = match args.query {
        Some(ref q) => q.clone(),
        None => bail!("Search query is required."),
    };

    let domains: Vec<String> = if let Some(ref domain) = args.domain {
        config::ensure_domain_exists(&cfg, domain)?;
        vec![domain.clone()]
    } else {
        cfg.domains.clone()
    };

    let mut json_domains: Vec<serde_json::Value> = Vec::new();
    let mut sections: Vec<String> = Vec::new();
    let mut total_matches = 0usize;

    for domain in &domains {
        let file_path = config::get_expertise_path(domain, &ctx.cwd)?;
        let records = storage::read_expertise_file(&file_path)?;
        let last_updated = records.iter().map(|r| r.recorded_at().to_string()).max();

        // Apply pre-search filters
        let mut filtered: Vec<ExpertiseRecord> = records;

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
            filtered.retain(|r| r.record_type() == record_type);
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

        if let Some(ref tag) = args.tag {
            let tag_lower = tag.to_lowercase();
            filtered.retain(|r| {
                r.tags()
                    .map(|tags| tags.iter().any(|t| t.to_lowercase() == tag_lower))
                    .unwrap_or(false)
            });
        }

        if let Some(ref file) = args.file {
            let file_lower = file.to_lowercase();
            filtered.retain(|r| {
                r.files()
                    .map(|files| files.iter().any(|f| f.to_lowercase().contains(&file_lower)))
                    .unwrap_or(false)
            });
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

        // BM25 search
        let matches: Vec<ExpertiseRecord> = search::search_records(&filtered, &query)
            .into_iter()
            .cloned()
            .collect();

        if args.sort_by_score {
            let mut refs: Vec<&ExpertiseRecord> = matches.iter().collect();
            scoring::sort_by_confirmation_score(&mut refs);
        }

        if matches.is_empty() {
            continue;
        }

        total_matches += matches.len();

        if ctx.json {
            json_domains.push(serde_json::json!({
                "domain": domain,
                "matches": matches,
            }));
        } else {
            let output =
                format::format_domain_expertise(domain, &matches, last_updated.as_deref(), false);
            sections.push(output);
        }
    }

    if ctx.json {
        output_json(&serde_json::json!({
            "success": true,
            "command": "search",
            "query": query,
            "total": total_matches,
            "domains": json_domains,
        }));
    } else if sections.is_empty() {
        println!("No records matching \"{}\" found.", query);
    } else {
        println!("{}", sections.join("\n\n"));
        let suffix = if total_matches == 1 { "" } else { "es" };
        println!("\n{total_matches} match{suffix} found.");
    }

    Ok(())
}
