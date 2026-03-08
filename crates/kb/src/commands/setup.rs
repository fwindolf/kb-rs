use anyhow::{Context, Result, bail};
use std::fs;
use std::path::PathBuf;

use crate::cli::SetupArgs;
use crate::context::RuntimeContext;
use crate::output::*;

const POST_COMMIT_HOOK: &str = r#"#!/bin/sh
# kb: suggest recording learnings after each commit
kb learn --since HEAD~1 2>/dev/null || true
"#;

fn install_git_hook(cwd: &std::path::Path) -> Result<()> {
    let hooks_dir = cwd.join(".git").join("hooks");
    if !hooks_dir.is_dir() {
        bail!("No .git/hooks directory found. Is this a git repository?");
    }

    let hook_path = hooks_dir.join("post-commit");

    if hook_path.exists() {
        let existing = fs::read_to_string(&hook_path)?;
        if existing.contains("kb learn") {
            return Ok(()); // Already installed
        }
        // Append to existing hook
        let updated = format!("{}\n{}", existing.trim_end(), POST_COMMIT_HOOK.trim());
        fs::write(&hook_path, updated)?;
    } else {
        fs::write(&hook_path, POST_COMMIT_HOOK)?;
    }

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o755);
        fs::set_permissions(&hook_path, perms)?;
    }

    Ok(())
}

fn get_provider_config(provider: &str) -> Result<(PathBuf, String)> {
    let (path, content) = match provider {
        "claude" => (
            PathBuf::from(".claude").join("settings.json"),
            serde_json::json!({
                "customInstructions": "Run `kb prime` at the start of each session to load project expertise. Before ending a session, run `kb learn` and `kb record` to capture new learnings, then `kb sync`."
            })
            .to_string(),
        ),
        "cursor" => (
            PathBuf::from(".cursor").join("rules"),
            "Run `kb prime` at the start of each session to load project expertise.\nBefore ending a session, run `kb learn` and `kb record` to capture new learnings, then `kb sync`.".to_string(),
        ),
        "codex" => (
            PathBuf::from("codex.md"),
            "## KB Integration\n\nRun `kb prime` at the start of each session to load project expertise.\nBefore ending a session, run `kb learn` and `kb record` to capture new learnings, then `kb sync`.".to_string(),
        ),
        "gemini" => (
            PathBuf::from(".gemini").join("settings.json"),
            serde_json::json!({
                "systemInstructions": "Run `kb prime` at the start of each session to load project expertise. Before ending a session, run `kb learn` and `kb record` to capture new learnings, then `kb sync`."
            })
            .to_string(),
        ),
        "windsurf" => (
            PathBuf::from(".windsurf").join("rules"),
            "Run `kb prime` at the start of each session to load project expertise.\nBefore ending a session, run `kb learn` and `kb record` to capture new learnings, then `kb sync`.".to_string(),
        ),
        "aider" => (
            PathBuf::from(".aider.conf.yml"),
            "# KB integration\nread:\n  - .kb/expertise/*.jsonl\n".to_string(),
        ),
        other => bail!("Unknown provider: {other}"),
    };

    Ok((path, content))
}

pub fn run(ctx: &RuntimeContext, args: &SetupArgs) -> Result<()> {
    let mut actions: Vec<String> = Vec::new();

    if args.git_hook {
        install_git_hook(&ctx.cwd)?;
        actions.push("Installed post-commit git hook.".to_string());
    }

    if let Some(ref provider) = args.provider {
        let (rel_path, content) = get_provider_config(provider)?;
        let full_path = ctx.cwd.join(&rel_path);

        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        fs::write(&full_path, &content)
            .with_context(|| format!("Failed to write {}", full_path.display()))?;

        actions.push(format!(
            "Wrote {} config to {}",
            provider,
            rel_path.display()
        ));
    }

    if actions.is_empty() {
        if ctx.json {
            output_json_error(
                "setup",
                "No action specified. Use --git-hook or --provider.",
            );
            return Ok(());
        }
        bail!("No action specified. Use --git-hook or --provider.");
    }

    if ctx.json {
        output_json(&serde_json::json!({
            "success": true,
            "command": "setup",
            "actions": actions,
        }));
    } else {
        for action in &actions {
            print_success(action);
        }
    }

    Ok(())
}
