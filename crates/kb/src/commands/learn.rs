use anyhow::{Result, bail};
use std::collections::HashMap;

use crate::cli::LearnArgs;
use crate::context::RuntimeContext;
use crate::output::*;
use kb_core::{config, git, storage};

pub fn run(ctx: &RuntimeContext, args: &LearnArgs) -> Result<()> {
    config::ensure_kb_dir(&ctx.cwd)?;

    if !git::is_git_repo(&ctx.cwd) {
        if ctx.json {
            output_json_error("learn", "Not in a git repository.");
            return Ok(());
        }
        bail!("Not in a git repository. `kb learn` requires git.");
    }

    let cfg = config::read_config(&ctx.cwd)?;
    let changed_files = git::get_changed_files(&ctx.cwd, &args.since);

    if changed_files.is_empty() {
        if ctx.json {
            output_json(&serde_json::json!({
                "success": true,
                "command": "learn",
                "changed_files": 0,
                "suggestions": [],
            }));
        } else {
            println!("No changed files found since {}.", args.since);
        }
        return Ok(());
    }

    // Build a map of domain -> file patterns from existing records
    let mut domain_files: HashMap<String, Vec<String>> = HashMap::new();
    for domain in &cfg.domains {
        let file_path = config::get_expertise_path(domain, &ctx.cwd)?;
        let records = storage::read_expertise_file(&file_path)?;
        let mut files: Vec<String> = Vec::new();
        for r in &records {
            if let Some(record_files) = r.files() {
                files.extend(record_files.iter().cloned());
            }
        }
        domain_files.insert(domain.clone(), files);
    }

    // Group changed files by best-matching domain
    let mut suggestions: HashMap<String, Vec<String>> = HashMap::new();
    let mut unmatched: Vec<String> = Vec::new();

    for changed in &changed_files {
        // Skip .kb/ files
        if changed.starts_with(".kb/") || changed.starts_with(".kb\\") {
            continue;
        }

        let mut best_domain: Option<String> = None;

        for (domain, files) in &domain_files {
            if git::file_matches_any(changed, files) {
                best_domain = Some(domain.clone());
                break;
            }
        }

        // Heuristic: match by directory prefix / domain name substring
        if best_domain.is_none() {
            let changed_lower = changed.to_lowercase();
            for domain in &cfg.domains {
                let domain_lower = domain.to_lowercase();
                if changed_lower.contains(&domain_lower) {
                    best_domain = Some(domain.clone());
                    break;
                }
            }
        }

        match best_domain {
            Some(d) => suggestions.entry(d).or_default().push(changed.clone()),
            None => unmatched.push(changed.clone()),
        }
    }

    if ctx.json {
        let domain_suggestions: Vec<serde_json::Value> = suggestions
            .iter()
            .map(|(domain, files)| {
                serde_json::json!({
                    "domain": domain,
                    "files": files,
                })
            })
            .collect();
        output_json(&serde_json::json!({
            "success": true,
            "command": "learn",
            "changed_files": changed_files.len(),
            "suggestions": domain_suggestions,
            "unmatched": unmatched,
        }));
    } else {
        println!(
            "Changed files since {}: {}",
            args.since,
            changed_files.len()
        );
        println!();

        if !suggestions.is_empty() {
            println!("Suggested domains to record in:");
            for (domain, files) in &suggestions {
                println!("  {} ({} file(s)):", domain, files.len());
                for f in files.iter().take(5) {
                    println!("    {}", f);
                }
                if files.len() > 5 {
                    println!("    ... and {} more", files.len() - 5);
                }
            }
            println!();
        }

        if !unmatched.is_empty() {
            print_warning(&format!(
                "{} file(s) did not match any domain:",
                unmatched.len()
            ));
            for f in unmatched.iter().take(10) {
                println!("    {}", f);
            }
            if unmatched.len() > 10 {
                println!("    ... and {} more", unmatched.len() - 10);
            }
            println!();
        }

        println!("Record learnings with:");
        println!("  kb record <domain> --type <type> --description \"...\"");
    }

    Ok(())
}
