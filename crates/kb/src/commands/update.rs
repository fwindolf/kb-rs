use anyhow::Result;

use crate::cli::UpdateArgs;
use crate::context::RuntimeContext;
use crate::output::*;

pub fn run(ctx: &RuntimeContext, args: &UpdateArgs) -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");

    if ctx.json {
        output_json(&serde_json::json!({
            "success": true,
            "command": "update",
            "current_version": current_version,
            "check": args.check,
            "instructions": "cargo install --force kb",
        }));
    } else if args.check {
        println!("Current version: {current_version}");
        println!("To update, run:");
        println!("  cargo install --force kb");
    } else {
        println!("Current version: {current_version}");
        println!();
        println!("To update kb, run:");
        println!("  cargo install --force kb");
        println!();
        println!("Or if installed from source:");
        println!("  cargo install --path .");
    }

    Ok(())
}
