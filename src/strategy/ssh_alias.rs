//! SSH hostname alias strategy
//!
//! This strategy modifies Git URLs to use identity-specific hostnames
//! that are mapped via SSH config to the correct provider and key.
//!
//! How it works:
//! 1. Generates an SSH key for the identity (if needed)
//! 2. Creates SSH config entry mapping alias hostname to real provider
//! 3. Transforms Git repository URLs to use the alias hostname
//!
//! Example:
//! - Identity: work
//! - Provider: github.com
//! - SSH Key: ~/.ssh/id_gt_work
//! - SSH Alias: gt-work.github.com
//! - Git URL: git@gt-work.github.com:user/repo.git

use crate::core::identity::Identity;
use crate::core::path;
use crate::core::repo::Repo;
use crate::core::url::GitUrl;
use crate::error::{Error, Result};
use crate::io::ssh_config::{SshConfig, SshHostEntry};
use crate::io::ssh_key::{generate_key, verify_key, KeyGenOptions, KeyType};
use crate::strategy::{ApplyResult, SetupStep, Strategy, StrategyType, ValidationResult};

/// SSH hostname alias strategy
pub struct SshAliasStrategy {
    /// Prefix for hostname aliases (default: "gt")
    prefix: String,
}

impl SshAliasStrategy {
    /// Create a new SSH alias strategy with default "gt" prefix
    #[must_use]
    pub fn new() -> Self {
        Self {
            prefix: "gt".to_string(),
        }
    }

