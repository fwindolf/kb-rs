use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "kb",
    about = "Let your agents grow",
    version,
    propagate_version = true
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Args, Debug, Clone)]
pub struct GlobalArgs {
    /// Output as structured JSON.
    #[arg(long, global = true)]
    pub json: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a .kb directory
    Init,

    /// Add a new expertise domain
    Add(AddArgs),

    /// Remove an expertise domain
    Remove(RemoveArgs),

    /// Record an expertise record (preferences and rationale, not code-discoverable details)
    Record(RecordArgs),

    /// Edit an existing record
    Edit(EditArgs),

    /// Query expertise records
    Query(QueryArgs),

    /// Search records across domains (BM25)
    Search(SearchArgs),

    /// Delete a record by ID
    Delete(DeleteArgs),

    /// Output a priming prompt from expertise
    Prime(PrimeArgs),

    /// Show domain statistics
    Status,

    /// Validate all records against schema
    Validate,

    /// Remove stale records
    Prune(PruneArgs),

    /// Run diagnostic checks
    Doctor(DoctorArgs),

    /// Show recently added/updated records
    Ready(ReadyArgs),

    /// Show changed files and suggest domains
    Learn(LearnArgs),

    /// Merge/consolidate record groups
    Compact(CompactArgs),

    /// Configure IDE provider recipes and git hooks
    Setup(SetupArgs),

    /// Write onboarding content to agent instruction file
    Onboard(OnboardArgs),

    /// Validate, stage, and commit .kb/ to git
    #[command(name = "sync")]
    Sync(SyncArgs),

    /// Check for updates
    Update(UpdateArgs),

    /// Show expertise changes since a git ref
    Diff(DiffArgs),

    /// Query the access log
    #[command(name = "access-log")]
    AccessLog(AccessLogArgs),

    /// Manage sessions
    #[command(subcommand)]
    Session(SessionCommands),

    /// Check file references in records
    Check(CheckArgs),

    /// Run the MCP server (stdio transport)
    Mcp,
}

// ── Argument structs ───────────────────────────────────────────────────────

#[derive(Args, Debug)]
pub struct AddArgs {
    /// Domain name to add
    pub domain: String,
}

#[derive(Args, Debug)]
pub struct RemoveArgs {
    /// Domain name to remove
    pub domain: String,

    /// Force removal even if domain has records
    #[arg(long)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct RecordArgs {
    /// Expertise domain
    pub domain: String,

    /// Record content (positional)
    pub content: Option<String>,

    /// Record type
    #[arg(long = "type", value_parser = ["convention", "pattern", "failure", "decision", "reference", "guide"])]
    pub record_type: Option<String>,

    /// Classification level
    #[arg(long, default_value = "tactical", value_parser = ["foundational", "tactical", "observational"])]
    pub classification: String,

    /// Name (for pattern, reference, guide)
    #[arg(long)]
    pub name: Option<String>,

    /// Why this approach is preferred, not how it works in code
    #[arg(long)]
    pub description: Option<String>,

    /// Resolution (for failure records)
    #[arg(long)]
    pub resolution: Option<String>,

    /// Title (for decision records)
    #[arg(long)]
    pub title: Option<String>,

    /// Rationale (for decision records)
    #[arg(long)]
    pub rationale: Option<String>,

    /// Related files (comma-separated)
    #[arg(long)]
    pub files: Option<String>,

    /// Comma-separated tags
    #[arg(long)]
    pub tags: Option<String>,

    /// Evidence: commit hash
    #[arg(long = "evidence-commit")]
    pub evidence_commit: Option<String>,

    /// Evidence: issue reference
    #[arg(long = "evidence-issue")]
    pub evidence_issue: Option<String>,

    /// Evidence: file path
    #[arg(long = "evidence-file")]
    pub evidence_file: Option<String>,

    /// Evidence: bead ID
    #[arg(long = "evidence-bead")]
    pub evidence_bead: Option<String>,

    /// Comma-separated record IDs this relates to
    #[arg(long = "relates-to")]
    pub relates_to: Option<String>,

    /// Comma-separated record IDs this supersedes
    #[arg(long)]
    pub supersedes: Option<String>,

    /// Outcome status
    #[arg(long = "outcome-status", value_parser = ["success", "failure", "partial"])]
    pub outcome_status: Option<String>,

    /// Outcome duration in milliseconds
    #[arg(long = "outcome-duration")]
    pub outcome_duration: Option<f64>,

