use anyhow::Result;

use crate::context::RuntimeContext;
use crate::output::*;

pub fn run(ctx: &RuntimeContext) -> Result<()> {
    kb_core::config::init_kb_dir(&ctx.cwd)?;

    if ctx.json {
        output_json(&serde_json::json!({
            "success": true,
            "command": "init",
        }));
    } else {
        print_success("Initialized .kb/ directory.");
    }

    Ok(())
}
