use anyhow::Result;

use crate::cli::DoctorArgs;
use crate::context::RuntimeContext;
use crate::output::*;
use kb_core::types::ExpertiseRecord;
use kb_core::{config, storage};

pub fn run(ctx: &RuntimeContext, args: &DoctorArgs) -> Result<()> {
    config::ensure_kb_dir(&ctx.cwd)?;

    let mut issues: Vec<serde_json::Value> = Vec::new();
    let mut fixed: Vec<String> = Vec::new();

    // Check 1: config readable
    let cfg = match config::read_config(&ctx.cwd) {
        Ok(c) => {
            if !ctx.json {
                print_success("  Config: OK");
            }
            Some(c)
        }
        Err(e) => {
            let msg = format!("Config unreadable: {e}");
            issues.push(serde_json::json!({"check": "config", "error": msg}));
            if !ctx.json {
                print_error(&format!("  Config: {msg}"));
            }
            None
        }
    };

    if let Some(ref cfg) = cfg {
        // Check 2: each domain file parseable
        for domain in &cfg.domains {
            let file_path = match config::get_expertise_path(domain, &ctx.cwd) {
                Ok(p) => p,
                Err(e) => {
                    let msg = format!("Invalid domain path for \"{domain}\": {e}");
                    issues.push(
                        serde_json::json!({"check": "domain", "domain": domain, "error": msg}),
                    );
                    if !ctx.json {
                        print_error(&format!("  {domain}: {msg}"));
                    }
                    continue;
                }
            };

            if !file_path.exists() {
                let msg = format!("Missing file for domain \"{domain}\"");
                issues.push(
                    serde_json::json!({"check": "domain_file", "domain": domain, "error": msg}),
                );
                if !ctx.json {
                    print_error(&format!("  {domain}: {msg}"));
                }
                if args.fix {
                    if let Ok(()) = storage::create_expertise_file(&file_path) {
                        fixed.push(format!("Created missing file for \"{domain}\""));
                        if !ctx.json {
                            print_success(&format!("    Fixed: created {}", file_path.display()));
                        }
                    }
                }
                continue;
            }

            let content = std::fs::read_to_string(&file_path).unwrap_or_default();
            let mut line_errors = 0usize;
            let mut valid = 0usize;

            for (line_num, line) in content.lines().enumerate() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                match serde_json::from_str::<ExpertiseRecord>(trimmed) {
                    Ok(_) => valid += 1,
                    Err(e) => {
                        line_errors += 1;
                        let msg = format!("{}:{}: {}", domain, line_num + 1, e);
                        issues.push(serde_json::json!({
                            "check": "parse",
                            "domain": domain,
                            "line": line_num + 1,
                            "error": e.to_string(),
                        }));
                        if !ctx.json {
                            print_error(&format!("  {domain}: {msg}"));
                        }
                    }
                }
            }

            if line_errors == 0 && !ctx.json {
                print_success(&format!("  {domain}: {valid} records OK"));
            }

            // Fix: remove unparseable lines
            if args.fix && line_errors > 0 {
                let mut good_lines: Vec<String> = Vec::new();
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if serde_json::from_str::<ExpertiseRecord>(trimmed).is_ok() {
                        good_lines.push(trimmed.to_string());
                    }
                }
                let cleaned = good_lines.join("\n") + "\n";
                if std::fs::write(&file_path, cleaned).is_ok() {
                    fixed.push(format!(
                        "Removed {} bad line(s) from \"{domain}\"",
                        line_errors
                    ));
                    if !ctx.json {
                        print_success(&format!("    Fixed: removed {} bad line(s)", line_errors));
                    }
                }
            }
        }

        // Check 3: orphan files (files in expertise/ not referenced by config)
        let expertise_dir = config::get_expertise_dir(&ctx.cwd);
        if expertise_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&expertise_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                        if !cfg.domains.contains(&stem.to_string()) {
                            let msg = format!("Orphan file: {}", path.display());
                            issues.push(serde_json::json!({
                                "check": "orphan",
                                "file": path.display().to_string(),
                            }));
                            if !ctx.json {
                                print_warning(&format!("  {msg}"));
                            }
                            if args.fix && std::fs::remove_file(&path).is_ok() {
                                fixed.push(format!("Removed orphan: {}", path.display()));
                                if !ctx.json {
                                    print_success(&format!(
                                        "    Fixed: removed {}",
                                        path.display()
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if ctx.json {
        output_json(&serde_json::json!({
            "success": issues.is_empty(),
            "command": "doctor",
            "issues": issues,
            "fixed": fixed,
        }));
    } else if issues.is_empty() {
        print_success("No issues found.");
    } else {
        let issue_count = issues.len();
        let fixed_count = fixed.len();
        if args.fix {
            println!("Found {} issue(s), fixed {}.", issue_count, fixed_count);
        } else {
            println!(
                "Found {} issue(s). Run with --fix to attempt repairs.",
                issue_count
            );
        }
    }

    Ok(())
}
