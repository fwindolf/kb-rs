use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::cli::OnboardArgs;
use crate::context::RuntimeContext;
use crate::output::*;
use kb_core::markers;

const ONBOARD_CONTENT: &str = r#"## Knowledge
This project uses kb to build knowledge across different domains in this project.
At session start, use `kb prime` or `kb prime --files <path>` to regain the knowledge.
After compaction or context clear, re-prime with `kb prime` to restore context.
At session end, or when an important decision is made, use `kb learn` to remember how to persist knowledge.
Use `kb record <domain> --type <convention|pattern|decision|failure|reference|guide> --description <...>` and `kb sync` to persist.
Record WHY and WHICH approach, not implementation details readable from code. Use stable references (doc paths, module names), not line numbers."#;

/// Auto-discovery order for target files.
const TARGET_FILES: &[&str] = &[
    "AGENTS.md",
    "CLAUDE.md",
    ".github/copilot-instructions.md",
    "CODEX.md",
    ".opencode/instructions.md",
];

fn build_onboard_section() -> String {
    markers::wrap_in_markers(&format!("{ONBOARD_CONTENT}\n"))
}

/// Resolve which file to target based on flags or auto-discovery.
fn resolve_target_file(args: &OnboardArgs, cwd: &std::path::Path) -> PathBuf {
    if args.agents {
        return cwd.join("AGENTS.md");
    }
    if args.claude {
        return cwd.join("CLAUDE.md");
    }
    if args.copilot {
        return cwd.join(".github/copilot-instructions.md");
    }
    if args.codex {
        return cwd.join("CODEX.md");
    }
    if args.opencode {
        return cwd.join(".opencode/instructions.md");
    }
    // Auto-discover: pick the first existing file
    for target in TARGET_FILES {
        let path = cwd.join(target);
        if path.exists() {
            return path;
        }
    }
    // Default
    cwd.join("AGENTS.md")
}

fn target_display_name(path: &std::path::Path, cwd: &std::path::Path) -> String {
    path.strip_prefix(cwd)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

/// Check if the onboard section is installed.
fn check_onboard(ctx: &RuntimeContext, args: &OnboardArgs) -> Result<()> {
    let target_path = resolve_target_file(args, &ctx.cwd);
    let name = target_display_name(&target_path, &ctx.cwd);

    if !target_path.exists() {
        if ctx.json {
            output_json(&serde_json::json!({
                "success": false,
                "command": "onboard",
                "action": "check",
                "installed": false,
                "reason": "file_not_found",
                "file": name,
            }));
        } else {
            println!("\u{2717} {} not found", name);
        }
        anyhow::bail!("{} not found", name);
    }

    let content = fs::read_to_string(&target_path).context(format!("Failed to read {name}"))?;

    if markers::has_marker_section(&content) {
        if ctx.json {
            output_json(&serde_json::json!({
                "success": true,
                "command": "onboard",
                "action": "check",
                "installed": true,
                "file": name,
            }));
        } else {
            println!("\u{2713} Onboard section installed in {name}");
        }
        Ok(())
    } else {
        if ctx.json {
            output_json(&serde_json::json!({
                "success": false,
                "command": "onboard",
                "action": "check",
                "installed": false,
                "reason": "no_marker_section",
                "file": name,
            }));
        } else {
            println!("\u{2717} No onboard section in {name}");
        }
        anyhow::bail!("no onboard section in {}", name);
    }
}

/// Remove the onboard section from the target file.
fn remove_onboard(ctx: &RuntimeContext, args: &OnboardArgs) -> Result<()> {
    let target_path = resolve_target_file(args, &ctx.cwd);
    let name = target_display_name(&target_path, &ctx.cwd);

    if !target_path.exists() {
        if ctx.json {
            output_json(&serde_json::json!({
                "success": true,
                "command": "onboard",
                "action": "remove",
                "result": "no_file",
                "file": name,
            }));
        } else {
            println!("No {} found", name);
        }
        return Ok(());
    }

    let content = fs::read_to_string(&target_path).context(format!("Failed to read {name}"))?;

    if !markers::has_marker_section(&content) {
        if ctx.json {
            output_json(&serde_json::json!({
                "success": true,
                "command": "onboard",
                "action": "remove",
                "result": "no_section",
                "file": name,
            }));
        } else {
            println!("No onboard section found in {name}");
        }
        return Ok(());
    }

    let new_content = markers::remove_marker_section(&content);
    if new_content.trim().is_empty() {
        fs::remove_file(&target_path).context(format!("Failed to remove {name}"))?;
        if ctx.json {
            output_json(&serde_json::json!({
                "success": true,
                "command": "onboard",
                "action": "remove",
                "result": "file_deleted",
                "file": name,
            }));
        } else {
            println!(
                "Removed {} (file was empty after removing onboard section)",
                name
            );
        }
    } else {
        fs::write(&target_path, &new_content).context(format!("Failed to write {name}"))?;
        if ctx.json {
            output_json(&serde_json::json!({
                "success": true,
                "command": "onboard",
                "action": "remove",
                "result": "section_removed",
                "file": name,
            }));
        } else {
            println!("Removed onboard section from {name}");
        }
    }

    Ok(())
}

pub fn run(ctx: &RuntimeContext, args: &OnboardArgs) -> Result<()> {
    if args.check {
        return check_onboard(ctx, args);
    }

    if args.remove {
        return remove_onboard(ctx, args);
    }

    let target_path = resolve_target_file(args, &ctx.cwd);
    let name = target_display_name(&target_path, &ctx.cwd);
    let section = build_onboard_section();

    if target_path.exists() {
        let content = fs::read_to_string(&target_path).context(format!("Failed to read {name}"))?;

        if markers::has_marker_section(&content) {
            if args.update {
                if let Some(updated) = markers::replace_marker_section(&content, &section) {
                    fs::write(&target_path, updated)?;
                    if ctx.json {
                        output_json(&serde_json::json!({
                            "success": true,
                            "command": "onboard",
                            "action": "updated",
                            "file": name,
                        }));
                    } else {
                        print_success(&format!("Updated onboard section in {name}."));
                    }
                    return Ok(());
                }
            }
            if ctx.json {
                output_json(&serde_json::json!({
                    "success": true,
                    "command": "onboard",
                    "action": "already_exists",
                    "file": name,
                }));
            } else {
                println!("Onboard section already exists in {name}. Use --update to replace it.");
            }
            return Ok(());
        }

        // Append section
        let updated = format!("{}\n\n{}\n", content.trim_end(), section);
        fs::write(&target_path, updated)?;
    } else {
        // Create parent dir if needed
        if let Some(parent) = target_path.parent() {
            if parent != std::path::Path::new("") && parent != ctx.cwd {
                fs::create_dir_all(parent)
                    .context(format!("Failed to create directory {}", parent.display()))?;
            }
        }
        fs::write(&target_path, format!("{section}\n"))?;
    }

    if ctx.json {
        output_json(&serde_json::json!({
            "success": true,
            "command": "onboard",
            "action": "created",
            "file": name,
        }));
    } else {
        print_success(&format!("Wrote onboarding to {name}."));
    }

    Ok(())
}
