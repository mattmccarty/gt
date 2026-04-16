//! Identity management strategies
//!
//! This module defines the Strategy trait and implementations for:
//! - SSH hostname aliases
//! - Git conditional includes
//! - URL rewriting with insteadOf

pub mod conditional;
pub mod ssh_alias;
pub mod url_rewrite;

use crate::core::identity::Identity;
use crate::core::repo::Repo;
use crate::error::Result;

/// Strategy type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StrategyType {
    /// SSH hostname alias strategy
    SshAlias,
    /// Git conditional includes strategy
    Conditional,
    /// URL rewriting with insteadOf
    UrlRewrite,
}

impl StrategyType {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            // New short names (primary)
            "ssh" => Some(StrategyType::SshAlias),
            "conditional" | "cond" | "dir" => Some(StrategyType::Conditional),
            "url" | "rewrite" => Some(StrategyType::UrlRewrite),
            // Old names (backwards compatibility)
            "ssh-alias" | "sshalias" => Some(StrategyType::SshAlias),
            "include" => Some(StrategyType::Conditional),
            "url-rewrite" | "urlrewrite" | "insteadof" => Some(StrategyType::UrlRewrite),
            _ => None,
        }
    }
}

impl std::fmt::Display for StrategyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StrategyType::SshAlias => write!(f, "ssh"),
            StrategyType::Conditional => write!(f, "conditional"),
            StrategyType::UrlRewrite => write!(f, "url"),
        }
    }
}

/// Result of applying a strategy
#[derive(Debug)]
pub struct ApplyResult {
    /// Changes made
    pub changes: Vec<String>,
    /// Warnings
    pub warnings: Vec<String>,
}

impl ApplyResult {
    /// Create a new empty result
    #[must_use]
    pub fn new() -> Self {
        Self {
            changes: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Add a change
    pub fn add_change(&mut self, change: impl Into<String>) {
        self.changes.push(change.into());
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }
}

impl Default for ApplyResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Validation result for a strategy
#[derive(Debug)]
pub struct ValidationResult {
    /// Whether the strategy is valid
    pub valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Warnings
    pub warnings: Vec<String>,
}

/// A setup step required for a strategy
#[derive(Debug)]
pub struct SetupStep {
    /// Description of the step
    pub description: String,
    /// Whether this step is complete
    pub complete: bool,
}

/// Core trait for identity management strategies
pub trait Strategy: Send + Sync {
    /// Returns the strategy type
    fn strategy_type(&self) -> StrategyType;

    /// Applies the identity to a repository
    fn apply(&self, identity: &Identity, repo: &Repo) -> Result<ApplyResult>;

    /// Removes the identity from a repository
    fn remove(&self, identity: &Identity, repo: &Repo) -> Result<()>;

    /// Checks if this strategy is currently active for the repo
    fn is_active(&self, identity: &Identity, repo: &Repo) -> Result<bool>;

    /// Validates the strategy can be used in current environment
    fn validate(&self) -> Result<ValidationResult>;

    /// Returns required setup steps
    fn setup_requirements(&self) -> Vec<SetupStep>;
}

/// Create a strategy instance
pub fn create_strategy(strategy_type: StrategyType) -> Box<dyn Strategy> {
    match strategy_type {
        StrategyType::SshAlias => Box::new(ssh_alias::SshAliasStrategy::new()),
        StrategyType::Conditional => Box::new(conditional::ConditionalStrategy::new()),
        StrategyType::UrlRewrite => Box::new(url_rewrite::UrlRewriteStrategy::new()),
    }
}
