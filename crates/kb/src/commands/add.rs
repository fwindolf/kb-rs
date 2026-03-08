use anyhow::{Result, bail};

use crate::cli::AddArgs;
use crate::context::RuntimeContext;
use crate::output::*;
use kb_core::{config, storage};

pub fn run(ctx: &RuntimeContext, args: &AddArgs) -> Result<()> {
    config::ensure_kb_dir(&ctx.cwd)?;
    config::validate_domain_name(&args.domain)?;

    let mut cfg = config::read_config(&ctx.cwd)?;

    if cfg.domains.contains(&args.domain) {
        if ctx.json {
            output_json_error(
                "add",
                &format!("Domain \"{}\" already exists.", args.domain),
            );
            return Ok(());
        }
        bail!("Domain \"{}\" already exists.", args.domain);
    }

    cfg.domains.push(args.domain.clone());
    config::write_config(&cfg, &ctx.cwd)?;

    let file_path = config::get_expertise_path(&args.domain, &ctx.cwd)?;
    storage::create_expertise_file(&file_path)?;

    if ctx.json {
        output_json(&serde_json::json!({
            "success": true,
            "command": "add",
            "domain": args.domain,
        }));
    } else {
        print_success(&format!("Added domain \"{}\".", args.domain));
    }

    Ok(())
}
