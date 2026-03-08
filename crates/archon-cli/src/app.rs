use clap::{Parser, Subcommand};
use archon_core::profile::Profile;

#[derive(Parser)]
#[command(
    name = "archon",
    about = "Architecture governance tool for the Hologram ecosystem",
    long_about = "Initializes, verifies, and syncs architecture standards across repositories.\n\n\
        archon is the executor/enforcer for architecture decisions defined in\n\
        hologram-architecture. It manages repo metadata, conformance checking,\n\
        file sync, ADRs, and dependency graphs.",
    version,
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a repo with architecture governance metadata and docs
    Init(InitArgs),

    /// Run conformance checks against architecture standards
    Verify(VerifyArgs),

    /// Show repo conformance status (read-only summary)
    Status(StatusArgs),

    /// Sync managed files from the architecture repo
    Sync(SyncArgs),

    /// Manage Architecture Decision Records
    #[command(subcommand)]
    Adr(AdrCommands),

    /// Manage policy exceptions
    #[command(subcommand)]
    Exception(ExceptionCommands),

    /// Bootstrap a new architecture repository with initial ADRs, policies, and templates
    Bootstrap(BootstrapArgs),

    /// Use AI to draft an architecture decision based on a question
    Decide(DecideArgs),

    /// Manage AI provider authentication (OAuth login, refresh, status, logout)
    #[command(subcommand)]
    Auth(AuthCommands),

    /// Interactively generate architecture documents (specs, prompts, ADRs, plans) using AI
    Generate(GenerateArgs),
}

#[derive(Subcommand)]
pub enum AuthCommands {
    /// Log in to an AI provider via browser OAuth
    Login {
        /// Provider name: claude or openai
        #[arg(long)]
        provider: Option<String>,

        /// Override the OAuth callback port
        #[arg(long)]
        port: Option<u16>,
    },

    /// Refresh an expired OAuth token
    Refresh {
        /// Provider name to refresh
        #[arg(long)]
        provider: Option<String>,
    },

    /// Show stored credential status
    Status,

    /// Remove stored credentials for a provider
    Logout {
        /// Provider name to log out
        #[arg(long)]
        provider: Option<String>,
    },
}

#[derive(clap::Args)]
pub struct InitArgs {
    /// Repository profile: rust-workspace | runtime-system | compiler-ai | cli-tool | service-app
    #[arg(long, value_enum)]
    pub profile: Option<Profile>,

    /// Standards version to use (default: current date-based version)
    #[arg(long, default_value = "2026.03")]
    pub standards_version: String,

    /// Show what would be created; write nothing
    #[arg(long)]
    pub dry_run: bool,

    /// Overwrite files that already exist
    #[arg(long)]
    pub force: bool,

    /// Skip AI-driven TODO filling even if a backend is available
    #[arg(long)]
    pub no_ai: bool,

    /// Path to architecture repo (for AI context loading)
    #[arg(long)]
    pub arch_root: Option<String>,
}

#[derive(clap::Args)]
pub struct VerifyArgs {
    /// Output format
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,

    /// Treat warnings as errors
    #[arg(long)]
    pub strict: bool,

    /// Path to architecture repo (default: auto-detect)
    #[arg(long)]
    pub arch_root: Option<String>,
}

#[derive(clap::Args)]
pub struct StatusArgs {
    /// Output format
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

#[derive(clap::Args)]
pub struct SyncArgs {
    /// Show what would change; write nothing
    #[arg(long)]
    pub dry_run: bool,

    /// Overwrite even if source hasn't changed
    #[arg(long)]
    pub force: bool,

    /// Path to architecture repo (default: auto-detect)
    #[arg(long)]
    pub arch_root: Option<String>,
}

#[derive(Subcommand)]
pub enum AdrCommands {
    /// Create a new Architecture Decision Record
    New {
        /// Title of the ADR
        #[arg(long)]
        title: String,

        /// Initial status (default: proposed)
        #[arg(long)]
        status: Option<String>,
    },

    /// List existing ADRs
    List,
}

#[derive(Subcommand)]
pub enum ExceptionCommands {
    /// Declare a new policy exception
    New {
        /// Policy rule ID to except (e.g., STR-003)
        #[arg(long)]
        rule: String,

        /// Reason for the exception
        #[arg(long)]
        reason: String,

        /// Expiry date (YYYY-MM-DD)
        #[arg(long)]
        expires: Option<String>,
    },

    /// List declared exceptions
    List,
}

#[derive(clap::Args)]
pub struct BootstrapArgs {
    /// Target directory for the architecture repo (default: current directory)
    #[arg(long)]
    pub path: Option<String>,

    /// Standards version to use
    #[arg(long, default_value = "2026.03")]
    pub standards_version: String,

    /// Show what would be created; write nothing
    #[arg(long)]
    pub dry_run: bool,

    /// Overwrite files that already exist
    #[arg(long)]
    pub force: bool,
}

#[derive(clap::Args)]
pub struct DecideArgs {
    /// Title for the architecture decision
    #[arg(long)]
    pub title: String,

    /// The question or problem to address (defaults to title if not provided)
    #[arg(long)]
    pub question: Option<String>,

    /// Path to architecture repo (default: auto-detect)
    #[arg(long)]
    pub arch_root: Option<String>,

    /// Preview the draft without writing files
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(clap::Args)]
pub struct GenerateArgs {
    /// Document type: spec, prompt, adr, plan
    #[arg(long, value_enum)]
    pub doc_type: Option<DocType>,

    /// Title for the document
    #[arg(long)]
    pub title: Option<String>,

    /// Brief description or purpose
    #[arg(long)]
    pub description: Option<String>,

    /// Path to architecture repo (for context loading, especially ADRs)
    #[arg(long)]
    pub arch_root: Option<String>,

    /// Preview the draft without writing files
    #[arg(long)]
    pub dry_run: bool,

    /// Skip the AI refinement loop; generate from structured answers only
    #[arg(long)]
    pub no_refine: bool,
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub enum DocType {
    Spec,
    Prompt,
    Adr,
    Plan,
}

impl DocType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DocType::Spec => "spec",
            DocType::Prompt => "prompt",
            DocType::Adr => "adr",
            DocType::Plan => "plan",
        }
    }
}

impl std::fmt::Display for DocType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}
