use anyhow::Result;

use crate::cli::SessionCommands;
use crate::context::RuntimeContext;
use crate::output::*;

pub fn run(ctx: &RuntimeContext, cmd: &SessionCommands) -> Result<()> {
    kb_core::config::ensure_kb_dir(&ctx.cwd)?;

    match cmd {
        SessionCommands::List => run_list(ctx),
        SessionCommands::Show(args) => run_show(ctx, &args.id),
    }
}

fn run_list(ctx: &RuntimeContext) -> Result<()> {
    let sessions = kb_core::session::list_sessions(&ctx.cwd)?;

    if ctx.json {
        output_json(&serde_json::json!({
            "success": true,
            "command": "session list",
            "count": sessions.len(),
            "sessions": sessions,
        }));
    } else if sessions.is_empty() {
        println!("No sessions found.");
    } else {
        for s in &sessions {
            let status = if s.ended_at.is_some() {
                "ended"
            } else {
                "active"
            };
            println!(
                "{} [{}] {} started={}",
                s.id,
                status,
                s.label.as_deref().unwrap_or("(no label)"),
                s.started_at.format("%Y-%m-%d %H:%M:%S"),
            );
        }
    }

    Ok(())
}

fn run_show(ctx: &RuntimeContext, id: &str) -> Result<()> {
    let session = kb_core::session::get_session(&ctx.cwd, id)?;
    let access_entries = kb_core::access_log::query_log(
        &ctx.cwd,
        &kb_core::access_log::AccessLogFilter {
            session_id: Some(id.to_string()),
            ..Default::default()
        },
    )?;

    if ctx.json {
        output_json(&serde_json::json!({
            "success": true,
            "command": "session show",
            "session": session,
            "access_log": access_entries,
        }));
    } else {
        let status = if session.ended_at.is_some() {
            "ended"
        } else {
            "active"
        };
        println!("Session: {}", session.id);
        println!("Status:  {status}");
        if let Some(ref label) = session.label {
            println!("Label:   {label}");
        }
        println!(
            "Started: {}",
            session.started_at.format("%Y-%m-%d %H:%M:%S")
        );
        if let Some(ref ended) = session.ended_at {
            println!("Ended:   {}", ended.format("%Y-%m-%d %H:%M:%S"));
        }

        if access_entries.is_empty() {
            println!("\nNo access log entries for this session.");
        } else {
            println!("\nAccess log ({} entries):", access_entries.len());
            for entry in &access_entries {
                println!(
                    "  [{}] tool={} domain={} query={}",
                    entry.timestamp.format("%H:%M:%S"),
                    entry.tool,
                    entry.domain.as_deref().unwrap_or("-"),
                    entry.query.as_deref().unwrap_or("-"),
                );
            }
        }
    }

    Ok(())
}
