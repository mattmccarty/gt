//! Command implementations for gitid
//!
//! This module contains the implementation for each CLI command.
//! Each command is in its own submodule.

use std::path::PathBuf;

use crate::cli::args::{Cli, OutputFormat};
use crate::error::Result;
use crate::io::toml_config::GtConfig;

pub mod add;
pub mod clone;
pub mod commit;
pub mod config;
pub mod delete;
pub mod fix;
pub mod git_status;
pub mod import;
pub mod key;
pub mod list;
pub mod migrate;
pub mod push;
pub mod reset;
pub mod status;
pub mod update;
#[path = "use_.rs"]
pub mod use_;

/// Execution context for commands
///
/// Contains shared state and configuration needed by commands.
pub struct Context {
    /// Loaded gitid configuration
    pub config: Option<GtConfig>,

    /// Path to the configuration file
    pub config_path: PathBuf,

    /// Output format
    pub output_format: OutputFormat,

    /// Verbosity level (0 = normal, 1+ = verbose)
    pub verbosity: u8,

    /// Quiet mode
    pub quiet: bool,

    /// Dry-run mode
    pub dry_run: bool,

    /// Force mode
    pub force: bool,

    /// Auto mode (for commit chronological ordering)
    pub auto: bool,

    /// Show all (e.g., all commits, not just unpushed)
    pub all: bool,

    /// Disable colors
    pub no_color: bool,
}

impl Context {
    /// Create a new context from CLI arguments
    pub fn new(cli: &Cli) -> Result<Self> {
        let config_path = cli
            .config
            .clone()
            .unwrap_or_else(|| crate::util::config_path().unwrap_or_default());

        let config = if config_path.exists() {
            Some(GtConfig::load(&config_path)?)
        } else {
            None
        };

        Ok(Self {
            config,
            config_path,
            output_format: cli.output,
            verbosity: cli.verbose,
            quiet: cli.quiet,
            dry_run: cli.dry_run,
            force: cli.force,
            auto: cli.auto,
            all: cli.all,
            no_color: cli.no_color,
        })
    }

    /// Check if config is loaded
    #[must_use]
    pub fn has_config(&self) -> bool {
        self.config.is_some()
    }

    /// Get config, returning error if not loaded
    pub fn require_config(&self) -> Result<&GtConfig> {
        self.config
            .as_ref()
            .ok_or_else(|| crate::error::Error::ConfigNotFound {
                path: self.config_path.clone(),
            })
    }

    /// Log at debug level if verbose
    pub fn debug(&self, message: &str) {
        if self.verbosity >= 2 {
            eprintln!("[DEBUG] {}", message);
        }
    }

    /// Log at info level if verbose
    pub fn info(&self, message: &str) {
        if self.verbosity >= 1 && !self.quiet {
            eprintln!("[INFO] {}", message);
        }
    }
}
