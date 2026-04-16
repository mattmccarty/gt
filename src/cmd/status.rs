//! Implementation of `gt id status` command
//!
//! This command shows the current identity status with multi-level detection:
//!
//! Priority 1: Repository Level (if in a git repo)
//!   - Check local git config user.email
//!   - Match to identity from SSH/URL rewrite strategies
//!   - Show: "Using identity 'X' (repository)"
//!
//! Priority 2: Conditional/Directory Level (works everywhere)
//!   - Check if current directory matches any conditional include patterns
//!   - Parse ~/.gitconfig for [includeIf] sections
//!   - Find which identity's conditional config would apply
//!   - Show: "Using identity 'X' (conditional)"
//!   - This works even if not in a git repository
//!
//! Priority 3: Global Level (fallback)
//!   - Check global git config user.email
//!   - Match to identity
//!   - Show: "Using identity 'X' (global)"
//!
//! Priority 4: No Identity
//!   - Show: "No identity configured"

use crate::cli::args::StatusOpts;
use crate::cli::output::Output;
use crate::cmd::Context;
use crate::core::path;
use crate::error::{Error, Result};
use crate::io::git_config;
use crate::strategy::conditional::ConditionalStrategy;
use std::path::Path;
use std::process::Command;

/// Configuration level for identity detection
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigLevel {
    /// Repository-level configuration (.git/config)
    Repository,
    /// Conditional include configuration (includeIf)
    Conditional,
    /// Global configuration (~/.gitconfig)
    Global,
    /// No configuration found
    None,
}

impl std::fmt::Display for ConfigLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigLevel::Repository => write!(f, "repository"),
            ConfigLevel::Conditional => write!(f, "conditional"),
            ConfigLevel::Global => write!(f, "global"),
            ConfigLevel::None => write!(f, "none"),
        }
    }
}

/// Result of identity detection
#[derive(Debug)]
pub struct IdentityStatus {
    /// The detected identity name (if any)
    pub identity_name: Option<String>,
    /// The email being used
    pub email: Option<String>,
    /// The user name being used
    pub user_name: Option<String>,
    /// The configuration level where identity was found
    pub level: ConfigLevel,
    /// Whether we're in a git repository
    pub in_repository: bool,
    /// The remote URL (if in a repository)
    pub remote_url: Option<String>,
    /// The conditional directory pattern (if applicable)
    pub conditional_directory: Option<String>,
    /// Whether the identity is managed by gt
    pub is_managed: bool,
}

/// Execute the status command
pub fn execute(opts: &StatusOpts, ctx: &Context) -> Result<Output> {
    let path = opts
        .repo
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    ctx.debug(&format!("Getting status for {}", path.display()));

    // Detect identity with multi-level priority
    let status = detect_identity(&path, ctx)?;

    // Try to match email to a known identity (if config exists)
    let matched_identity = if let Some(ref email) = status.email {
        if let Some(config) = ctx.config.as_ref() {
            config.identities.iter()
                .find(|(_, identity_config)| identity_config.email == *email)
                .map(|(name, _)| name.clone())
        } else {
            None
        }
    } else {
        status.identity_name.clone()
    };

    // Build output
    let mut output = if let Some(ref identity) = matched_identity {
        if status.in_repository {
            Output::success(format!("Using identity '{}' ({})", identity, status.level))
        } else {
            Output::success(format!("Would use identity '{}' ({})", identity, status.level))
        }
    } else if let Some(ref email) = status.email {
        Output::success(format!("Using unmanaged identity: {}", email))
            .with_detail("warning", "This email is not managed by gt")
    } else {
        Output::success("No identity configured".to_string())
            .with_detail("warning", "Run 'gt id use <identity>' to set an identity")
    };

    // Add details if requested
    if opts.all {
        if let Some(ref identity) = matched_identity {
            output = output.with_detail("identity", identity);

            if let Some(config) = ctx.config.as_ref() {
                if let Some(identity_config) = config.identities.get(identity) {
                    // Get strategies list
                    let strategies: Vec<String> = identity_config
                        .strategies
                        .iter()
                        .map(|s| s.strategy_type.clone())
                        .collect();
                    let strategies_str = if strategies.is_empty() {
                        "none".to_string()
                    } else {
                        strategies.join(", ")
                    };

                    output = output
                        .with_detail("provider", &identity_config.provider)
                        .with_detail("strategies", &strategies_str);
                }
            }
        }

        if let Some(ref email) = status.email {
            output = output.with_detail("email", email);
        }

        if let Some(ref name) = status.user_name {
            output = output.with_detail("name", name);
        }

        if let Some(ref url) = status.remote_url {
            output = output.with_detail("remote", url);
        }

        output = output.with_detail("config_level", &status.level.to_string());
        output = output.with_detail("in_repository", &status.in_repository.to_string());
        output = output.with_detail("managed", &status.is_managed.to_string());

        if let Some(ref dir) = status.conditional_directory {
            output = output.with_detail("conditional_directory", dir);
        }
    }

    Ok(output)
}

