//! Identity model and operations
//!
//! An identity represents a Git persona with associated credentials.

use serde::{Deserialize, Serialize};

use crate::core::provider::Provider;
use crate::error::{Error, Result};
use crate::util::validate_identity_name;

/// An SSH key configuration for an identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    /// Path to the SSH private key
    pub key_path: Option<String>,

    /// Key type (ed25519, rsa, ecdsa)
    pub key_type: Option<String>,

    /// Key bits (for RSA: 2048-4096, for ECDSA: 256/384/521)
    pub key_bits: Option<u32>,
}

/// A Git identity with credentials and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    /// Identity name (e.g., "work", "personal")
    pub name: String,

    /// Git user.email
    pub email: String,

    /// Git user.name
    pub user_name: String,

    /// Provider (e.g., "github", "gitlab")
    pub provider: Provider,

    /// SSH configuration
    pub ssh: Option<SshConfig>,

    /// Strategy override (if different from default)
    pub strategy: Option<String>,
}

impl Identity {
    /// Create a new identity builder
    #[must_use]
    pub fn builder(name: impl Into<String>) -> IdentityBuilder {
        IdentityBuilder::new(name)
    }

    /// Validate the identity
    pub fn validate(&self) -> Result<()> {
        validate_identity_name(&self.name)?;

        if self.email.is_empty() {
            return Err(Error::IdentityValidation {
                message: "email is required".to_string(),
            });
        }

        if !self.email.contains('@') {
            return Err(Error::IdentityValidation {
                message: "invalid email format".to_string(),
            });
        }

        if self.user_name.is_empty() {
            return Err(Error::IdentityValidation {
                message: "user_name is required".to_string(),
            });
        }

        Ok(())
    }

    /// Get the SSH key path, generating the default if not set
    #[must_use]
    pub fn ssh_key_path(&self) -> String {
        self.ssh
            .as_ref()
            .and_then(|s| s.key_path.clone())
            .unwrap_or_else(|| format!("~/.ssh/id_gt_{}", self.name))
    }

    /// Get the SSH host name for this identity
    #[must_use]
    pub fn ssh_host(&self) -> String {
        format!("gt-{}.{}", self.name, self.provider.hostname())
    }
}

/// Builder for creating identities
pub struct IdentityBuilder {
    name: String,
    email: Option<String>,
    user_name: Option<String>,
    provider: Option<Provider>,
    ssh: Option<SshConfig>,
    strategy: Option<String>,
}

impl IdentityBuilder {
    /// Create a new builder with the given name
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            email: None,
            user_name: None,
            provider: None,
            ssh: None,
            strategy: None,
        }
    }

    /// Set the email
    #[must_use]
    pub fn email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Set the user name
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.user_name = Some(name.into());
        self
    }

    /// Set the provider
    #[must_use]
    pub fn provider(mut self, provider: Provider) -> Self {
        self.provider = Some(provider);
        self
    }

    /// Set the provider from a string
    #[must_use]
    pub fn provider_str(mut self, provider: impl AsRef<str>) -> Self {
        self.provider = Some(Provider::from_name(provider.as_ref()));
        self
    }

    /// Set the SSH key path
    #[must_use]
    pub fn ssh_key(mut self, path: impl Into<String>) -> Self {
        self.ssh = Some(SshConfig {
            key_path: Some(path.into()),
            key_type: None,
            key_bits: None,
        });
        self
    }

    /// Set the full SSH configuration
    #[must_use]
    pub fn ssh_config(mut self, ssh: SshConfig) -> Self {
        self.ssh = Some(ssh);
        self
    }

    /// Set the strategy
    #[must_use]
    pub fn strategy(mut self, strategy: impl Into<String>) -> Self {
        self.strategy = Some(strategy.into());
        self
    }

    /// Build the identity
    pub fn build(self) -> Result<Identity> {
        let identity = Identity {
            name: self.name,
            email: self.email.ok_or(Error::IdentityValidation {
                message: "email is required".to_string(),
            })?,
            user_name: self.user_name.ok_or(Error::IdentityValidation {
                message: "user_name is required".to_string(),
            })?,
            provider: self.provider.unwrap_or_default(),
            ssh: self.ssh,
            strategy: self.strategy,
        };

        identity.validate()?;
        Ok(identity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_builder() {
        let identity = Identity::builder("work")
            .email("work@company.com")
            .name("Work User")
            .provider_str("github")
            .build()
            .unwrap();

        assert_eq!(identity.name, "work");
        assert_eq!(identity.email, "work@company.com");
        assert_eq!(identity.provider, Provider::GitHub);
    }

    #[test]
    fn test_identity_validation() {
        // Valid
        let result = Identity::builder("work")
            .email("work@company.com")
            .name("Work User")
            .build();
        assert!(result.is_ok());

        // Invalid name (contains reserved prefix)
        let result = Identity::builder("gt-work")
            .email("work@company.com")
            .name("Work User")
            .build();
        assert!(result.is_err());

        // Missing email
        let result = Identity::builder("work")
            .name("Work User")
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_ssh_host() {
        let identity = Identity::builder("work")
            .email("work@company.com")
            .name("Work User")
            .provider_str("github")
            .build()
            .unwrap();

        assert_eq!(identity.ssh_host(), "gt-work.github.com");
    }

    #[test]
    fn test_ssh_key_path_default() {
        let identity = Identity::builder("work")
            .email("work@company.com")
            .name("Work User")
            .build()
            .unwrap();

        assert_eq!(identity.ssh_key_path(), "~/.ssh/id_gt_work");
    }

    #[test]
    fn test_ssh_key_path_custom() {
        let identity = Identity::builder("work")
            .email("work@company.com")
            .name("Work User")
            .ssh_key("~/.ssh/custom_key")
            .build()
            .unwrap();

        assert_eq!(identity.ssh_key_path(), "~/.ssh/custom_key");
    }
}
