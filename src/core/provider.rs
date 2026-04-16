//! Git provider definitions
//!
//! This module defines supported Git providers and their configurations.

use serde::{Deserialize, Serialize};

/// A Git provider (GitHub, GitLab, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    /// GitHub.com
    #[default]
    GitHub,
    /// GitLab.com
    GitLab,
    /// Bitbucket.org
    Bitbucket,
    /// Azure DevOps
    Azure,
    /// AWS CodeCommit
    CodeCommit,
    /// Custom/self-hosted provider
    Custom(CustomProvider),
}

/// Configuration for a custom provider
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomProvider {
    /// Provider name
    pub name: String,
    /// Hostname
    pub hostname: String,
    /// SSH user (usually "git")
    pub ssh_user: String,
}

impl Provider {
    /// Create a provider from a name string
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "github" | "github.com" => Provider::GitHub,
            "gitlab" | "gitlab.com" => Provider::GitLab,
            "bitbucket" | "bitbucket.org" => Provider::Bitbucket,
            "azure" | "dev.azure.com" => Provider::Azure,
            "codecommit" | "aws" => Provider::CodeCommit,
            _ => Provider::Custom(CustomProvider {
                name: name.to_string(),
                hostname: name.to_string(),
                ssh_user: "git".to_string(),
            }),
        }
    }

    /// Create a provider from a hostname
    #[must_use]
    pub fn from_hostname(hostname: &str) -> Option<Self> {
        // Remove any gitid prefix
        let clean_hostname = if hostname.contains(".") {
            let parts: Vec<&str> = hostname.split('.').collect();
            if parts.len() >= 2 && parts[0].starts_with("gitid-") {
                parts[1..].join(".")
            } else {
                hostname.to_string()
            }
        } else {
            hostname.to_string()
        };

        Some(match clean_hostname.as_str() {
            "github.com" => Provider::GitHub,
            "gitlab.com" => Provider::GitLab,
            "bitbucket.org" => Provider::Bitbucket,
            "dev.azure.com" | "ssh.dev.azure.com" => Provider::Azure,
            h if h.contains("codecommit") && h.contains("amazonaws.com") => Provider::CodeCommit,
            _ => return None,
        })
    }

    /// Get the hostname for this provider
    #[must_use]
    pub fn hostname(&self) -> &str {
        match self {
            Provider::GitHub => "github.com",
            Provider::GitLab => "gitlab.com",
            Provider::Bitbucket => "bitbucket.org",
            Provider::Azure => "dev.azure.com",
            Provider::CodeCommit => "git-codecommit.us-east-1.amazonaws.com",
            Provider::Custom(c) => &c.hostname,
        }
    }

    /// Get the SSH user for this provider
    #[must_use]
    pub fn ssh_user(&self) -> &str {
        match self {
            Provider::CodeCommit => "APKAEIBAERJR2EXAMPLE",
            Provider::Custom(c) => &c.ssh_user,
            _ => "git",
        }
    }

    /// Get the display name for this provider
    #[must_use]
    pub fn display_name(&self) -> &str {
        match self {
            Provider::GitHub => "GitHub",
            Provider::GitLab => "GitLab",
            Provider::Bitbucket => "Bitbucket",
            Provider::Azure => "Azure DevOps",
            Provider::CodeCommit => "AWS CodeCommit",
            Provider::Custom(c) => &c.name,
        }
    }

    /// Check if this is a known provider (not custom)
    #[must_use]
    pub fn is_known(&self) -> bool {
        !matches!(self, Provider::Custom(_))
    }
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_from_name() {
        assert_eq!(Provider::from_name("github"), Provider::GitHub);
        assert_eq!(Provider::from_name("GitHub"), Provider::GitHub);
        assert_eq!(Provider::from_name("gitlab"), Provider::GitLab);
        assert_eq!(Provider::from_name("bitbucket"), Provider::Bitbucket);
    }

    #[test]
    fn test_provider_from_hostname() {
        assert_eq!(
            Provider::from_hostname("github.com"),
            Some(Provider::GitHub)
        );
        assert_eq!(
            Provider::from_hostname("gitid-work.github.com"),
            Some(Provider::GitHub)
        );
        assert_eq!(Provider::from_hostname("unknown.com"), None);
    }

    #[test]
    fn test_provider_hostname() {
        assert_eq!(Provider::GitHub.hostname(), "github.com");
        assert_eq!(Provider::GitLab.hostname(), "gitlab.com");
    }
}
