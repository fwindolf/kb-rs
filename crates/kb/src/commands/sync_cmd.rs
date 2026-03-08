use anyhow::{Result, bail};
use std::process::Command;

use crate::cli::SyncArgs;
use crate::context::RuntimeContext;
use crate::output::*;
use kb_core::{config, git};

pub fn run(ctx: &RuntimeContext, args: &SyncArgs) -> Result<()> {
    config::ensure_kb_dir(&ctx.cwd)?;

    if !git::is_git_repo(&ctx.cwd) {
        if ctx.json {
            output_json_error("sync", "Not in a git repository.");
            return Ok(());
        }
        bail!("Not in a git repository. `kb sync` requires git.");
    }

    // Validate unless --no-validate
    if !args.no_validate {
        let cfg = config::read_config(&ctx.cwd)?;
        let mut has_errors = false;

        for domain in &cfg.domains {
            let file_path = config::get_expertise_path(domain, &ctx.cwd)?;
            let content = std::fs::read_to_string(&file_path).unwrap_or_default();

            for (line_num, line) in content.lines().enumerate() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if serde_json::from_str::<kb_core::types::ExpertiseRecord>(trimmed).is_err() {
                    has_errors = true;
                    if !ctx.json {
                        print_error(&format!(
                            "Validation error in {}:{}: invalid record",
                            domain,
                            line_num + 1
                        ));
                    }
                }
            }
        }

        if has_errors {
            if ctx.json {
                output_json_error(
                    "sync",
                    "Validation failed. Fix errors or use --no-validate.",
                );
                return Ok(());
            }
            bail!("Validation failed. Fix errors or use --no-validate.");
        }

        if !ctx.json {
            print_success("Validation passed.");
        }
    }

    // Git add .kb/
    let add_output = Command::new("git")
        .args(["add", ".kb/"])
        .current_dir(&ctx.cwd)
        .output()?;

    if !add_output.status.success() {
        let stderr = String::from_utf8_lossy(&add_output.stderr);
        if ctx.json {
            output_json_error("sync", &format!("git add failed: {stderr}"));
            return Ok(());
        }
        bail!("git add .kb/ failed: {stderr}");
    }

    // Check if there's anything to commit
    let diff_output = Command::new("git")
        .args(["diff", "--cached", "--name-only", "--", ".kb/"])
        .current_dir(&ctx.cwd)
        .output()?;

    let staged_files = String::from_utf8_lossy(&diff_output.stdout);
    if staged_files.trim().is_empty() {
        if ctx.json {
            output_json(&serde_json::json!({
                "success": true,
                "command": "sync",
                "action": "nothing_to_commit",
            }));
        } else {
            println!("No .kb/ changes to commit.");
        }
        return Ok(());
    }

    // Git commit
    let message = args
        .message
        .clone()
        .unwrap_or_else(|| "chore: sync kb expertise".to_string());

    let commit_output = Command::new("git")
        .args(["commit", "-m", &message, "--", ".kb/"])
        .current_dir(&ctx.cwd)
        .output()?;

    if !commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        if ctx.json {
            output_json_error("sync", &format!("git commit failed: {stderr}"));
            return Ok(());
        }
        bail!("git commit failed: {stderr}");
    }

    if ctx.json {
        output_json(&serde_json::json!({
            "success": true,
            "command": "sync",
            "action": "committed",
            "message": message,
        }));
    } else {
        print_success(&format!("Committed .kb/ changes: {message}"));
    }

    Ok(())
}
