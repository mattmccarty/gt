//! CLI argument definitions using clap
//!
//! This module defines all CLI arguments, subcommands, and options.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

/// gt - Cross-platform Git tool
#[derive(Parser, Debug)]
#[command(name = "gt")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress non-error output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Use alternate config file
    #[arg(short, long, global = true, env = "GT_CONFIG")]
    pub config: Option<PathBuf>,

    /// Output format
    #[arg(short, long, global = true, default_value = "terminal")]
    pub output: OutputFormat,

    /// Disable colored output
    #[arg(long, global = true, env = "NO_COLOR")]
    pub no_color: bool,

    /// Show what would be done without making changes
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Force overwrite of existing files/configurations
    #[arg(long, global = true)]
    pub force: bool,

    /// Auto-pick random date for commits (within 2 hours after HEAD)
    #[arg(long, global = true)]
    pub auto: bool,

    /// Show all commits, not just unpushed (local) commits
    #[arg(short, long, global = true)]
    pub all: bool,

    #[command(subcommand)]
    pub command: Commands,
}

/// Output format for command results
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum OutputFormat {
    /// Human-readable terminal output
    #[default]
    Terminal,
    /// JSON output
    Json,
    /// CSV output
    Csv,
}

/// Available top-level subcommands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Get or set configuration values
    Config(ConfigOpts),

    /// Clone repository with automatic identity detection
    Clone(CloneOpts),

    /// Git commit with enhanced date support (passthrough to git commit)
    Commit(CommitOpts),

    /// Git status (passthrough to git status)
    Status(GitStatusOpts),

    /// Git push with scheduled push support
    Push(PushOpts),

    /// Reset commits, staged files, or passthrough to git reset
    Reset(ResetOpts),

    /// Add files to staging (passthrough to git add)
    Add(GitPassthroughOpts),

    /// Pull from remote (passthrough to git pull)
    Pull(GitPassthroughOpts),

    /// Fetch from remote (passthrough to git fetch)
    Fetch(GitPassthroughOpts),

    /// Checkout branches/files (passthrough to git checkout)
    Checkout(GitPassthroughOpts),

    /// Branch operations (passthrough to git branch)
    Branch(GitPassthroughOpts),

    /// Merge branches (passthrough to git merge)
    Merge(GitPassthroughOpts),

    /// Rebase branches (passthrough to git rebase)
    Rebase(GitPassthroughOpts),

    /// Show changes (passthrough to git diff)
    Diff(GitPassthroughOpts),

    /// Show commit logs (passthrough to git log)
    Log(GitPassthroughOpts),

    /// Stash changes (passthrough to git stash)
    Stash(GitPassthroughOpts),

    /// Tag commits (passthrough to git tag)
    Tag(GitPassthroughOpts),

    /// Remote repository management (passthrough to git remote)
    Remote(GitPassthroughOpts),

    // Future: Find/search commands
    // Future: AI commands
    // Future: Statistics commands
}

/// Options for `gt config id add`
#[derive(Parser, Debug)]
pub struct AddOpts {
    /// Identity name
    pub name: String,

    /// Email for this identity (Git user.email)
    #[arg(short, long)]
    pub email: Option<String>,

    /// Git user name for this identity (Git user.name)
    #[arg(short = 'u', long = "user")]
    pub user_name: Option<String>,

    /// Provider
    #[arg(short, long, default_value = "github")]
    pub provider: String,

    /// Strategy override
    #[arg(short, long)]
    pub strategy: Option<StrategyArg>,

    /// Use existing SSH key
    #[arg(short, long)]
    pub key: Option<PathBuf>,

    /// SSH key type
    #[arg(long, default_value = "ed25519")]
    pub key_type: KeyTypeArg,

    /// Don't generate or associate SSH key
    #[arg(long)]
    pub no_key: bool,

    /// Custom hostname for self-hosted providers
    #[arg(long)]
    pub host: Option<String>,

    /// Scope for URL rewriting (organization or user name, e.g., 'mycompany')
    #[arg(long)]
    pub scope: Option<String>,

    /// Directory pattern for conditional strategy (e.g., ~/repos/llc)
    /// Creates an includeIf rule so all repositories under this directory use this identity
    #[arg(short = 'd', long)]
    pub directory: Option<String>,
}

/// Options for `gt config id import`
#[derive(Parser, Debug)]
pub struct ImportOpts {
    /// Identity name to import
    pub name: String,