    /// Create with custom prefix
    #[must_use]
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }

    /// Generate the SSH hostname for an identity
    fn hostname(&self, identity: &Identity) -> String {
        format!(
            "{}-{}.{}",
            self.prefix,
            identity.name,
            identity.provider.hostname()
        )
    }

    /// Transform a URL to use this identity
    fn transform_url(&self, url: &GitUrl, identity: &Identity) -> Result<String> {
        let new_host = self.hostname(identity);
        Ok(format!("git@{}:{}", new_host, url.path))
    }

    /// Ensure SSH key exists for the identity
    fn ensure_ssh_key(&self, identity: &Identity) -> Result<String> {
        let key_path = identity.ssh_key_path();
        let key_path_expanded = path::expand_tilde(std::path::Path::new(&key_path))?;

        // Check if key already exists and is valid
        if verify_key(&key_path_expanded).unwrap_or(false) {
            return Ok(key_path);
        }

        // Generate new key
        let key_type = identity
            .ssh
            .as_ref()
            .and_then(|s| s.key_type.as_ref())
            .and_then(|t| KeyType::from_str(t))
            .unwrap_or(KeyType::Ed25519);

        let comment = format!("{} <{}>", identity.user_name, identity.email);
        let opts = match key_type {
            KeyType::Ed25519 => KeyGenOptions::ed25519(key_path_expanded.clone(), comment),
            KeyType::Rsa => {
                let bits = identity
                    .ssh
                    .as_ref()
                    .and_then(|s| s.key_bits)
                    .unwrap_or(4096);
                KeyGenOptions::rsa(key_path_expanded.clone(), comment, bits)
            }
            KeyType::Ecdsa => {
                let bits = identity
                    .ssh
                    .as_ref()
                    .and_then(|s| s.key_bits)
                    .unwrap_or(521);
                KeyGenOptions::ecdsa(key_path_expanded.clone(), comment, bits)
            }
        };

        generate_key(&opts)?;
        Ok(key_path)
    }

    /// Ensure SSH config entry exists for the identity
    fn ensure_ssh_config(&self, identity: &Identity, key_path: &str) -> Result<()> {
        let ssh_config_path = path::ssh_config_path()?;

        // Load existing config (or create new)
        let mut config = if ssh_config_path.exists() {
            SshConfig::load(&ssh_config_path)?
        } else {
            SshConfig::default()
        };

        // Create SSH host entry
        let alias_hostname = self.hostname(identity);
        let entry = SshHostEntry::new(&alias_hostname)
            .with_hostname(identity.provider.hostname())
            .with_user("git")
            .with_identity_file(key_path)
            .with_identities_only(true)
            .with_preferred_auth("publickey");

        // Upsert (update or insert)
        config.upsert_host(entry);

        // Save config
        config.save(&ssh_config_path)?;

        Ok(())
    }

    /// Set up SSH infrastructure for an identity (public method for commands)
    ///
    /// This generates the SSH key and creates the SSH config entry.
    /// Returns the key path and whether the key was newly created.
    pub fn setup_identity(&self, identity: &Identity, force: bool) -> Result<(String, bool)> {
        let key_path = identity.ssh_key_path();
        let key_path_expanded = path::expand_tilde(std::path::Path::new(&key_path))?;

        // Check if key already exists
        let key_exists = verify_key(&key_path_expanded).unwrap_or(false);

        if key_exists && !force {
            // Key exists and we're not forcing - just ensure SSH config is up to date
            self.ensure_ssh_config(identity, &key_path)?;
            return Ok((key_path, false));
        }

        // Generate key (or regenerate if force)
        if key_exists && force {
            // Remove existing key before regenerating
            let _ = std::fs::remove_file(&key_path_expanded);
            let _ = std::fs::remove_file(key_path_expanded.with_extension("pub"));
        }

        let key_type = identity
            .ssh
            .as_ref()
            .and_then(|s| s.key_type.as_ref())
            .and_then(|t| KeyType::from_str(t))
            .unwrap_or(KeyType::Ed25519);

        let comment = format!("{} <{}>", identity.user_name, identity.email);
        let opts = match key_type {
            KeyType::Ed25519 => KeyGenOptions::ed25519(key_path_expanded.clone(), comment),
            KeyType::Rsa => {
                let bits = identity
                    .ssh
                    .as_ref()
                    .and_then(|s| s.key_bits)
                    .unwrap_or(4096);
                KeyGenOptions::rsa(key_path_expanded.clone(), comment, bits)
            }
            KeyType::Ecdsa => {
                let bits = identity
                    .ssh
                    .as_ref()
                    .and_then(|s| s.key_bits)
                    .unwrap_or(521);
                KeyGenOptions::ecdsa(key_path_expanded.clone(), comment, bits)
            }
        };

        generate_key(&opts)?;

        // Create SSH config entry
        self.ensure_ssh_config(identity, &key_path)?;

        Ok((key_path, true))
    }
}

impl Default for SshAliasStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl Strategy for SshAliasStrategy {
    fn strategy_type(&self) -> StrategyType {
        StrategyType::SshAlias
    }

    fn apply(&self, identity: &Identity, repo: &Repo) -> Result<ApplyResult> {
        let mut result = ApplyResult::new();

        // Step 1: Ensure SSH key exists
        let key_path = self.ensure_ssh_key(identity)?;
        result.add_change(format!("SSH key: {}", key_path));

        // Step 2: Ensure SSH config entry exists
        self.ensure_ssh_config(identity, &key_path)?;
        result.add_change(format!(
            "SSH config entry: {} -> {}",
            self.hostname(identity),
            identity.provider.hostname()
        ));

        // Step 3: Transform repository URL
        let current_url = repo
            .parsed_url
            .as_ref()
            .ok_or_else(|| Error::UrlUnrecognized {
                url: repo.remote_url.clone().unwrap_or_default(),
            })?;

        let new_url = self.transform_url(current_url, identity)?;

        // Only update if URL is different
        if repo.remote_url.as_ref() != Some(&new_url) {
            repo.set_remote_url("origin", &new_url)?;
            result.add_change(format!("Repository URL: {}", new_url));
        } else {
            result.add_warning("Repository URL already configured".to_string());
        }

        Ok(result)
    }

