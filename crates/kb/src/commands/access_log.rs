use anyhow::Result;

use crate::cli::AccessLogArgs;
use crate::context::RuntimeContext;
use crate::output::*;

pub fn run(ctx: &RuntimeContext, args: &AccessLogArgs) -> Result<()> {
    kb_core::config::ensure_kb_dir(&ctx.cwd)?;

    let filters = kb_core::access_log::AccessLogFilter {
        session_id: args.session.clone(),
        domain: args.domain.clone(),
        tool: args.tool_type.clone(),
    };

    let entries = kb_core::access_log::query_log(&ctx.cwd, &filters)?;

    if ctx.json {
        output_json(&serde_json::json!({
            "success": true,
            "command": "access-log",
            "count": entries.len(),
            "entries": entries,
        }));
    } else if entries.is_empty() {
        println!("No access log entries found.");
    } else {
        for entry in &entries {
            println!(
                "[{}] {} tool={} domain={} query={}",
                entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
                entry.session_id,
                entry.tool,
                entry.domain.as_deref().unwrap_or("-"),
                entry.query.as_deref().unwrap_or("-"),
            );
        }
        println!("\n{} entries total.", entries.len());
    }

    Ok(())
}
