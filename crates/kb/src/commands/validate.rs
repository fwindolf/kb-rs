use anyhow::Result;

use crate::context::RuntimeContext;
use crate::output::*;
use kb_core::config;
use kb_core::types::ExpertiseRecord;

pub fn run(ctx: &RuntimeContext) -> Result<()> {
    config::ensure_kb_dir(&ctx.cwd)?;
    let cfg = config::read_config(&ctx.cwd)?;

    let mut all_errors: Vec<serde_json::Value> = Vec::new();
    let mut total_records = 0usize;
    let mut total_errors = 0usize;

    for domain in &cfg.domains {
        let file_path = config::get_expertise_path(domain, &ctx.cwd)?;
        let content = std::fs::read_to_string(&file_path).unwrap_or_default();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            total_records += 1;
            let line_number = line_num + 1;

            match serde_json::from_str::<ExpertiseRecord>(trimmed) {
                Ok(_) => {}
                Err(e) => {
                    total_errors += 1;
                    let msg = format!(
                        "{}:{} - Schema validation failed: {}",
                        domain, line_number, e
                    );
                    all_errors.push(serde_json::json!({
                        "domain": domain,
                        "line": line_number,
                        "message": e.to_string(),
                    }));
                    if !ctx.json {
                        print_error(&msg);
                    }
                }
            }
        }
    }

    if ctx.json {
        output_json(&serde_json::json!({
            "success": total_errors == 0,
            "command": "validate",
            "valid": total_errors == 0,
            "totalRecords": total_records,
            "totalErrors": total_errors,
            "errors": all_errors,
        }));
    } else if total_errors > 0 {
        print_error(&format!(
            "{} records validated, {} errors found",
            total_records, total_errors
        ));
    } else {
        print_success(&format!(
            "{} records validated, {} errors found",
            total_records, total_errors
        ));
    }

    Ok(())
}