    /// Email for this identity (Git user.email)
    #[arg(short, long)]
    pub email: Option<String>,

    /// Git user name for this identity (Git user.name)
    #[arg(short = 'u', long = "user")]
    pub user_name: Option<String>,

    /// Provider (auto-detected if not specified)
    #[arg(short, long)]
    pub provider: Option<String>,

    /// Strategy (auto-detected if not specified)
    #[arg(short, long)]
    pub strategy: Option<StrategyArg>,
}

/// Options for `gt config id list`
#[derive(Parser, Debug)]
pub struct ListOpts {
    /// Show all identities (including SSH-only/unmanaged)
    #[arg(short, long)]
    pub all: bool,

    /// Validate SSH config and show warnings
    #[arg(long)]
    pub validate: bool,

    /// Show detailed information (sources, strategies by type)
    #[arg(short = 'd', long = "details")]
    pub details: bool,

    /// Include SSH key paths
    #[arg(long)]
    pub show_keys: bool,
}

/// Options for `gt config id use`
#[derive(Parser, Debug)]
pub struct UseOpts {
    /// Identity to use
    pub identity: String,

    /// Repository path (defaults to current directory)
    #[arg(short, long)]
    pub repo: Option<PathBuf>,

    /// Directory pattern for conditional strategy (e.g., ~/work/)
    /// When specified with conditional strategy, creates an includeIf rule
    /// so all repositories under this directory use this identity
    #[arg(short, long)]
    pub directory: Option<String>,

    /// Use global configuration instead of repository-local
    /// For conditional strategy, this sets up the directory mapping
    #[arg(short, long)]
    pub global: bool,
}

/// Options for `gt clone`
#[derive(Parser, Debug)]
pub struct CloneOpts {
    /// Repository URL to clone
    pub url: String,

    /// Local path for clone
    pub path: Option<PathBuf>,

    /// Identity to use (overrides auto-detection)
    #[arg(long)]
    pub id: Option<String>,

    /// Override strategy
    #[arg(short, long)]
    pub strategy: Option<StrategyArg>,

    /// Clone with original URL (no transformation)
    #[arg(long)]
    pub no_transform: bool,
}

/// Options for `gt config`
#[derive(Parser, Debug)]
pub struct ConfigOpts {
    #[command(subcommand)]
    pub command: Option<ConfigCommands>,
}

/// Config subcommands
#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// List all configuration
    List,

    /// Open config in editor
    Edit,

    /// Validate configuration
    Validate,

    /// Identity configuration
    Id(ConfigIdOpts),
}

/// Options for `gt config id`
#[derive(Parser, Debug)]
pub struct ConfigIdOpts {
    #[command(subcommand)]
    pub command: Option<ConfigIdCommands>,
}

/// Identity config subcommands
#[derive(Subcommand, Debug)]
pub enum ConfigIdCommands {
    /// Add a new identity
    Add(AddOpts),

    /// Import an existing unmanaged identity
    Import(ImportOpts),

    /// List all configured identities
    List(ListOpts),

    /// Use an identity for the current repository
    Use(UseOpts),

    /// Migrate between identity strategies
    Migrate(MigrateOpts),

    /// SSH key management
    Key(KeyOpts),

    /// Show current identity status
    Status(StatusOpts),

    /// Delete an identity and its SSH key
    Delete(DeleteOpts),

    /// Update an existing identity
    Update(UpdateOpts),

    /// Fix repository URLs and configurations
    Fix(FixIdOpts),

    /// Get or set default identity
    Default {
        /// Identity name to set as default
        name: Option<String>,
    },
}

/// Options for `gt config id migrate`
#[derive(Parser, Debug)]
pub struct MigrateOpts {
    /// Identity name to migrate (interactive selection if not provided)
    pub identity: Option<String>,

    /// Target strategy (only for strategy migration)
    #[arg(short, long)]
    pub target: Option<StrategyArg>,

    /// Migrate all legacy identities
    #[arg(long)]
    pub all: bool,

    /// Also update repository URLs
    #[arg(long)]
    pub repos: bool,

    /// Skip confirmation prompts
    #[arg(short, long)]
    pub yes: bool,
}

/// Options for `gt config id fix`
#[derive(Parser, Debug)]
pub struct FixIdOpts {
    /// Repository or directory path
    pub path: Option<PathBuf>,

    /// Fix using specific identity
    #[arg(long)]
    pub id: Option<String>,

