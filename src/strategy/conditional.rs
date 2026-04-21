//! Git conditional includes strategy
//!
//! This strategy uses Git's includeIf directive to apply
//! configurations based on repository location.
//!
//! How it works:
//! 1. Creates a per-identity config file in ~/.gitconfig.d/{identity}
//! 2. Adds includeIf directive to ~/.gitconfig pointing to the identity config
//! 3. The includeIf condition is based on directory (gitdir:PATH/)
//!
//! Example:
//! ```ini
//! [includeIf "gitdir:~/work/"]
//!     path = ~/.gitconfig.d/work
//! ```

use std::path::{Path, PathBuf};

use crate::core::identity::Identity;
use crate::core::path;
use crate::core::repo::Repo;
use crate::error::{Error, Result};
use crate::io::git_config;
use crate::strategy::{ApplyResult, SetupStep, Strategy, StrategyType, ValidationResult};

/// Git conditional includes strategy
pub struct ConditionalStrategy {
    /// Directory for include files
    config_dir: String,
    /// Whether to include core.sshCommand in the identity config
    use_ssh_command: bool,
}

impl ConditionalStrategy {
    /// Create a new conditional strategy
    #[must_use]
    pub fn new() -> Self {
        Self {
            config_dir: "~/.gitconfig.d".to_string(),
            use_ssh_command: true,
        }
    }

    /// Create with custom config directory
    #[must_use]
    pub fn with_config_dir(dir: impl Into<String>) -> Self {
        Self {
            config_dir: dir.into(),
            use_ssh_command: true,
        }
    }

    /// Enable or disable SSH command in config
    #[must_use]
    pub fn with_ssh_command(mut self, use_ssh_command: bool) -> Self {
        self.use_ssh_command = use_ssh_command;
        self
    }

    /// Get the config directory path expanded
    pub fn config_dir_expanded(&self) -> Result<PathBuf> {
        path::expand_tilde(Path::new(&self.config_dir))
    }

    /// Get the path to the identity's include file
    pub fn identity_config_path(&self, identity_name: &str) -> Result<PathBuf> {
        let config_dir = self.config_dir_expanded()?;
        Ok(config_dir.join(identity_name))
    }

    /// Create the identity include file
    ///
    /// Creates ~/.gitconfig.d/{identity} with user.email, user.name,
    /// and optionally core.sshCommand for SSH key selection.
    pub fn create_identity_config(
        &self,
        identity: &Identity,
        ssh_key_path: Option<&str>,
    ) -> Result<PathBuf> {
        let config_path = self.identity_config_path(&identity.name)?;

        // Ensure config directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Build config content
        let ssh_key = if self.use_ssh_command {
            ssh_key_path.or_else(|| {
                // Use the identity's SSH key path if available
                identity.ssh.as_ref().and_then(|s| s.key_path.as_deref())
            })
        } else {
            None
        };

        git_config::write_include_file(
            &config_path,
            &identity.email,
            &identity.user_name,
            ssh_key,
        )?;

        Ok(config_path)
    }

    /// Add conditional include to ~/.gitconfig
    ///
    /// Adds: [includeIf "gitdir:DIRECTORY/"] path = CONFIG_PATH
    pub fn add_conditional_include(&self, directory: &str, identity_name: &str) -> Result<()> {
        let config_path = self.identity_config_path(identity_name)?;
        let config_path_str = path::contract_tilde(&config_path);

        // Normalize directory path - ensure it ends with /
        let normalized_dir = normalize_gitdir_path(directory)?;
        let condition = format!("gitdir:{}", normalized_dir);

        git_config::add_conditional_include(&condition, &config_path_str)
    }

    /// Remove conditional include from ~/.gitconfig
    pub fn remove_conditional_include(&self, directory: &str) -> Result<()> {
        let normalized_dir = normalize_gitdir_path(directory)?;
        let condition = format!("gitdir:{}", normalized_dir);
        git_config::remove_conditional_include(&condition)
    }