    fn remove(&self, identity: &Identity, repo: &Repo) -> Result<()> {
        // Restore the original URL
        let current_url = repo
            .parsed_url
            .as_ref()
            .ok_or_else(|| Error::UrlUnrecognized {
                url: repo.remote_url.clone().unwrap_or_default(),
            })?;

        // Check if this identity is active
        if current_url.identity.as_ref() == Some(&identity.name) {
            let restored = current_url.without_identity()?;
            repo.set_remote_url("origin", &restored.to_string())?;
        }

        Ok(())
    }

    fn is_active(&self, identity: &Identity, repo: &Repo) -> Result<bool> {
        let expected_host = self.hostname(identity);

        Ok(repo
            .parsed_url
            .as_ref()
            .map_or(false, |url| url.host == expected_host))
    }

    fn validate(&self) -> Result<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check for ssh-keygen
        if std::process::Command::new("ssh-keygen")
            .arg("-V")
            .output()
            .is_err()
        {
            errors.push("ssh-keygen not found - required for key generation".to_string());
        }

        // Check for git
        if std::process::Command::new("git")
            .arg("--version")
            .output()
            .is_err()
        {
            errors.push("git not found - required for repository operations".to_string());
        }

        // Check for SSH directory
        if let Ok(ssh_dir) = path::ssh_dir() {
            if !ssh_dir.exists() {
                warnings.push("SSH directory does not exist (will be created)".to_string());
            }
        }

        // Check for SSH config
        if let Ok(ssh_config) = path::ssh_config_path() {
            if !ssh_config.exists() {
                warnings.push("SSH config does not exist (will be created)".to_string());
            }
        }

        Ok(ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    fn setup_requirements(&self) -> Vec<SetupStep> {
        let ssh_dir_exists = path::ssh_dir().map(|p| p.exists()).unwrap_or(false);

        let ssh_config_exists = path::ssh_config_path().map(|p| p.exists()).unwrap_or(false);

        vec![
            SetupStep {
                description: "SSH directory exists (~/.ssh)".to_string(),
                complete: ssh_dir_exists,
            },
            SetupStep {
                description: "SSH config accessible".to_string(),
                complete: ssh_config_exists || ssh_dir_exists,
            },
            SetupStep {
                description: "ssh-keygen available".to_string(),
                complete: std::process::Command::new("ssh-keygen")
                    .arg("-V")
                    .output()
                    .is_ok(),
            },
            SetupStep {
                description: "git available".to_string(),
                complete: std::process::Command::new("git")
                    .arg("--version")
                    .output()
                    .is_ok(),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::identity::Identity;
    use crate::core::provider::Provider;

    fn test_identity() -> Identity {
        Identity {
            name: "work".to_string(),
            email: "work@company.com".to_string(),
            user_name: "Work User".to_string(),
            provider: Provider::GitHub,
            ssh: None,
            strategy: None,
        }
    }

    #[test]
    fn test_hostname_generation() {
        let strategy = SshAliasStrategy::new();
        let identity = test_identity();

        assert_eq!(strategy.hostname(&identity), "gt-work.github.com");
    }

    #[test]
    fn test_custom_prefix() {
        let strategy = SshAliasStrategy::with_prefix("myid");
        let identity = test_identity();

        assert_eq!(strategy.hostname(&identity), "myid-work.github.com");
    }

    #[test]
    fn test_strategy_type() {
        let strategy = SshAliasStrategy::new();
        assert_eq!(strategy.strategy_type(), StrategyType::SshAlias);
    }

    #[test]
    fn test_url_transformation() {
        let strategy = SshAliasStrategy::new();
        let identity = test_identity();

        let url = GitUrl {
            protocol: crate::core::url::Protocol::Ssh,
            user: Some("git".to_string()),
            host: "github.com".to_string(),
            port: None,
            path: "user/repo.git".to_string(),
            provider: Some(Provider::GitHub),
            identity: None,
        };

        let transformed = strategy.transform_url(&url, &identity).unwrap();
        assert_eq!(transformed, "git@gt-work.github.com:user/repo.git");
    }
}
