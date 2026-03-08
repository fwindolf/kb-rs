use anyhow::Result;

use crate::context::RuntimeContext;
use crate::output::*;
use kb_core::format::{self, DomainStat};
use kb_core::{config, storage};

pub fn run(ctx: &RuntimeContext) -> Result<()> {
    config::ensure_kb_dir(&ctx.cwd)?;
    let cfg = config::read_config(&ctx.cwd)?;

    let mut stats: Vec<DomainStat> = Vec::new();

    for domain in &cfg.domains {
        let file_path = config::get_expertise_path(domain, &ctx.cwd)?;
        let records = storage::read_expertise_file(&file_path)?;
        let last_updated = records.iter().map(|r| r.recorded_at().to_string()).max();
        stats.push(DomainStat {
            domain: domain.clone(),
            count: records.len(),
            last_updated,
        });
    }

    if ctx.json {
        let domain_data: Vec<serde_json::Value> = stats
            .iter()
            .map(|s| {
                serde_json::json!({
                    "domain": s.domain,
                    "count": s.count,
                    "last_updated": s.last_updated,
                })
            })
            .collect();
        output_json(&serde_json::json!({
            "success": true,
            "command": "status",
            "domains": domain_data,
        }));
    } else {
        println!("{}", format::format_status_output(&stats, &cfg.governance));
    }

    Ok(())
}