    /// Restore original URLs
    #[arg(long)]
    pub restore: bool,

    /// Update to current identity format
    #[arg(long)]
    pub update: bool,

    /// Fix all repos in directory tree
    #[arg(long)]
    pub recursive: bool,
}

/// Options for `gt commit`
///
/// This command supports both:
/// - Passthrough to `git commit` with enhanced date syntax
/// - Subcommands like `list` to show commits with schedule info
#[derive(Parser, Debug)]
#[command(disable_help_flag = true)]
pub struct CommitOpts {
    #[command(subcommand)]
    pub command: Option<CommitCommands>,

    /// Date for the commit (supports shorthand: -30s, 15m, -1h, 2d, -1w, now)
    #[arg(long, global = true)]
    pub date: Option<String>,

    /// Force overwrite of existing files/configurations (use current time for commits)
    #[arg(long, global = true)]
    pub force: bool,

    /// Auto-pick random date for commits (within 2 hours after HEAD)
    #[arg(long, global = true)]
    pub auto: bool,

    /// Display help information
    #[arg(long, short = 'h', global = true)]
    pub help: bool,

    /// Show all commits, not just unpushed (local) commits
    #[arg(short, long, global = true)]
    pub all: bool,

    /// All other git commit arguments (passed through when no subcommand)
    #[arg(allow_hyphen_values = true, num_args = 0.., trailing_var_arg = true)]
    pub git_args: Vec<String>,
}

/// Commit subcommands
#[derive(Subcommand, Debug)]
pub enum CommitCommands {
    /// List commits with scheduled push information
    List(CommitListOpts),
}

/// Options for `gt commit list`
#[derive(Parser, Debug)]
pub struct CommitListOpts {
    /// Number of commits to show (default: 10, 0 for unlimited with pagination)
    #[arg(short, long, default_value = "10")]
    pub limit: usize,
}

/// Options for `gt status`
///
/// This is a passthrough command that forwards all arguments to `git status`.
#[derive(Parser, Debug)]
#[command(trailing_var_arg = true)]
#[command(disable_help_flag = true)]
pub struct GitStatusOpts {
    /// Display help information
    #[arg(long, short = 'h')]
    pub help: bool,

    /// All git status arguments (passed through)
    #[arg(allow_hyphen_values = true, num_args = 0..)]
    pub git_args: Vec<String>,
}

/// Options for `gt push`
///
/// This command supports scheduled pushes for repositories with future-dated commits.
/// When a push would include commits with future dates, it creates a schedule and
/// installs a pre-push hook to block manual pushes until the scheduled time.
#[derive(Parser, Debug)]
#[command(trailing_var_arg = true)]
pub struct PushOpts {
    /// Remote name (defaults to "origin")
    pub remote: Option<String>,

    /// Branch name (defaults to current branch)
    pub branch: Option<String>,

    /// Push immediately, ignoring any schedule
    #[arg(short, long)]
    pub force: bool,

    /// List all scheduled pushes
    #[arg(short, long)]
    pub list: bool,

    /// Cancel scheduled push for current or specified branch
    #[arg(long)]
    pub cancel: bool,

    /// Internal: called by pre-push hook to check if push should be blocked
    #[arg(long, hide = true)]
    pub hook_check: bool,

    /// Additional git push arguments (passed through)
    #[arg(allow_hyphen_values = true, num_args = 0..)]
    pub git_args: Vec<String>,
}

/// Options for `gt reset`
///
/// This command supports:
/// - `gt reset commits` - Reset to initial commit, keep changes, clear schedule & history
/// - `gt reset staged` - Unstage all files (git reset HEAD)
/// - `gt reset <args>` - Passthrough to git reset
#[derive(Parser, Debug)]
#[command(trailing_var_arg = true)]
pub struct ResetOpts {
    /// Don't clear reflog and gc when resetting commits
    #[arg(long)]
    pub keep_history: bool,

    /// All git reset arguments (or special: "commits", "staged")
    #[arg(allow_hyphen_values = true, num_args = 0..)]
    pub args: Vec<String>,
}

/// Generic git command passthrough options
#[derive(Parser, Debug)]
#[command(trailing_var_arg = true)]
#[command(disable_help_flag = true)]
pub struct GitPassthroughOpts {
    /// Display help information
    #[arg(long, short = 'h')]
    pub help: bool,