    /// Get all directories configured for an identity
    pub fn get_identity_directories(&self, identity_name: &str) -> Result<Vec<String>> {
        let includes = git_config::find_conditional_includes()?;
        let identity_config_path = self.identity_config_path(identity_name)?;
        let identity_config_str = path::contract_tilde(&identity_config_path);

        let mut directories = Vec::new();
        for include in includes {
            // Check if this include points to our identity's config
            if include.path == identity_config_str
                || include.path == identity_config_path.to_string_lossy()
            {
                // Extract directory from condition (gitdir:PATH/)
                if let Some(dir) = include.condition.strip_prefix("gitdir:") {
                    directories.push(dir.to_string());
                }
            }
        }

        Ok(directories)
    }

    /// Check if a directory matches any conditional include for an identity
    pub fn directory_matches_identity(
        &self,
        directory: &Path,
        identity_name: &str,
    ) -> Result<bool> {
        let directories = self.get_identity_directories(identity_name)?;
        let dir_expanded = path::expand_tilde(directory)?;

        for pattern in directories {
            if path_matches_gitdir_pattern(&dir_expanded, &pattern)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Find which identity (if any) matches the given directory
    pub fn find_matching_identity(&self, directory: &Path) -> Result<Option<String>> {
        let includes = git_config::find_conditional_includes()?;
        let dir_expanded = path::expand_tilde(directory)?;

        for include in &includes {
            if let Some(pattern) = include.condition.strip_prefix("gitdir:") {
                if path_matches_gitdir_pattern(&dir_expanded, pattern)? {
                    // Extract identity name from config path
                    // Path format: ~/.gitconfig.d/{identity}
                    let config_path = Path::new(&include.path);
                    if let Some(name) = config_path.file_name() {
                        return Ok(Some(name.to_string_lossy().to_string()));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Get the email configured for a conditional include that matches the directory
    pub fn get_matching_email(&self, directory: &Path) -> Result<Option<String>> {
        let includes = git_config::find_conditional_includes()?;
        let dir_expanded = path::expand_tilde(directory)?;

        for include in &includes {
            if let Some(pattern) = include.condition.strip_prefix("gitdir:") {
                if path_matches_gitdir_pattern(&dir_expanded, pattern)? {
                    // Read the email from the include file
                    let include_path = path::expand_tilde(Path::new(&include.path))?;
                    if include_path.exists() {
                        if let Ok(content) = std::fs::read_to_string(&include_path) {
                            if let Some(email) = parse_email_from_gitconfig(&content) {
                                return Ok(Some(email));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Setup conditional strategy for an identity with a specific directory
    ///
    /// This is the main entry point for setting up the conditional strategy.
    /// It creates the identity config file and adds the includeIf directive.
    pub fn setup_for_directory(
        &self,
        identity: &Identity,
        directory: &str,
        ssh_key_path: Option<&str>,
    ) -> Result<ApplyResult> {
        let mut result = ApplyResult::new();

        // Step 1: Create identity config file
        let config_path = self.create_identity_config(identity, ssh_key_path)?;
        result.add_change(format!(
            "Created identity config: {}",
            path::contract_tilde(&config_path)
        ));

        // Step 2: Add conditional include
        self.add_conditional_include(directory, &identity.name)?;
        result.add_change(format!(
            "Added conditional include for {} -> {}",
            directory, identity.name
        ));

        Ok(result)
    }

    /// Remove all configuration for an identity
    pub fn cleanup_identity(&self, identity_name: &str) -> Result<()> {
        // Remove all conditional includes for this identity
        let directories = self.get_identity_directories(identity_name)?;
        for dir in directories {
            self.remove_conditional_include(&dir)?;
        }

        // Remove the identity config file
        let config_path = self.identity_config_path(identity_name)?;
        if config_path.exists() {
            std::fs::remove_file(config_path)?;
        }

        Ok(())
    }
}

impl Default for ConditionalStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl Strategy for ConditionalStrategy {
    fn strategy_type(&self) -> StrategyType {
        StrategyType::Conditional
    }

    fn apply(&self, identity: &Identity, repo: &Repo) -> Result<ApplyResult> {
        let mut result = ApplyResult::new();

        // For conditional strategy when applied to a specific repo,
        // we just set the local git config as a fallback.
        // The actual conditional includes are set up via setup_for_directory.

        // Check if repo path matches any configured conditional include
        let matches = self.directory_matches_identity(&repo.path, &identity.name)?;

        if !matches {
            result.add_warning(format!(
                "Repository at {} may not match conditional include pattern for '{}'",
                repo.path.display(),
                identity.name
            ));
            result.add_warning(format!(
                "Consider running: gt id use {} --repo {} to add this directory",
                identity.name,
                repo.path.display()
            ));
        }

        // Set local git config for this repo as an override
        repo.set_config("user.email", &identity.email)?;
        repo.set_config("user.name", &identity.user_name)?;

        result.add_change(format!("Set user.email to {}", identity.email));
        result.add_change(format!("Set user.name to {}", identity.user_name));

        Ok(result)
    }

    fn remove(&self, _identity: &Identity, repo: &Repo) -> Result<()> {
        // Remove local config overrides
        let _ = std::process::Command::new("git")
            .current_dir(&repo.path)
            .args(["config", "--local", "--unset", "user.email"])
            .output();

        let _ = std::process::Command::new("git")
            .current_dir(&repo.path)
            .args(["config", "--local", "--unset", "user.name"])
            .output();

        Ok(())
    }

    fn is_active(&self, identity: &Identity, repo: &Repo) -> Result<bool> {
        // Check if the repo has the identity's email configured
        // This could be from local config or from a matching conditional include
        let email = repo.get_config("user.email")?;
        Ok(email.as_ref() == Some(&identity.email))
    }

    fn validate(&self) -> Result<ValidationResult> {
        let errors = Vec::new();
        let mut warnings = Vec::new();

        // Check if config directory exists
        let config_dir = self.config_dir_expanded()?;
        if !config_dir.exists() {
            warnings.push(format!(
                "Config directory {} does not exist (will be created)",
                self.config_dir
            ));
        }

        // Check if git is available
        if std::process::Command::new("git")
            .arg("--version")
            .output()
            .is_err()
        {
            return Err(Error::ToolNotFound {
                tool: "git".to_string(),
            });
        }

        Ok(ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    fn setup_requirements(&self) -> Vec<SetupStep> {
        let config_dir_exists = self
            .config_dir_expanded()
            .map(|p| p.exists())
            .unwrap_or(false);

        vec![
            SetupStep {
                description: format!("Config directory exists ({})", self.config_dir),
                complete: config_dir_exists,
            },
            SetupStep {
                description: "Identity config file created".to_string(),
                complete: false, // Checked per-identity
            },
            SetupStep {
                description: "includeIf directive added to ~/.gitconfig".to_string(),
                complete: false, // Checked per-identity
            },
        ]
    }
}

/// Normalize a directory path for gitdir pattern
///
/// - Expands ~ to home directory representation
/// - Ensures path ends with /
/// - Handles various path formats
fn normalize_gitdir_path(directory: &str) -> Result<String> {
    let mut path = directory.to_string();

    // Trim whitespace
    path = path.trim().to_string();

    // Ensure it ends with /
    if !path.ends_with('/') {
        path.push('/');
    }

    // If path starts with ~, keep it as ~ (git understands this)
    // Otherwise, try to make it relative to home if possible
    if !path.starts_with('~') && !path.starts_with('/') {
        // Assume it's relative to home
        path = format!("~/{}", path);
    }

    Ok(path)
}

/// Check if a path matches a gitdir pattern
///
/// Git's gitdir patterns support:
/// - Literal paths (with trailing /)
/// - ~ for home directory
/// - ** for arbitrary directory depth (globbing)
fn path_matches_gitdir_pattern(path: &Path, pattern: &str) -> Result<bool> {
    // Normalize pattern
    let pattern_path = if pattern.starts_with('~') {
        path::expand_tilde(Path::new(pattern))?
    } else {
        PathBuf::from(pattern)
    };

    // Remove trailing slash from pattern for comparison
    let pattern_str = pattern_path.to_string_lossy();
    let pattern_normalized = pattern_str.trim_end_matches('/');
    let pattern_path = PathBuf::from(pattern_normalized);

    // Simple prefix matching - check if path starts with pattern
    // This handles the case where pattern is ~/work/ and path is ~/work/project/.git
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

/// Parse email from a gitconfig file content
fn parse_email_from_gitconfig(content: &str) -> Option<String> {
    // Simple parser - look for email = value under [user] section
    let mut in_user_section = false;

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with('[') {
            in_user_section = line.to_lowercase().starts_with("[user]");
            continue;
        }

        if in_user_section {
            if let Some(email_part) = line.strip_prefix("email") {
                let rest = email_part.trim();
                if let Some(value) = rest.strip_prefix('=') {
                    return Some(value.trim().to_string());
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::provider::Provider;

    fn test_identity() -> Identity {
        Identity {
            name: "work".to_string(),
            email: "work@company.com".to_string(),
            user_name: "Work User".to_string(),
            provider: Provider::GitHub,
            ssh: None,
            strategy: Some("conditional".to_string()),
        }
    }

    #[test]
    fn test_strategy_type() {
        let strategy = ConditionalStrategy::new();
        assert_eq!(strategy.strategy_type(), StrategyType::Conditional);
    }

    #[test]
    fn test_normalize_gitdir_path() {
        // Basic path
        let result = normalize_gitdir_path("~/work").unwrap();
        assert_eq!(result, "~/work/");

        // Path with trailing slash
        let result = normalize_gitdir_path("~/work/").unwrap();
        assert_eq!(result, "~/work/");

        // Relative path
        let result = normalize_gitdir_path("work").unwrap();
        assert_eq!(result, "~/work/");
    }

    #[test]
    fn test_path_matches_gitdir_pattern() {
        // This test needs actual paths, so we use temp paths
        let temp = std::env::temp_dir();
        let work_dir = temp.join("work");
        let project_dir = temp.join("work").join("project");

        // Create the temp path string
        let pattern = format!("{}/", work_dir.display());

        // Project should match work pattern
        let result = path_matches_gitdir_pattern(&project_dir, &pattern).unwrap();
        assert!(result, "Project dir should match work pattern");

        // Work dir should match its own pattern
        let result = path_matches_gitdir_pattern(&work_dir, &pattern).unwrap();
        assert!(result, "Work dir should match its own pattern");

        // Different dir should not match
        let other_dir = temp.join("other");
        let result = path_matches_gitdir_pattern(&other_dir, &pattern).unwrap();
        assert!(!result, "Other dir should not match work pattern");
    }

    #[test]
    fn test_parse_email_from_gitconfig() {
        let config = r#"
[user]
    email = test@example.com
    name = Test User
"#;
        let email = parse_email_from_gitconfig(config);
        assert_eq!(email, Some("test@example.com".to_string()));

        // With different formatting
        let config2 = "[user]\nemail=test2@example.com";
        let email2 = parse_email_from_gitconfig(config2);
        assert_eq!(email2, Some("test2@example.com".to_string()));

        // No user section
        let config3 = "[core]\nautocrlf = true";
        let email3 = parse_email_from_gitconfig(config3);
        assert!(email3.is_none());
    }

    #[test]
    fn test_identity_config_path() {
        let strategy = ConditionalStrategy::new();
        let path = strategy.identity_config_path("work").unwrap();
        assert!(path.to_string_lossy().contains("gitconfig.d"));
        assert!(path.to_string_lossy().contains("work"));
    }

    #[test]
    fn test_custom_config_dir() {
        let strategy = ConditionalStrategy::with_config_dir("~/.config/git-identities");
        let path = strategy.identity_config_path("personal").unwrap();
        assert!(path.to_string_lossy().contains("git-identities"));
        assert!(path.to_string_lossy().contains("personal"));
    }
}
