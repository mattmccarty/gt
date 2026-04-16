//! Strategy detection
//!
//! This module detects which identity strategy is in use.

use crate::core::provider::Provider;
use crate::error::Result;
use crate::io::{git_config, ssh_config};
use crate::strategy::StrategyType;

/// A detected identity configuration
#[derive(Debug, Clone)]
pub struct DetectedIdentity {
    /// Identity name
    pub name: String,
    /// Detected strategy
    pub strategy: StrategyType,
    /// Provider
    pub provider: Option<Provider>,
    /// Email (if found)
    pub email: Option<String>,
    /// SSH key path (if found)
    pub key_path: Option<String>,
    /// Source of detection
    pub source: DetectionSource,
    /// Whether this is a legacy gitid-* entry (vs current gt-*)
    pub is_legacy: bool,
}

/// Source of detection
#[derive(Debug, Clone)]
pub enum DetectionSource {
    /// SSH config entry
    SshConfig {
        /// Host pattern
        host: String,
    },
    /// Git conditional include
    GitConditional {
        /// Condition
        condition: String,
        /// Include path
        path: String,
    },
    /// Git URL rewrite
    GitUrlRewrite {
        /// Original pattern
        original: String,
        /// Replacement
        replacement: String,
    },
    /// Repository URL
    RepoUrl {
        /// Repository path
        path: String,
    },
}

/// Detect all identities from existing configuration
pub fn detect_identities() -> Result<Vec<DetectedIdentity>> {
    let mut identities = Vec::new();

    // Detect from SSH config
    identities.extend(detect_from_ssh_config()?);

    // Detect from Git conditional includes
    identities.extend(detect_from_git_conditionals()?);

    // Detect from URL rewrites
    identities.extend(detect_from_url_rewrites()?);

    Ok(identities)
}

/// Detect identities from SSH config
fn detect_from_ssh_config() -> Result<Vec<DetectedIdentity>> {
    let ssh_config_path = crate::core::path::ssh_config_path()?;

    if !ssh_config_path.exists() {
        return Ok(Vec::new());
    }

    let config = ssh_config::SshConfig::load(&ssh_config_path)?;
    let mut identities = Vec::new();

    // Detect current gt-* entries
    for host in config.find_gt_hosts("gt") {
        // Parse identity name from host pattern
        // Pattern: gt-{identity}.{provider}
        if let Some(name) = host.host.strip_prefix("gt-") {
            let parts: Vec<&str> = name.splitn(2, '.').collect();
            if parts.len() == 2 {
                let identity_name = parts[0].to_string();
                let provider_host = parts[1];

                identities.push(DetectedIdentity {
                    name: identity_name,
                    strategy: StrategyType::SshAlias,
                    provider: Provider::from_hostname(provider_host),
                    email: None,
                    key_path: host.identity_file.clone(),
                    source: DetectionSource::SshConfig {
                        host: host.host.clone(),
                    },
                    is_legacy: false,
                });
            }
        }
    }

    // Detect legacy gitid-* entries (from old gitid tool)
    for host in config.find_gt_hosts("gitid") {
        // Parse identity name from host pattern
        // Pattern: gitid-{identity}.{provider}
        if let Some(name) = host.host.strip_prefix("gitid-") {
            let parts: Vec<&str> = name.splitn(2, '.').collect();
            if parts.len() == 2 {
                let identity_name = parts[0].to_string();
                let provider_host = parts[1];

                identities.push(DetectedIdentity {
                    name: identity_name,
                    strategy: StrategyType::SshAlias,
                    provider: Provider::from_hostname(provider_host),
                    email: None,
                    key_path: host.identity_file.clone(),
                    source: DetectionSource::SshConfig {
                        host: host.host.clone(),
                    },
                    is_legacy: true,
                });
            }
        }
    }

    // Detect generic Git provider entries (e.g., github.com, bitbucket.org)
    for host in config.find_git_provider_hosts() {
        // Skip if already detected as gt-* or gitid-* entry
        if host.host.starts_with("gt-") || host.host.starts_with("gitid-") {
            continue;
        }

        // Extract identity name from the host pattern
        // For "shastic.bitbucket.org" -> "shastic"
        // For "github.com" -> "github.com" (default)
        let identity_name = if host.host.contains('.') {
            // Get first part before the provider domain
            let parts: Vec<&str> = host.host.splitn(2, '.').collect();
            parts[0].to_string()
        } else {
            host.host.clone()
        };

        // Determine provider from hostname or host
        let provider = if let Some(ref hostname) = host.hostname {
            Provider::from_hostname(hostname)
        } else {
            Provider::from_hostname(&host.host)
        };

        identities.push(DetectedIdentity {
            name: identity_name,
            strategy: StrategyType::SshAlias,
            provider,
            email: None,
            key_path: host.identity_file.clone(),
            source: DetectionSource::SshConfig {
                host: host.host.clone(),
            },
            is_legacy: false,
        });
    }

    Ok(identities)
}