    /// All arguments to pass through to git command
    #[arg(allow_hyphen_values = true, num_args = 0..)]
    pub args: Vec<String>,
}

/// Options for `gt config id key`
#[derive(Parser, Debug)]
pub struct KeyOpts {
    #[command(subcommand)]
    pub command: KeyCommands,
}

/// Key management subcommands
#[derive(Subcommand, Debug)]
pub enum KeyCommands {
    /// Generate a new SSH key
    Generate {
        /// Identity name
        identity: String,

        /// Key type
        #[arg(short = 't', long, default_value = "ed25519")]
        key_type: KeyTypeArg,

        /// RSA key bits
        #[arg(short, long, default_value = "4096")]
        bits: u32,

        /// Email address (sets SSH key comment field)
        #[arg(short, long)]
        email: Option<String>,

        /// Passphrase to encrypt the private key
        #[arg(short, long)]
        passphrase: Option<String>,

        /// Overwrite existing key
        #[arg(long)]
        force: bool,
    },

    /// List SSH keys
    List {
        /// Show all SSH keys
        #[arg(short, long)]
        all: bool,

        /// Filter by identity
        #[arg(long)]
        identity: Option<String>,
    },

    /// Add existing key to identity
    Add {
        /// Identity name
        identity: String,

        /// Path to SSH key
        key_path: PathBuf,
    },

    /// Remove key from identity
    Remove {
        /// Identity name
        identity: String,
    },

    /// Add key to SSH agent
    Activate {
        /// Identity name
        identity: String,
    },

    /// Show public key
    Show {
        /// Identity name
        identity: String,
    },

    /// Test key authentication
    Test {
        /// Identity name
        identity: String,
    },
}

/// Options for `gt config id status`
#[derive(Parser, Debug)]
pub struct StatusOpts {
    /// Repository path
    #[arg(short, long)]
    pub repo: Option<PathBuf>,

    /// Show detailed status
    #[arg(short, long)]
    pub all: bool,
}

/// Options for `gt config id delete`
#[derive(Parser, Debug)]
pub struct DeleteOpts {
    /// Identity name to delete
    pub identity: String,

    /// Only delete this specific strategy type
    #[arg(short, long)]
    pub strategy: Option<StrategyArg>,

    /// Scope for URL strategy (to delete specific scope variant)
    #[arg(long)]
    pub scope: Option<String>,

    /// Directory for conditional strategy (to delete specific directory variant)
    #[arg(short = 'd', long)]
    pub directory: Option<String>,

    /// Delete SSH key without confirmation
    #[arg(long)]
    pub delete_key: bool,

    /// Keep SSH key (only delete SSH config entry)
    #[arg(long)]
    pub keep_key: bool,
}

/// Options for `gt config id update`
#[derive(Parser, Debug)]
pub struct UpdateOpts {
    /// Identity name to update
    pub identity: String,

    /// Rename the identity to a new name
    #[arg(short, long)]
    pub name: Option<String>,

    /// New email address
    #[arg(short, long)]
    pub email: Option<String>,

    /// New Git user name
    #[arg(short = 'u', long = "user")]
    pub user: Option<String>,

    /// Change strategy
    #[arg(short, long)]
    pub strategy: Option<StrategyArg>,

    /// Scope for URL rewriting (organization or user name)
    #[arg(long)]
    pub scope: Option<String>,

    /// Directory pattern for conditional strategy (e.g., ~/repos/llc)
    #[arg(short = 'd', long)]
    pub directory: Option<String>,
}

/// Strategy type argument
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum StrategyArg {
    /// SSH hostname alias strategy
    Ssh,
    /// Git conditional includes strategy (directory-based)
    Conditional,
    /// URL rewriting strategy
    Url,
}

/// SSH key type argument
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum KeyTypeArg {
    /// Ed25519 (recommended)
    #[default]
    Ed25519,
    /// RSA
    Rsa,
    /// ECDSA
    Ecdsa,
}

impl std::fmt::Display for StrategyArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StrategyArg::Ssh => write!(f, "ssh"),
            StrategyArg::Conditional => write!(f, "conditional"),
            StrategyArg::Url => write!(f, "url"),
        }
    }
}

impl std::fmt::Display for KeyTypeArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyTypeArg::Ed25519 => write!(f, "ed25519"),
            KeyTypeArg::Rsa => write!(f, "rsa"),
            KeyTypeArg::Ecdsa => write!(f, "ecdsa"),
        }
    }
}