/// Detect the current identity with multi-level priority
pub fn detect_identity(dir: &Path, ctx: &Context) -> Result<IdentityStatus> {
    let in_repo = dir.join(".git").exists();

    // Priority 1: Repository level (if in a git repo)
    if in_repo {
        if let Some(status) = detect_repository_identity(dir, ctx)? {
            return Ok(status);
        }
    }

    // Priority 2: Conditional/Directory level (works everywhere)
    if let Some(status) = detect_conditional_identity(dir, ctx)? {
        return Ok(IdentityStatus {
            in_repository: in_repo,
            remote_url: if in_repo {
                get_remote_url(dir, "origin", ctx).ok()
            } else {
                None
            },
            ..status
        });
    }

    // Priority 3: Global level
    if let Some(status) = detect_global_identity(ctx)? {
        return Ok(IdentityStatus {
            in_repository: in_repo,
            remote_url: if in_repo {
                get_remote_url(dir, "origin", ctx).ok()
            } else {
                None
            },
            ..status
        });
    }

    // Priority 4: No identity
    Ok(IdentityStatus {
        identity_name: None,
        email: None,
        user_name: None,
        level: ConfigLevel::None,
        in_repository: in_repo,
        remote_url: if in_repo {
            get_remote_url(dir, "origin", ctx).ok()
        } else {
            None
        },
        conditional_directory: None,
        is_managed: false,
    })
}

/// Detect identity from repository-level git config
fn detect_repository_identity(repo_path: &Path, ctx: &Context) -> Result<Option<IdentityStatus>> {
    // Try to get local (repository) config
    let email = match get_git_config_local(repo_path, "user.email", ctx) {
        Ok(email) => Some(email),
        Err(_) => None,
    };

    // If no local email, this isn't a repository-level identity
    if email.is_none() {
        return Ok(None);
    }

    let user_name = get_git_config_local(repo_path, "user.name", ctx).ok();
    let remote_url = get_remote_url(repo_path, "origin", ctx).ok();

    Ok(Some(IdentityStatus {
        identity_name: None, // Will be matched later by caller
        email,
        user_name,
        level: ConfigLevel::Repository,
        in_repository: true,
        remote_url,
        conditional_directory: None,
        is_managed: true, // Assume managed if local config is set
    }))
}

/// Detect identity from conditional includes
fn detect_conditional_identity(dir: &Path, ctx: &Context) -> Result<Option<IdentityStatus>> {
    let _conditional = ConditionalStrategy::new();

    // Find which identity matches this directory
    let includes = git_config::find_conditional_includes()?;
    ctx.debug(&format!("Found {} conditional includes", includes.len()));

    let dir_expanded = path::expand_tilde(dir)?;
    ctx.debug(&format!("Checking directory: {}", dir_expanded.display()));

    for include in &includes {
        ctx.debug(&format!("  Checking include: {} -> {}", include.condition, include.path));
        if let Some(pattern) = include.condition.strip_prefix("gitdir:") {
            ctx.debug(&format!("    Pattern: {}", pattern));
            if path_matches_gitdir_pattern(&dir_expanded, pattern)? {
                ctx.debug("    MATCH!");
                // Found a matching conditional include
                ctx.debug(&format!(
                    "Directory matches conditional pattern: {}",
                    pattern
                ));

                // Extract identity name from config path
                let config_path = Path::new(&include.path);
                let identity_name = config_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string());

                // Read email and name from the include file
                let include_path = path::expand_tilde(config_path)?;
                let (email, user_name) = if include_path.exists() {
                    let content = std::fs::read_to_string(&include_path).unwrap_or_default();
                    (
                        parse_value_from_gitconfig(&content, "email"),
                        parse_value_from_gitconfig(&content, "name"),
                    )
                } else {
                    (None, None)
                };

                return Ok(Some(IdentityStatus {
                    identity_name,
                    email,
                    user_name,
                    level: ConfigLevel::Conditional,
                    in_repository: false, // Will be updated by caller
                    remote_url: None,
                    conditional_directory: Some(pattern.to_string()),
                    is_managed: true,
                }));
            }
        }
    }

    Ok(None)
}