    /// Outcome test results summary
    #[arg(long = "outcome-test-results")]
    pub outcome_test_results: Option<String>,

    /// Outcome agent name
    #[arg(long = "outcome-agent")]
    pub outcome_agent: Option<String>,

    /// Force recording even if duplicate exists
    #[arg(long)]
    pub force: bool,

    /// Read JSON record(s) from stdin
    #[arg(long)]
    pub stdin: bool,

    /// Read JSON record(s) from file
    #[arg(long)]
    pub batch: Option<String>,

    /// Preview what would be recorded without writing
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct EditArgs {
    /// Expertise domain
    pub domain: String,

    /// Record ID (full, bare hash, or prefix)
    pub id: String,

    /// New classification
    #[arg(long, value_parser = ["foundational", "tactical", "observational"])]
    pub classification: Option<String>,

    /// New content (convention)
    #[arg(long)]
    pub content: Option<String>,

    /// New name (pattern, reference, guide)
    #[arg(long)]
    pub name: Option<String>,

    /// New description
    #[arg(long)]
    pub description: Option<String>,

    /// New resolution (failure)
    #[arg(long)]
    pub resolution: Option<String>,

    /// New title (decision)
    #[arg(long)]
    pub title: Option<String>,

    /// New rationale (decision)
    #[arg(long)]
    pub rationale: Option<String>,

    /// New files (comma-separated)
    #[arg(long)]
    pub files: Option<String>,

    /// Comma-separated record IDs this relates to
    #[arg(long = "relates-to")]
    pub relates_to: Option<String>,

    /// Comma-separated record IDs this supersedes
    #[arg(long)]
    pub supersedes: Option<String>,

    /// Outcome status
    #[arg(long = "outcome-status", value_parser = ["success", "failure", "partial"])]
    pub outcome_status: Option<String>,

    /// Outcome duration in milliseconds
    #[arg(long = "outcome-duration")]
    pub outcome_duration: Option<f64>,

    /// Outcome test results summary
    #[arg(long = "outcome-test-results")]
    pub outcome_test_results: Option<String>,

    /// Outcome agent name
    #[arg(long = "outcome-agent")]
    pub outcome_agent: Option<String>,
}

#[derive(Args, Debug)]
pub struct QueryArgs {
    /// Domain to query (omit for all)
    pub domain: Option<String>,

    /// Filter by record type
    #[arg(long = "type", value_parser = ["convention", "pattern", "failure", "decision", "reference", "guide"])]
    pub record_type: Option<String>,

    /// Filter by classification
    #[arg(long, value_parser = ["foundational", "tactical", "observational"])]
    pub classification: Option<String>,

    /// Filter by file path (substring match)
    #[arg(long)]
    pub file: Option<String>,

    /// Filter by outcome status
    #[arg(long = "outcome-status", value_parser = ["success", "failure", "partial"])]
    pub outcome_status: Option<String>,

    /// Sort by confirmation score
    #[arg(long)]
    pub sort_by_score: bool,

    /// Query all domains
    #[arg(long)]
    pub all: bool,
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Search query
    pub query: Option<String>,

    /// Limit to specific domain
    #[arg(long)]
    pub domain: Option<String>,

    /// Filter by record type
    #[arg(long = "type", value_parser = ["convention", "pattern", "failure", "decision", "reference", "guide"])]
    pub record_type: Option<String>,

    /// Filter by tag
    #[arg(long)]
    pub tag: Option<String>,

    /// Filter by classification
    #[arg(long, value_parser = ["foundational", "tactical", "observational"])]
    pub classification: Option<String>,

    /// Filter by file path
    #[arg(long)]
    pub file: Option<String>,

    /// Filter by outcome status
    #[arg(long = "outcome-status", value_parser = ["success", "failure", "partial"])]
    pub outcome_status: Option<String>,

    /// Sort by confirmation score
    #[arg(long)]
    pub sort_by_score: bool,
}

#[derive(Args, Debug)]
pub struct DeleteArgs {
    /// Expertise domain
    pub domain: String,

    /// Record ID to delete
    pub id: String,
}

#[derive(Args, Debug)]
pub struct PrimeArgs {
    /// Domains to include (positional, optional)
    pub domains: Vec<String>,

    /// Include full metadata
    #[arg(long)]
    pub full: bool,

    /// Verbose output (alias for --full)
    #[arg(long)]
    pub verbose: bool,

