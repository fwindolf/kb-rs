use anyhow::Result;

use crate::cli::DeleteArgs;
use crate::context::RuntimeContext;
use crate::output::*;
use kb_core::{config, lock, resolve, storage};

pub fn run(ctx: &RuntimeContext, args: &DeleteArgs) -> Result<()> {
    config::ensure_kb_dir(&ctx.cwd)?;
    let cfg = config::read_config(&ctx.cwd)?;
    config::ensure_domain_exists(&cfg, &args.domain)?;

    let file_path = config::get_expertise_path(&args.domain, &ctx.cwd)?;
    let records = storage::read_expertise_file(&file_path)?;
    let (idx, matched) = resolve::resolve_record_id(&records, &args.id)?;
    let record_id = matched.id().unwrap_or("unknown").to_string();
    let summary = kb_core::format::get_record_summary(matched);

    lock::with_file_lock(&file_path, || {
        let mut records = storage::read_expertise_file(&file_path)?;
        records.remove(idx);
        storage::write_expertise_file(&file_path, &mut records)?;
        Ok(())
    })?;

    if ctx.json {
        output_json(&serde_json::json!({
            "success": true,
            "command": "delete",
            "domain": args.domain,
            "id": record_id,
        }));
    } else {
        print_success(&format!(
            "Deleted record {} (\"{}\") from \"{}\".",
            record_id, summary, args.domain
        ));
    }

    Ok(())
}