/// Detect identity from global git config
fn detect_global_identity(_ctx: &Context) -> Result<Option<IdentityStatus>> {
    let email = match git_config::get_config("user.email", git_config::ConfigScope::Global) {
        Ok(Some(email)) => Some(email),
        _ => None,
    };

    if email.is_none() {
        return Ok(None);
    }

    let user_name = git_config::get_config("user.name", git_config::ConfigScope::Global)
        .ok()
        .flatten();

    Ok(Some(IdentityStatus {
        identity_name: None,
        email,
        user_name,
        level: ConfigLevel::Global,
        in_repository: false,
        remote_url: None,
        conditional_directory: None,
        is_managed: false, // Global config is typically not gt-managed
    }))
}

/// Check if a path matches a gitdir pattern
fn path_matches_gitdir_pattern(path: &Path, pattern: &str) -> Result<bool> {
    // Normalize pattern
    let pattern_path = if pattern.starts_with('~') {
        path::expand_tilde(Path::new(pattern))?
    } else {
        std::path::PathBuf::from(pattern)
    };

    // Remove trailing slash from pattern for comparison
    let pattern_str = pattern_path.to_string_lossy();
    let pattern_normalized = pattern_str.trim_end_matches('/');
    let pattern_path = std::path::PathBuf::from(pattern_normalized);

    // Simple prefix matching
    if path.starts_with(&pattern_path) {
        return Ok(true);
    }

    // Check if any parent of path matches
    let mut current = path.to_path_buf();
    while let Some(parent) = current.parent() {
        if parent == pattern_path {
            return Ok(true);
        }
        if parent.as_os_str().is_empty() || parent == Path::new("/") {
            break;
        }
        current = parent.to_path_buf();
    }

    Ok(false)
}

/// Parse a value from gitconfig content
fn parse_value_from_gitconfig(content: &str, key: &str) -> Option<String> {
    let mut in_user_section = false;

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with('[') {
            in_user_section = line.to_lowercase().starts_with("[user]");
            continue;
        }

        if in_user_section {
            if let Some(value_part) = line.strip_prefix(key) {
                let rest = value_part.trim();
                if let Some(value) = rest.strip_prefix('=') {
                    return Some(value.trim().to_string());
                }
            }
        }
    }

    None
}

/// Get git config value (checks local, then global)
#[allow(dead_code)]
fn get_git_config(repo_path: &Path, key: &str, ctx: &Context) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("config")
        .arg(key)
        .output()?;

    if !output.status.success() {
        return Err(Error::GitCommand {
            message: format!("Git config {} not set", key),
        });
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    ctx.debug(&format!("Git config {} = {}", key, value));
    Ok(value)
}

/// Get git config value from local repository only (not global)
fn get_git_config_local(repo_path: &Path, key: &str, ctx: &Context) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("config")
        .arg("--local")
        .arg(key)
        .output()?;

    if !output.status.success() {
        return Err(Error::GitCommand {
            message: format!("Git config {} not set locally", key),
        });
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    ctx.debug(&format!("Git config --local {} = {}", key, value));
    Ok(value)
}

/// Get remote URL for a given remote name
fn get_remote_url(repo_path: &Path, remote: &str, ctx: &Context) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("remote")
        .arg("get-url")
        .arg(remote)
        .output()?;

    if !output.status.success() {
        return Err(Error::GitCommand {
            message: format!("Remote '{}' not found", remote),
        });
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    ctx.debug(&format!("Remote '{}' URL = {}", remote, url));
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_value_from_gitconfig() {
        let config = r#"
[user]
    email = test@example.com
    name = Test User
"#;
        assert_eq!(
            parse_value_from_gitconfig(config, "email"),
            Some("test@example.com".to_string())
        );
        assert_eq!(
            parse_value_from_gitconfig(config, "name"),
            Some("Test User".to_string())
        );
    }

    #[test]
    fn test_config_level_display() {
        assert_eq!(ConfigLevel::Repository.to_string(), "repository");
        assert_eq!(ConfigLevel::Conditional.to_string(), "conditional");
        assert_eq!(ConfigLevel::Global.to_string(), "global");
        assert_eq!(ConfigLevel::None.to_string(), "none");
    }
}
