use anyhow::{Result, bail};

use crate::cli::RemoveArgs;
use crate::context::RuntimeContext;
use crate::output::*;
use kb_core::config;

pub fn run(ctx: &RuntimeContext, args: &RemoveArgs) -> Result<()> {
    config::ensure_kb_dir(&ctx.cwd)?;
    let mut cfg = config::read_config(&ctx.cwd)?;

    if !cfg.domains.contains(&args.domain) {
        if ctx.json {
            output_json_error(
                "remove",
                &format!("Domain \"{}\" does not exist.", args.domain),
            );
            return Ok(());
        }
        bail!("Domain \"{}\" does not exist.", args.domain);
    }

    let file_path = config::get_expertise_path(&args.domain, &ctx.cwd)?;

    // Check record count for confirmation
    if file_path.exists() && !args.force {
        let records = kb_core::storage::read_expertise_file(&file_path)?;
        if !records.is_empty() {
            if ctx.json {
                output_json_error(
                    "remove",
                    &format!(
                        "Domain \"{}\" has {} record(s). Use --force to remove.",
                        args.domain,
                        records.len()
                    ),
                );
                return Ok(());
            }
            bail!(
                "Domain \"{}\" has {} record(s). Use --force to remove.",
                args.domain,
                records.len()
            );
        }
    }

    // Remove from config
    cfg.domains.retain(|d| d != &args.domain);
    config::write_config(&cfg, &ctx.cwd)?;

    // Delete the JSONL file
    if file_path.exists() {
        std::fs::remove_file(&file_path)?;
    }

    if ctx.json {
        output_json(&serde_json::json!({
            "success": true,
            "command": "remove",
            "domain": args.domain,
        }));
    } else {
        print_success(&format!("Removed domain \"{}\".", args.domain));
    }

    Ok(())
}
