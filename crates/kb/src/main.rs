mod cli;
mod commands;
mod context;
mod mcp;
mod output;

use clap::Parser;

use cli::{Cli, Commands};
use context::RuntimeContext;

fn main() {
    let cli = Cli::parse();
    let ctx = RuntimeContext::from_global_args(&cli.global);

    // MCP server needs an async runtime; handle it separately
    if matches!(&cli.command, Commands::Mcp) {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        if let Err(e) = rt.block_on(mcp::run_mcp_server(ctx.cwd.clone())) {
            output::print_error(&format!("MCP server error: {e:#}"));
            std::process::exit(1);
        }
        return;
    }

    let result = match &cli.command {
        Commands::Init => commands::init::run(&ctx),
        Commands::Add(args) => commands::add::run(&ctx, args),
        Commands::Remove(args) => commands::remove::run(&ctx, args),
        Commands::Record(args) => commands::record::run(&ctx, args),
        Commands::Edit(args) => commands::edit::run(&ctx, args),
        Commands::Query(args) => commands::query::run(&ctx, args),
        Commands::Search(args) => commands::search::run(&ctx, args),
        Commands::Delete(args) => commands::delete::run(&ctx, args),
        Commands::Prime(args) => commands::prime::run(&ctx, args),
        Commands::Status => commands::status::run(&ctx),
        Commands::Validate => commands::validate::run(&ctx),
        Commands::Prune(args) => commands::prune::run(&ctx, args),
        Commands::Doctor(args) => commands::doctor::run(&ctx, args),
        Commands::Ready(args) => commands::ready::run(&ctx, args),
        Commands::Learn(args) => commands::learn::run(&ctx, args),
        Commands::Compact(args) => commands::compact::run(&ctx, args),
        Commands::Setup(args) => commands::setup::run(&ctx, args),
        Commands::Onboard(args) => commands::onboard::run(&ctx, args),
        Commands::Sync(args) => commands::sync_cmd::run(&ctx, args),
        Commands::Update(args) => commands::update::run(&ctx, args),
        Commands::Diff(args) => commands::diff::run(&ctx, args),
        Commands::AccessLog(args) => commands::access_log::run(&ctx, args),
        Commands::Session(cmd) => commands::session::run(&ctx, cmd),
        Commands::Check(args) => commands::check::run(&ctx, args),
        Commands::Mcp => unreachable!(),
    };

    if let Err(e) = result {
        if ctx.json {
            output::output_json_error("unknown", &format!("{e:#}"));
        } else {
            output::print_error(&format!("Error: {e:#}"));
        }
        std::process::exit(1);
    }
}
