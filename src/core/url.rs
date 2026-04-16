//! Git URL parsing and transformation
//!
//! This module handles parsing Git URLs and transforming them
//! for different identity strategies.

use regex::Regex;

use crate::core::provider::Provider;
use crate::error::{Error, Result};

/// A parsed Git URL
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitUrl {
    /// Protocol (ssh, https, git)
    pub protocol: Protocol,
    /// User (usually "git")
    pub user: Option<String>,
    /// Host (e.g., "github.com" or "gitid-work.github.com")
    pub host: String,
    /// Port (if specified)
    pub port: Option<u16>,
    /// Path (e.g., "owner/repo.git")
    pub path: String,
    /// Detected provider
    pub provider: Option<Provider>,
    /// Detected identity (from modified URL)
    pub identity: Option<String>,
}

/// URL protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// SSH (git@host:path)
    Ssh,
    /// HTTPS (https://host/path)
    Https,
    /// Git protocol (git://host/path)
    Git,
}

impl GitUrl {
    /// Parse a Git URL string
    pub fn parse(url: &str) -> Result<Self> {
        // SSH format: git@host:path or user@host:path
        let ssh_re = Regex::new(r"^([^@]+)@([^:]+):(.+)$").unwrap();
        if let Some(caps) = ssh_re.captures(url) {
            let user = caps.get(1).map(|m| m.as_str().to_string());
            let host = caps.get(2).unwrap().as_str().to_string();
            let path = caps.get(3).unwrap().as_str().to_string();

            let (provider, identity) = Self::extract_provider_and_identity(&host);

            return Ok(GitUrl {
                protocol: Protocol::Ssh,
                user,
                host,
                port: None,
                path,
                provider,
                identity,
            });
        }

        // HTTPS format: https://host/path
        let https_re = Regex::new(r"^https://([^/]+)/(.+)$").unwrap();
        if let Some(caps) = https_re.captures(url) {
            let host = caps.get(1).unwrap().as_str().to_string();
            let path = caps.get(2).unwrap().as_str().to_string();

            let (provider, identity) = Self::extract_provider_and_identity(&host);

            return Ok(GitUrl {
                protocol: Protocol::Https,
                user: None,
                host,
                port: None,
                path,
                provider,
                identity,
            });
        }

        // Git protocol: git://host/path
        let git_re = Regex::new(r"^git://([^/]+)/(.+)$").unwrap();
        if let Some(caps) = git_re.captures(url) {
            let host = caps.get(1).unwrap().as_str().to_string();
            let path = caps.get(2).unwrap().as_str().to_string();

            let (provider, identity) = Self::extract_provider_and_identity(&host);

            return Ok(GitUrl {
                protocol: Protocol::Git,
                user: None,
                host,
                port: None,
                path,
                provider,
                identity,
            });
        }

        Err(Error::UrlUnrecognized {
            url: url.to_string(),
        })
    }

    /// Extract provider and identity from a potentially modified hostname
    fn extract_provider_and_identity(host: &str) -> (Option<Provider>, Option<String>) {
        // Pattern: gitid-{identity}.{provider}
        let gitid_re = Regex::new(r"^gitid-([^.]+)\.(.+)$").unwrap();

        if let Some(caps) = gitid_re.captures(host) {
            let identity = caps.get(1).map(|m| m.as_str().to_string());
            let provider_host = caps.get(2).map(|m| m.as_str()).unwrap_or(host);
            let provider = Provider::from_hostname(provider_host);
            return (provider, identity);
        }

        // Standard hostname
        let provider = Provider::from_hostname(host);
        (provider, None)
    }

    /// Transform this URL to use a specific identity (SSH alias strategy)
    pub fn with_identity(&self, identity: &str) -> Result<Self> {
        let provider = self.provider.as_ref().ok_or(Error::ProviderUnknown {
            hostname: self.host.clone(),
        })?;

        let new_host = format!("gitid-{}.{}", identity, provider.hostname());

        Ok(GitUrl {
            protocol: Protocol::Ssh,
            user: Some("git".to_string()),
            host: new_host,
            port: self.port,
            path: self.path.clone(),
            provider: self.provider.clone(),
            identity: Some(identity.to_string()),
        })
    }

    /// Restore this URL to the original (non-identity) format
    pub fn without_identity(&self) -> Result<Self> {
        let provider = self.provider.as_ref().ok_or(Error::ProviderUnknown {
            hostname: self.host.clone(),
        })?;

        Ok(GitUrl {
            protocol: Protocol::Ssh,
            user: Some("git".to_string()),
            host: provider.hostname().to_string(),
            port: self.port,
            path: self.path.clone(),
            provider: self.provider.clone(),
            identity: None,
        })
    }

    /// Convert to string representation
    #[must_use]
    pub fn to_string(&self) -> String {
        match self.protocol {
            Protocol::Ssh => {
                let user = self.user.as_deref().unwrap_or("git");
                format!("{}@{}:{}", user, self.host, self.path)
            }
            Protocol::Https => {
                format!("https://{}/{}", self.host, self.path)
            }
            Protocol::Git => {
                format!("git://{}/{}", self.host, self.path)
            }
        }
    }

    /// Check if this URL has been modified with a gitid identity
    #[must_use]
    pub fn is_modified(&self) -> bool {
        self.identity.is_some()
    }

    /// Get the original provider hostname (without gitid prefix)
    #[must_use]
    pub fn original_host(&self) -> Option<&str> {
        self.provider.as_ref().map(Provider::hostname)
    }
}

/// Transform a URL string to use a specific identity
pub fn transform_url(url: &str, identity: &str) -> Result<String> {
    let parsed = GitUrl::parse(url)?;
    let transformed = parsed.with_identity(identity)?;
    Ok(transformed.to_string())
}

/// Restore a URL string to its original format
pub fn restore_url(url: &str) -> Result<String> {
    let parsed = GitUrl::parse(url)?;
    let restored = parsed.without_identity()?;
    Ok(restored.to_string())
}

/// Detect the identity from a potentially modified URL
pub fn detect_identity(url: &str) -> Result<Option<String>> {
    let parsed = GitUrl::parse(url)?;
    Ok(parsed.identity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ssh_url() {
        let url = GitUrl::parse("git@github.com:owner/repo.git").unwrap();
        assert_eq!(url.protocol, Protocol::Ssh);
        assert_eq!(url.host, "github.com");
        assert_eq!(url.path, "owner/repo.git");
        assert_eq!(url.provider, Some(Provider::GitHub));
        assert_eq!(url.identity, None);
    }

    #[test]
    fn test_parse_modified_url() {
        let url = GitUrl::parse("git@gitid-work.github.com:owner/repo.git").unwrap();
        assert_eq!(url.protocol, Protocol::Ssh);
        assert_eq!(url.host, "gitid-work.github.com");
        assert_eq!(url.provider, Some(Provider::GitHub));
        assert_eq!(url.identity, Some("work".to_string()));
    }

    #[test]
    fn test_transform_url() {
        let result = transform_url("git@github.com:owner/repo.git", "work").unwrap();
        assert_eq!(result, "git@gitid-work.github.com:owner/repo.git");
    }

    #[test]
    fn test_restore_url() {
        let result = restore_url("git@gitid-work.github.com:owner/repo.git").unwrap();
        assert_eq!(result, "git@github.com:owner/repo.git");
    }

    #[test]
    fn test_detect_identity() {
        let result = detect_identity("git@gitid-work.github.com:owner/repo.git").unwrap();
        assert_eq!(result, Some("work".to_string()));

        let result = detect_identity("git@github.com:owner/repo.git").unwrap();
        assert_eq!(result, None);
    }
}