/// Detect identities from Git conditional includes
fn detect_from_git_conditionals() -> Result<Vec<DetectedIdentity>> {
    let includes = git_config::find_conditional_includes()?;
    let mut identities = Vec::new();

    for include in includes {
        // Try to extract identity name from condition or path
        // Condition format: gitdir:~/work/
        let name = include
            .path
            .rsplit('/')
            .next()
            .and_then(|f| f.strip_suffix(".gitconfig"))
            .unwrap_or(&include.condition);

        // Try to read the include file for email
        let email = None; // TODO: Parse include file

        identities.push(DetectedIdentity {
            name: name.to_string(),
            strategy: StrategyType::Conditional,
            provider: None,
            email,
            key_path: None,
            source: DetectionSource::GitConditional {
                condition: include.condition,
                path: include.path,
            },
            is_legacy: false,
        });
    }

    Ok(identities)
}

/// Detect identities from URL rewrites
fn detect_from_url_rewrites() -> Result<Vec<DetectedIdentity>> {
    let rewrites = git_config::find_url_rewrites()?;
    let mut identities = Vec::new();

    for (original, replacement) in rewrites {
        // Try to extract identity from replacement
        // Pattern: git@{identity}-{provider}:
        let name = replacement
            .strip_prefix("git@")
            .and_then(|s| s.strip_suffix(':'))
            .unwrap_or(&replacement);

        identities.push(DetectedIdentity {
            name: name.to_string(),
            strategy: StrategyType::UrlRewrite,
            provider: None,
            email: None,
            key_path: None,
            source: DetectionSource::GitUrlRewrite {
                original,
                replacement,
            },
            is_legacy: false,
        });
    }

    Ok(identities)
}

/// Detect the strategy used for a specific repository
pub fn detect_repo_strategy(repo_path: &std::path::Path) -> Result<Option<StrategyType>> {
    use crate::core::repo::Repo;

    let repo = Repo::detect(Some(&repo_path.to_owned()))?;

    // Check for SSH alias (modified URL)
    if repo.is_url_modified() {
        return Ok(Some(StrategyType::SshAlias));
    }

    // Check for conditional (directory match)
    let includes = git_config::find_conditional_includes()?;
    for include in includes {
        if include.condition.starts_with("gitdir:") {
            let dir = include
                .condition
                .strip_prefix("gitdir:")
                .unwrap_or(&include.condition);
            let expanded = crate::util::expand_path(std::path::Path::new(dir))?;

            if repo_path.starts_with(&expanded) {
                return Ok(Some(StrategyType::Conditional));
            }
        }
    }

    // Check for URL rewrite
    let rewrites = git_config::find_url_rewrites()?;
    if let Some(url) = &repo.remote_url {
        for (original, _) in rewrites {
            if url.contains(&original) {
                return Ok(Some(StrategyType::UrlRewrite));
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detection_source() {
        let source = DetectionSource::SshConfig {
            host: "gitid-work.github.com".to_string(),
        };

        match source {
            DetectionSource::SshConfig { host } => {
                assert!(host.contains("gitid-work"));
            }
            _ => panic!("Wrong source type"),
        }
    }
}
