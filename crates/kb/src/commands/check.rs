use anyhow::Result;

use crate::cli::CheckArgs;
use crate::context::RuntimeContext;
use crate::output::*;

pub fn run(ctx: &RuntimeContext, args: &CheckArgs) -> Result<()> {
    let results = kb_core::check::check_references(&ctx.cwd, args.domain.as_deref())?;

    if ctx.json {
        output_json(&serde_json::json!({
            "success": true,
            "command": "check",
            "broken_count": results.len(),
            "results": results,
        }));
    } else if results.is_empty() {
        print_success("All file references are valid.");
    } else {
        println!("Found {} entries with broken references:\n", results.len());
        for r in &results {
            println!("  [{}/{}] {}", r.domain, r.entry_id, r.entry_summary);
            for path in &r.broken_refs {
                print_warning(&format!("    - {path}"));
            }
            println!();
        }
    }

    Ok(())
}
