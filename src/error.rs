//! Error types for gitid
//!
//! This module defines all error types used throughout the application,
//! providing clear error messages and recovery suggestions.

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias using gitid's Error type
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for gitid operations
#[derive(Debug, Error)]
pub enum Error {
    // ==================== Configuration Errors ====================
    /// Configuration file not found
    #[error("Configuration not found at {path}. Run 'gitid init' to create it.")]
    ConfigNotFound {
        /// Path where config was expected
        path: PathBuf,
    },

    /// Configuration file is invalid
    #[error("Invalid configuration: {message}")]
    ConfigInvalid {
        /// Description of what's invalid
        message: String,
    },

    /// Configuration parse error
    #[error("Failed to parse configuration: {0}")]
    ConfigParse(#[from] toml::de::Error),

    /// Configuration serialization error
    #[error("Failed to serialize configuration: {0}")]
    ConfigSerialize(#[from] toml::ser::Error),

    // ==================== Identity Errors ====================
    /// Identity not found
    #[error("Identity '{name}' not found. Run 'gitid list' to see available identities.")]
    IdentityNotFound {
        /// Name of the identity that wasn't found
        name: String,
    },

    /// Identity already exists
    #[error("Identity '{name}' already exists. Use 'gitid config' to modify it.")]
    IdentityExists {
        /// Name of the existing identity
        name: String,
    },

    /// Invalid identity name
    #[error("Invalid identity name '{name}': {reason}")]
    IdentityNameInvalid {
        /// The invalid name
        name: String,
        /// Why it's invalid
        reason: String,
    },

    /// Identity validation failed
    #[error("Identity validation failed: {message}")]
    IdentityValidation {
        /// Validation error message
        message: String,
    },

    // ==================== Repository Errors ====================
    /// Not inside a Git repository
    #[error("Not inside a Git repository. Run from a Git repository or use --repo.")]
    NotARepository,

    /// Repository has no remote configured
    #[error("Repository has no remote '{remote}' configured.")]
    NoRemote {
        /// Name of the missing remote
        remote: String,
    },

    /// Repository path not found
    #[error("Repository path not found: {path}")]
    RepoNotFound {
        /// Path that wasn't found
        path: PathBuf,
    },

    // ==================== URL Errors ====================
    /// Unrecognized URL format
    #[error("Unrecognized URL format: {url}")]
    UrlUnrecognized {
        /// The unrecognized URL
        url: String,
    },

    /// Unknown provider
    #[error("Unknown provider: {hostname}. Add it with 'gitid config providers.{hostname}.hostname {hostname}'")]
    ProviderUnknown {
        /// The unknown hostname
        hostname: String,
    },

    /// URL transformation failed
    #[error("Failed to transform URL: {message}")]
    UrlTransform {
        /// Error message
        message: String,
    },

    // ==================== SSH Errors ====================
    /// SSH key not found
    #[error("SSH key not found: {path}. Generate with 'gitid key generate {identity}'.")]
    SshKeyNotFound {
        /// Path where key was expected
        path: PathBuf,
        /// Associated identity
        identity: String,
    },

    /// SSH key generation failed
    #[error("Failed to generate SSH key: {message}")]
    SshKeyGeneration {
        /// Error message
        message: String,
    },

    /// SSH config parse error
    #[error("Failed to parse SSH config: {message}")]
    SshConfigParse {
        /// Error message
        message: String,
    },

    /// SSH agent error
    #[error("SSH agent error: {message}")]
    SshAgent {
        /// Error message
        message: String,
    },

    /// SSH authentication test failed
    #[error("SSH authentication failed for {identity} on {provider}: {message}")]
    SshAuthFailed {
        /// Identity that failed
        identity: String,
        /// Provider that was tested
        provider: String,
        /// Error message
        message: String,
    },

    // ==================== Git Errors ====================
    /// Git command failed
    #[error("Git command failed: {message}")]
    GitCommand {
        /// Error message
        message: String,
    },

    /// Git config parse error
    #[error("Failed to parse Git config: {message}")]
    GitConfigParse {
        /// Error message
        message: String,
    },

    // ==================== Strategy Errors ====================
    /// Strategy not supported for provider
    #[error("Strategy '{strategy}' not supported for provider '{provider}'.")]
    StrategyNotSupported {
        /// Strategy name
        strategy: String,
        /// Provider name
        provider: String,
    },

    /// Strategy validation failed
    #[error("Strategy validation failed: {message}")]
    StrategyValidation {
        /// Error message
        message: String,
    },

    /// Migration not possible
    #[error("Cannot migrate from {from} to {to}: {reason}")]
    MigrationNotPossible {
        /// Source strategy
        from: String,
        /// Target strategy
        to: String,
        /// Reason
        reason: String,
    },

    // ==================== File System Errors ====================
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// File permission error
    #[error("Insecure permissions on {path}: mode {mode:o}, required {required:o}")]
    InsecurePermissions {
        /// File path
        path: PathBuf,
        /// Current mode
        mode: u32,
        /// Required mode
        required: u32,
    },

    /// Backup failed
    #[error("Failed to create backup of {path}: {message}")]
    BackupFailed {
        /// File that couldn't be backed up
        path: PathBuf,
        /// Error message
        message: String,
    },

    /// Home directory not found
    #[error("Could not determine home directory. Set HOME environment variable.")]
    HomeNotFound,

    // ==================== User Interaction Errors ====================
    /// User cancelled operation
    #[error("Operation cancelled by user.")]
    Cancelled,

    /// Required input not provided
    #[error("Required input not provided: {field}")]
    InputRequired {
        /// Field that was required
        field: String,
    },

    // ==================== External Tool Errors ====================
    /// Required tool not found
    #[error("Required tool not found: {tool}. Please install it and try again.")]
    ToolNotFound {
        /// Name of the missing tool
        tool: String,
    },

    /// External tool execution failed
    #[error("External tool '{tool}' failed: {message}")]
    ToolFailed {
        /// Tool name
        tool: String,
        /// Error message
        message: String,
    },

    // ==================== Push Schedule Errors ====================
    /// Push is blocked by scheduled push
    #[error("Push blocked: scheduled for {scheduled_time}")]
    PushScheduled {
        /// Scheduled push time
        scheduled_time: String,
    },

    /// Schedule not found
    #[error("No scheduled push found for this branch")]
    ScheduleNotFound,

    /// Schedule is outdated
    #[error("Schedule is outdated: new commits detected after scheduled commit")]
    ScheduleOutdated,
}

impl Error {
    /// Returns the exit code for this error
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        match self {
            // General errors
            Error::Io(_) | Error::HomeNotFound => 1,

            // Configuration errors
            Error::ConfigNotFound { .. }
            | Error::ConfigInvalid { .. }
            | Error::ConfigParse(_)
            | Error::ConfigSerialize(_) => 2,

            // Identity errors
            Error::IdentityNotFound { .. }
            | Error::IdentityExists { .. }
            | Error::IdentityNameInvalid { .. }
            | Error::IdentityValidation { .. } => 3,

            // Repository errors
            Error::NotARepository
            | Error::NoRemote { .. }
            | Error::RepoNotFound { .. } => 4,

            // SSH errors
            Error::SshKeyNotFound { .. }
            | Error::SshKeyGeneration { .. }
            | Error::SshConfigParse { .. }
            | Error::SshAgent { .. }
            | Error::SshAuthFailed { .. } => 5,

            // Git errors
            Error::GitCommand { .. } | Error::GitConfigParse { .. } => 6,

            // URL errors
            Error::UrlUnrecognized { .. }
            | Error::ProviderUnknown { .. }
            | Error::UrlTransform { .. } => 7,

            // Strategy errors
            Error::StrategyNotSupported { .. }
            | Error::StrategyValidation { .. }
            | Error::MigrationNotPossible { .. } => 8,

            // Permission errors
            Error::InsecurePermissions { .. } | Error::BackupFailed { .. } => 9,

            // User cancelled
            Error::Cancelled | Error::InputRequired { .. } => 10,

            // External tools
            Error::ToolNotFound { .. } | Error::ToolFailed { .. } => 11,

            // Push schedule errors
            Error::PushScheduled { .. }
            | Error::ScheduleNotFound
            | Error::ScheduleOutdated => 12,
        }
    }

    /// Returns a suggestion for resolving this error
    #[must_use]
    pub fn suggestion(&self) -> Option<&str> {
        match self {
            Error::ConfigNotFound { .. } => Some("Run 'gitid init' to create configuration"),
            Error::IdentityNotFound { .. } => {
                Some("Run 'gitid list' to see available identities")
            }
            Error::NotARepository => Some("Run from inside a Git repository"),
            Error::SshKeyNotFound { .. } => Some("Run 'gitid key generate <identity>'"),
            Error::ToolNotFound { tool } if tool == "ssh-keygen" => {
                Some("Install OpenSSH to enable SSH key generation")
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_codes_are_unique_per_category() {
        // Configuration errors should all be 2
        assert_eq!(
            Error::ConfigNotFound {
                path: PathBuf::new()
            }
            .exit_code(),
            2
        );
        assert_eq!(
            Error::ConfigInvalid {
                message: String::new()
            }
            .exit_code(),
            2
        );

        // Identity errors should all be 3
        assert_eq!(
            Error::IdentityNotFound {
                name: String::new()
            }
            .exit_code(),
            3
        );
    }

    #[test]
    fn test_error_display() {
        let err = Error::IdentityNotFound {
            name: "work".to_string(),
        };
        assert!(err.to_string().contains("work"));
        assert!(err.to_string().contains("gitid list"));
    }
}
