//! TOML configuration for gt
//!
//! This module handles reading and writing the gt configuration file.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Main gt configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GtConfig {
    /// Default settings
    #[serde(default)]
    pub defaults: Defaults,

    /// SSH settings
    #[serde(default)]
    pub ssh: SshSettings,

    /// Backup settings
    #[serde(default)]
    pub backup: BackupSettings,

    /// UI settings
    #[serde(default)]
    pub ui: UiSettings,

    /// Provider configurations
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,

    /// Identity configurations
    #[serde(default)]
    pub identities: HashMap<String, IdentityConfig>,

    /// Strategy-specific settings
    #[serde(default)]
    pub strategy: StrategySettings,
}

/// Default settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Defaults {
    /// Default identity name
    pub identity: Option<String>,
    /// Default strategy
    pub strategy: Option<String>,
    /// Default provider
    pub provider: Option<String>,
}

/// SSH settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshSettings {
    /// Key type (ed25519, rsa)
    #[serde(default = "default_key_type")]
    pub key_type: String,
    /// RSA key bits
    #[serde(default = "default_rsa_bits")]
    pub rsa_bits: u32,
    /// Key file prefix
    #[serde(default = "default_key_prefix")]
    pub key_prefix: String,
    /// Comment format
    #[serde(default = "default_comment_format")]
    pub comment_format: String,
}

impl Default for SshSettings {
    fn default() -> Self {
        Self {
            key_type: default_key_type(),
            rsa_bits: default_rsa_bits(),
            key_prefix: default_key_prefix(),
            comment_format: default_comment_format(),
        }
    }
}

fn default_key_type() -> String {
    "ed25519".to_string()
}

fn default_rsa_bits() -> u32 {
    4096
}

fn default_key_prefix() -> String {
    "id_gt_".to_string()
}

fn default_comment_format() -> String {
    "{identity}@{provider}".to_string()
}

/// Backup settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSettings {
    /// Enable backups
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Maximum backup count per file
    #[serde(default = "default_max_backups")]
    pub max_count: usize,
    /// Backup directory (empty = same as original)
    #[serde(default)]
    pub directory: Option<String>,
}

impl Default for BackupSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_count: 2,
            directory: None,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_max_backups() -> usize {
    2
}

/// UI settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    /// Enable colors
    #[serde(default = "default_true")]
    pub color: bool,
    /// Enable interactive prompts
    #[serde(default = "default_true")]
    pub interactive: bool,
    /// Show progress indicators
    #[serde(default = "default_true")]
    pub progress: bool,
    /// Editor command
    #[serde(default)]
    pub editor: Option<String>,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            color: true,
            interactive: true,
            progress: true,
            editor: None,
        }
    }
}

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Hostname
    pub hostname: String,
    /// SSH user
    #[serde(default = "default_ssh_user")]
    pub ssh_user: String,
    /// URL pattern
    #[serde(default)]
    pub url_pattern: Option<String>,
}

fn default_ssh_user() -> String {
    "git".to_string()
}

/// Strategy configuration for a specific strategy type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    /// Strategy type ("ssh", "conditional", "url")
    #[serde(rename = "type")]
    pub strategy_type: String,

    /// Detection priority (lower = higher priority)
    /// Conditional: 10, URL: 50, SSH: 100
    #[serde(default = "default_priority")]
    pub priority: u32,

    /// Whether this strategy is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    // SSH-specific
    /// Use SSH hostname alias (transforms URLs to git@gt-{identity}.provider.com)
    #[serde(default)]
    pub use_hostname_alias: bool,

    // Conditional-specific
    /// Directory pattern for conditional includes
    #[serde(default)]
    pub directory: Option<String>,

    // URL rewrite-specific
    /// Scope for URL rewriting (organization or user name)
    #[serde(default)]
    pub scope: Option<String>,

    /// URL patterns to match
    #[serde(default)]
    pub patterns: Option<Vec<String>>,
}

fn default_priority() -> u32 {
    100
}

impl StrategyConfig {
    /// Get default priority for a strategy type
    pub fn default_priority_for_type(strategy_type: &str) -> u32 {
        match strategy_type {
            "conditional" => 10,
            "url" => 50,
            "ssh" => 100,
            _ => 100,
        }
    }
}