    /// Output for MCP server (JSON)
    #[arg(long)]
    pub mcp: bool,

    /// Output format
    #[arg(long, default_value = "markdown", value_parser = ["markdown", "xml", "plain"])]
    pub format: String,

    /// Filter by git-changed files
    #[arg(long)]
    pub context: bool,

    /// Filter by specific file paths (comma-separated)
    #[arg(long)]
    pub files: Option<String>,

    /// Export to file
    #[arg(long)]
    pub export: Option<String>,

    /// Token budget
    #[arg(long)]
    pub budget: Option<usize>,

    /// Remove token budget limit
    #[arg(long)]
    pub no_limit: bool,

    /// Limit to specific domain
    #[arg(long)]
    pub domain: Option<String>,

    /// Exclude specific domain
    #[arg(long = "exclude-domain")]
    pub exclude_domain: Option<String>,
}

#[derive(Args, Debug)]
pub struct PruneArgs {
    /// Preview what would be pruned without deleting
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct DoctorArgs {
    /// Attempt to fix issues
    #[arg(long)]
    pub fix: bool,
}

#[derive(Args, Debug)]
pub struct ReadyArgs {
    /// Maximum number of records to show
    #[arg(long, default_value = "10")]
    pub limit: usize,

    /// Filter by domain
    #[arg(long)]
    pub domain: Option<String>,

    /// Show records since duration (e.g., 24h, 7d, 2w)
    #[arg(long)]
    pub since: Option<String>,
}

#[derive(Args, Debug)]
pub struct LearnArgs {
    /// Git ref to compare against
    #[arg(long, default_value = "HEAD")]
    pub since: String,
}

#[derive(Args, Debug)]
pub struct CompactArgs {
    /// Auto-merge without prompting
    #[arg(long)]
    pub auto: bool,

    /// Preview what would be compacted
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct SetupArgs {
    /// Install git hook only
    #[arg(long)]
    pub git_hook: bool,

    /// Provider to configure
    #[arg(long, value_parser = ["claude", "cursor", "codex", "gemini", "windsurf", "aider"])]
    pub provider: Option<String>,
}

#[derive(Args, Debug)]
pub struct OnboardArgs {
    /// Auto-discover target file (default).
    #[arg(long, group = "target")]
    pub auto: bool,

    /// Write to AGENTS.md.
    #[arg(long, group = "target")]
    pub agents: bool,

    /// Write to CLAUDE.md.
    #[arg(long, group = "target")]
    pub claude: bool,

    /// Write to .github/copilot-instructions.md.
    #[arg(long, group = "target")]
    pub copilot: bool,

    /// Write to CODEX.md.
    #[arg(long, group = "target")]
    pub codex: bool,

    /// Write to .opencode/instructions.md.
    #[arg(long, group = "target")]
    pub opencode: bool,

    /// Update existing section instead of creating new.
    #[arg(long)]
    pub update: bool,

    /// Check if onboard section is installed.
    #[arg(long, conflicts_with = "remove")]
    pub check: bool,

    /// Remove the onboard section instead of writing it.
    #[arg(long, conflicts_with = "check")]
    pub remove: bool,
}

#[derive(Args, Debug)]
pub struct SyncArgs {
    /// Custom commit message
    #[arg(long)]
    pub message: Option<String>,

    /// Skip validation before committing
    #[arg(long)]
    pub no_validate: bool,
}

#[derive(Args, Debug)]
pub struct UpdateArgs {
    /// Check for updates without installing
    #[arg(long)]
    pub check: bool,
}

#[derive(Args, Debug)]
pub struct DiffArgs {
    /// Git ref to compare against
    #[arg(long, default_value = "HEAD~1")]
    pub since: String,
}

#[derive(Args, Debug)]
pub struct AccessLogArgs {
    /// Filter by session ID
    #[arg(long)]
    pub session: Option<String>,

    /// Filter by domain
    #[arg(long)]
    pub domain: Option<String>,

    /// Filter by tool type (e.g., "query", "search", "oracle")
    #[arg(long = "type")]
    pub tool_type: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum SessionCommands {
    /// List all sessions
    List,

    /// Show session detail and access log
    Show(SessionShowArgs),
}

#[derive(Args, Debug)]
pub struct SessionShowArgs {
    /// Session ID
    pub id: String,
}

#[derive(Args, Debug)]
pub struct CheckArgs {
    /// Limit check to a specific domain
    #[arg(long)]
    pub domain: Option<String>,
}
