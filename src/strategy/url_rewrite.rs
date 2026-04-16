//! URL rewriting strategy
//!
//! This strategy uses Git's url.<base>.insteadOf configuration
//! to transparently rewrite URLs.

use crate::core::identity::Identity;
use crate::core::repo::Repo;
use crate::error::Result;
use crate::strategy::{ApplyResult, SetupStep, Strategy, StrategyType, ValidationResult};

/// URL rewriting strategy
pub struct UrlRewriteStrategy {
    /// Scope for URL matching
    scope: UrlScope,
}

/// Scope for URL matching
#[derive(Debug, Clone, Copy)]
pub enum UrlScope {
    /// Match by organization/owner
    Organization,
    /// Match by user
    User,
    /// Match entire provider
    Provider,
}

impl UrlRewriteStrategy {
    /// Create a new URL rewrite strategy
    #[must_use]
    pub fn new() -> Self {
        Self {
            scope: UrlScope::Organization,
        }
    }

    /// Create with specific scope
    #[must_use]
    pub fn with_scope(scope: UrlScope) -> Self {
        Self { scope }
    }
}

impl Default for UrlRewriteStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl Strategy for UrlRewriteStrategy {
    fn strategy_type(&self) -> StrategyType {
        StrategyType::UrlRewrite
    }

    fn apply(&self, identity: &Identity, repo: &Repo) -> Result<ApplyResult> {
        let mut result = ApplyResult::new();

        // URL rewrite strategy doesn't modify repo URLs directly
        // Instead, it relies on global Git config to rewrite URLs transparently

        // Just set the local git config
        repo.set_config("user.email", &identity.email)?;
        repo.set_config("user.name", &identity.user_name)?;

        result.add_change(format!("Set user.email to {}", identity.email));
        result.add_change(format!("Set user.name to {}", identity.user_name));

        result.add_warning(
            "URL rewriting is configured globally. No URL changes made to this repository."
                .to_string(),
        );

        Ok(result)
    }

    fn remove(&self, _identity: &Identity, repo: &Repo) -> Result<()> {
        // Remove local config
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
        // Check if URL rewrite would apply to this repo's URL
        // This requires checking global Git config for insteadOf rules

        // For now, just check email
        let email = repo.get_config("user.email")?;
        Ok(email.as_ref() == Some(&identity.email))
    }

    fn validate(&self) -> Result<ValidationResult> {
        // URL rewrite strategy requires SSH config for key selection
        let errors = Vec::new();
        let warnings = Vec::new();

        Ok(ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    fn setup_requirements(&self) -> Vec<SetupStep> {
        vec![
            SetupStep {
                description: "SSH host alias configured".to_string(),
                complete: false, // Checked per-identity
            },
            SetupStep {
                description: "url.*.insteadOf rule added to ~/.gitconfig".to_string(),
                complete: false, // Checked per-identity
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_type() {
        let strategy = UrlRewriteStrategy::new();
        assert_eq!(strategy.strategy_type(), StrategyType::UrlRewrite);
    }
}