/// Identity configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConfig {
    /// Email
    pub email: String,
    /// User name
    pub name: String,
    /// Provider
    pub provider: String,

    /// Legacy single strategy field (for backward compatibility)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,

    /// SSH configuration
    #[serde(default)]
    pub ssh: Option<IdentitySshConfig>,

    /// Legacy conditional strategy config (for backward compatibility)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditional: Option<ConditionalConfig>,

    /// Legacy URL rewrite config (for backward compatibility)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_rewrite: Option<UrlRewriteConfig>,

    /// Multiple strategy configurations (new format)
    #[serde(default)]
    pub strategies: Vec<StrategyConfig>,
}

/// SSH configuration for an identity
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IdentitySshConfig {
    /// Key path
    pub key_path: Option<String>,
    /// Key type
    pub key_type: Option<String>,
    /// Use SSH hostname alias (transforms URLs to git@gt-{identity}.provider.com)
    #[serde(default)]
    pub use_hostname_alias: bool,
}

/// Conditional strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConditionalConfig {
    /// Directory for this identity
    pub directory: Option<String>,
}

/// URL rewrite configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UrlRewriteConfig {
    /// URL patterns to match
    pub patterns: Option<Vec<String>>,
}

/// Strategy-specific settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StrategySettings {
    /// SSH alias settings
    #[serde(default)]
    pub ssh_alias: SshAliasSettings,
    /// Conditional settings
    #[serde(default)]
    pub conditional: ConditionalSettings,
    /// URL rewrite settings
    #[serde(default)]
    pub url_rewrite: UrlRewriteSettings,
}

/// SSH alias strategy settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshAliasSettings {
    /// Hostname prefix
    #[serde(default = "default_prefix")]
    pub prefix: String,
    /// Include User directive
    #[serde(default = "default_true")]
    pub include_user: bool,
}

impl Default for SshAliasSettings {
    fn default() -> Self {
        Self {
            prefix: default_prefix(),
            include_user: true,
        }
    }
}

fn default_prefix() -> String {
    "gt".to_string()
}

/// Conditional strategy settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalSettings {
    /// Config directory
    #[serde(default = "default_config_dir")]
    pub config_dir: String,
    /// Use core.sshCommand
    #[serde(default = "default_true")]
    pub use_ssh_command: bool,
}

impl Default for ConditionalSettings {
    fn default() -> Self {
        Self {
            config_dir: default_config_dir(),
            use_ssh_command: true,
        }
    }
}

fn default_config_dir() -> String {
    "~/.gitconfig.d".to_string()
}

/// URL rewrite strategy settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlRewriteSettings {
    /// Default scope
    #[serde(default = "default_scope")]
    pub scope: String,
}

impl Default for UrlRewriteSettings {
    fn default() -> Self {
        Self {
            scope: default_scope(),
        }
    }
}

fn default_scope() -> String {
    "organization".to_string()
}

impl GtConfig {
    /// Load configuration from a file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::ConfigNotFound {
                    path: path.to_owned(),
                }
            } else {
                Error::Io(e)
            }
        })?;

        let mut config: Self = toml::from_str(&content)?;

        // Migrate legacy strategies for all identities
        for (_, identity) in config.identities.iter_mut() {
            identity.migrate_legacy_strategies();
        }

        Ok(config)
    }

    /// Save configuration to a file
    pub fn save(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get an identity by name
    pub fn get_identity(&self, name: &str) -> Result<&IdentityConfig> {
        self.identities.get(name).ok_or(Error::IdentityNotFound {
            name: name.to_string(),
        })
    }

    /// Add or update an identity
    pub fn set_identity(&mut self, name: String, config: IdentityConfig) {
        self.identities.insert(name, config);
    }

    /// Remove an identity
    pub fn remove_identity(&mut self, name: &str) -> Option<IdentityConfig> {
        self.identities.remove(name)
    }
}

impl IdentityConfig {
    /// Add a strategy configuration
    pub fn add_strategy(&mut self, strategy: StrategyConfig) {
        // Check if strategy with same type and discriminator already exists
        let discriminator = Self::get_strategy_discriminator(&strategy);

        // Remove existing strategy with same discriminator
        self.strategies.retain(|s| {
            Self::get_strategy_discriminator(s) != discriminator
        });

        // Add new strategy
        self.strategies.push(strategy);
    }

    /// Remove a strategy by type and optional discriminator
    pub fn remove_strategy(&mut self, strategy_type: &str, discriminator: Option<&str>) -> bool {
        let initial_len = self.strategies.len();

        self.strategies.retain(|s| {
            if s.strategy_type != strategy_type {
                return true; // Keep if different type
            }

            // If discriminator specified, only remove matching discriminator
            if let Some(disc) = discriminator {
                let strat_disc = Self::get_strategy_discriminator(s);
                strat_disc.as_deref() != Some(disc)
            } else {
                false // Remove all of this type
            }
        });

        self.strategies.len() < initial_len
    }

    /// Find a strategy by type
    pub fn find_strategy(&self, strategy_type: &str) -> Option<&StrategyConfig> {
        self.strategies
            .iter()
            .find(|s| s.strategy_type == strategy_type && s.enabled)
    }

    /// Find a strategy variant by type and discriminator
    pub fn find_strategy_variant(
        &self,
        strategy_type: &str,
        discriminator: Option<&str>,
    ) -> Option<&StrategyConfig> {
        self.strategies.iter().find(|s| {
            if s.strategy_type != strategy_type || !s.enabled {
                return false;
            }

            if let Some(disc) = discriminator {
                let strat_disc = Self::get_strategy_discriminator(s);
                strat_disc.as_deref() == Some(disc)
            } else {
                true
            }
        })
    }

    /// Get all enabled strategies sorted by priority
    pub fn get_sorted_strategies(&self) -> Vec<&StrategyConfig> {
        let mut strategies: Vec<&StrategyConfig> = self
            .strategies
            .iter()
            .filter(|s| s.enabled)
            .collect();

        strategies.sort_by_key(|s| s.priority);
        strategies
    }

    /// Get the discriminator for a strategy (for detecting duplicates)
    fn get_strategy_discriminator(strategy: &StrategyConfig) -> Option<String> {
        match strategy.strategy_type.as_str() {
            "conditional" => strategy.directory.clone(),
            "url" => strategy.scope.clone(),
            "ssh" => Some("default".to_string()), // SSH has no discriminator
            _ => None,
        }
    }

    /// Migrate from legacy single-strategy format to multi-strategy
    pub fn migrate_legacy_strategies(&mut self) {
        // If strategies array is not empty, assume already migrated
        if !self.strategies.is_empty() {
            return;
        }

        // Migrate from legacy `strategy` field
        if let Some(ref legacy_strategy) = self.strategy {
            let mut strategy = StrategyConfig {
                strategy_type: legacy_strategy.clone(),
                priority: StrategyConfig::default_priority_for_type(legacy_strategy),
                enabled: true,
                use_hostname_alias: false,
                directory: None,
                scope: None,
                patterns: None,
            };

            // Populate strategy-specific fields from legacy config
            match legacy_strategy.as_str() {
                "ssh" => {
                    strategy.use_hostname_alias = self
                        .ssh
                        .as_ref()
                        .map(|s| s.use_hostname_alias)
                        .unwrap_or(true);
                }
                "conditional" => {
                    strategy.directory = self.conditional.as_ref().and_then(|c| c.directory.clone());
                }
                "url" => {
                    strategy.scope = None; // Legacy url_rewrite didn't have scope
                    strategy.patterns = self
                        .url_rewrite
                        .as_ref()
                        .and_then(|u| u.patterns.clone());
                }
                _ => {}
            }

            self.strategies.push(strategy);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GtConfig::default();
        assert!(config.identities.is_empty());
        assert_eq!(config.ssh.key_type, "ed25519");
        assert_eq!(config.ssh.key_prefix, "id_gt_");
        assert_eq!(config.strategy.ssh_alias.prefix, "gt");
    }

    #[test]
    fn test_serialize_deserialize() {
        let mut config = GtConfig::default();
        config.defaults.identity = Some("work".to_string());
        config.identities.insert(
            "work".to_string(),
            IdentityConfig {
                email: "work@company.com".to_string(),
                name: "Work User".to_string(),
                provider: "github".to_string(),
                strategy: None,
                ssh: None,
                conditional: None,
                url_rewrite: None,
                strategies: vec![],
            },
        );

        let toml = toml::to_string_pretty(&config).unwrap();
        let parsed: GtConfig = toml::from_str(&toml).unwrap();

        assert_eq!(parsed.defaults.identity, Some("work".to_string()));
        assert!(parsed.identities.contains_key("work"));
    }
}
