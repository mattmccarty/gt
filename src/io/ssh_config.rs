//! SSH config parsing and writing
//!
//! This module handles parsing and modifying ~/.ssh/config files.

use std::collections::HashMap;
use std::path::Path;

use crate::error::Result;

/// A warning encountered during SSH config parsing
#[derive(Debug, Clone)]
pub struct ParseWarning {
    /// Line number where the issue occurred (1-indexed)
    pub line_number: usize,
    /// The directive that caused the warning
    pub directive: String,
    /// Description of the issue
    pub message: String,
}

impl ParseWarning {
    /// Create a new parse warning
    pub fn new(line_number: usize, directive: String, message: String) -> Self {
        Self {
            line_number,
            directive,
            message,
        }
    }
}

/// A Host entry in SSH config
#[derive(Debug, Clone)]
pub struct SshHostEntry {
    /// Host pattern (e.g., "gitid-work.github.com")
    pub host: String,
    /// HostName directive
    pub hostname: Option<String>,
    /// User directive
    pub user: Option<String>,
    /// IdentityFile directive
    pub identity_file: Option<String>,
    /// IdentitiesOnly directive
    pub identities_only: Option<bool>,
    /// PreferredAuthentications directive
    pub preferred_auth: Option<String>,
    /// Other directives
    pub other: HashMap<String, String>,
}

impl SshHostEntry {
    /// Create a new host entry
    #[must_use]
    pub fn new(host: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            hostname: None,
            user: None,
            identity_file: None,
            identities_only: None,
            preferred_auth: None,
            other: HashMap::new(),
        }
    }

    /// Set HostName
    #[must_use]
    pub fn with_hostname(mut self, hostname: impl Into<String>) -> Self {
        self.hostname = Some(hostname.into());
        self
    }

    /// Set User
    #[must_use]
    pub fn with_user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Set IdentityFile
    #[must_use]
    pub fn with_identity_file(mut self, path: impl Into<String>) -> Self {
        self.identity_file = Some(path.into());
        self
    }

    /// Set IdentitiesOnly
    #[must_use]
    pub fn with_identities_only(mut self, value: bool) -> Self {
        self.identities_only = Some(value);
        self
    }

    /// Set PreferredAuthentications
    #[must_use]
    pub fn with_preferred_auth(mut self, auth: impl Into<String>) -> Self {
        self.preferred_auth = Some(auth.into());
        self
    }

    /// Convert to SSH config format
    #[must_use]
    pub fn to_string(&self, indent: &str) -> String {
        let mut lines = vec![format!("Host {}", self.host)];

        if let Some(ref hostname) = self.hostname {
            lines.push(format!("{}HostName {}", indent, hostname));
        }
        if let Some(ref user) = self.user {
            lines.push(format!("{}User {}", indent, user));
        }
        if let Some(ref identity_file) = self.identity_file {
            lines.push(format!("{}IdentityFile {}", indent, identity_file));
        }
        if let Some(identities_only) = self.identities_only {
            let value = if identities_only { "yes" } else { "no" };
            lines.push(format!("{}IdentitiesOnly {}", indent, value));
        }
        if let Some(ref preferred_auth) = self.preferred_auth {
            lines.push(format!(
                "{}PreferredAuthentications {}",
                indent, preferred_auth
            ));
        }
        for (key, value) in &self.other {
            lines.push(format!("{}{} {}", indent, key, value));
        }

        lines.join("\n")
    }
}

/// Parsed SSH config file
#[derive(Debug, Clone, Default)]
pub struct SshConfig {
    /// Host entries
    pub hosts: Vec<SshHostEntry>,
    /// Global directives (before first Host)
    pub global: HashMap<String, String>,
    /// Parse warnings (malformed entries, suspicious directives, etc.)
    pub warnings: Vec<ParseWarning>,
    /// Raw content for sections we don't parse
    raw_sections: Vec<String>,
}

impl SshConfig {
    /// Parse SSH config from a string
    pub fn parse(content: &str) -> Result<Self> {
        let mut config = SshConfig::default();
        let mut current_host: Option<SshHostEntry> = None;

        for (line_idx, line) in content.lines().enumerate() {
            let line_number = line_idx + 1; // 1-indexed for user-friendly messages
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Parse directive
            let parts: Vec<&str> = trimmed.splitn(2, |c: char| c.is_whitespace()).collect();
            if parts.len() != 2 {
                continue;
            }

            let key = parts[0];
            let value = parts[1].trim();

            if key.eq_ignore_ascii_case("Host") {
                // Save previous host
                if let Some(host) = current_host.take() {
                    config.hosts.push(host);
                }

                // Start new host
                current_host = Some(SshHostEntry::new(value));
            } else if let Some(ref mut host) = current_host {
                // Add directive to current host
                match key.to_lowercase().as_str() {
                    "hostname" => host.hostname = Some(value.to_string()),
                    "user" => host.user = Some(value.to_string()),
                    "identityfile" => host.identity_file = Some(value.to_string()),
                    "identitiesonly" => {
                        host.identities_only = Some(value.eq_ignore_ascii_case("yes"))
                    }
                    "preferredauthentications" => host.preferred_auth = Some(value.to_string()),
                    _ => {
                        host.other.insert(key.to_string(), value.to_string());
                    }
                }
            } else {
                // Directive outside of any Host block - could be global or corruption
                let key_lower = key.to_lowercase();

                // These directives should almost always be under a Host block
                // If found outside, it's likely a corrupted config file
                let host_specific_directives = [
                    "hostname",
                    "identityfile",
                    "identitiesonly",
                    "user",
                    "port",
                    "proxycommand",
                    "localforward",
                    "remoteforward",
                ];

                if host_specific_directives.contains(&key_lower.as_str()) {
                    config.warnings.push(ParseWarning::new(
                        line_number,
                        key.to_string(),
                        format!(
                            "Host-specific directive '{}' found outside of any Host block. \
                            This is likely a corrupted SSH config. Expected format:\n\
                            Host <hostname>\n    {} {}",
                            key, key, value
                        ),
                    ));
                }

                // Still add as global directive (for backwards compatibility)
                config.global.insert(key.to_string(), value.to_string());
            }
        }

        // Save last host
        if let Some(host) = current_host {
            config.hosts.push(host);
        }

        Ok(config)
    }

    /// Load SSH config from a file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Save SSH config to a file
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = self.to_string();
        std::fs::write(path, content)?;

        // Set secure permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(path, perms)?;
        }

        Ok(())
    }

    /// Check if there are any parse warnings
    #[must_use]
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Get all parse warnings
    #[must_use]
    pub fn get_warnings(&self) -> &[ParseWarning] {
        &self.warnings
    }

    /// Check if a host entry exists
    #[must_use]
    pub fn has_host(&self, host: &str) -> bool {
        self.hosts.iter().any(|h| h.host == host)
    }

    /// Get a host entry
    #[must_use]
    pub fn get_host(&self, host: &str) -> Option<&SshHostEntry> {
        self.hosts.iter().find(|h| h.host == host)
    }

    /// Add or update a host entry
    pub fn upsert_host(&mut self, entry: SshHostEntry) {
        if let Some(existing) = self.hosts.iter_mut().find(|h| h.host == entry.host) {
            *existing = entry;
        } else {
            self.hosts.push(entry);
        }
    }

    /// Remove a host entry
    pub fn remove_host(&mut self, host: &str) -> Option<SshHostEntry> {
        if let Some(pos) = self.hosts.iter().position(|h| h.host == host) {
            Some(self.hosts.remove(pos))
        } else {
            None
        }
    }

    /// Convert to SSH config format
    #[must_use]
    pub fn to_string(&self) -> String {
        let mut lines = Vec::new();
        let indent = "    ";

        // Global directives
        for (key, value) in &self.global {
            lines.push(format!("{} {}", key, value));
        }

        if !self.global.is_empty() && !self.hosts.is_empty() {
            lines.push(String::new());
        }

        // Host entries
        for (i, host) in self.hosts.iter().enumerate() {
            if i > 0 {
                lines.push(String::new());
            }
            lines.push(host.to_string(indent));
        }

        lines.join("\n") + "\n"
    }

    /// Find all gt-related host entries
    pub fn find_gt_hosts(&self, prefix: &str) -> Vec<&SshHostEntry> {
        self.hosts
            .iter()
            .filter(|h| h.host.starts_with(&format!("{}-", prefix)))
            .collect()
    }

    /// Find all hosts that point to known Git providers
    pub fn find_git_provider_hosts(&self) -> Vec<&SshHostEntry> {
        const GIT_PROVIDERS: &[&str] = &[
            "github.com",
            "gitlab.com",
            "bitbucket.org",
            "gitea.com",
            "codeberg.org",
        ];

        self.hosts
            .iter()
            .filter(|h| {
                // Skip wildcard hosts
                if h.host == "*" || h.host.contains('*') {
                    return false;
                }

                // Check if the hostname or host pattern matches a known provider
                if let Some(ref hostname) = h.hostname {
                    GIT_PROVIDERS.iter().any(|provider| hostname == provider)
                } else {
                    // If no explicit HostName, check if the Host itself is a provider
                    GIT_PROVIDERS
                        .iter()
                        .any(|provider| h.host.contains(provider))
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ssh_config() {
        let content = r#"
Host *
    AddKeysToAgent yes

Host gt-work.github.com
    HostName github.com
    User git
    IdentityFile ~/.ssh/id_gt_work
    IdentitiesOnly yes
"#;

        let config = SshConfig::parse(content).unwrap();
        assert_eq!(config.hosts.len(), 2);

        let work = config.get_host("gt-work.github.com").unwrap();
        assert_eq!(work.hostname, Some("github.com".to_string()));
        assert_eq!(work.user, Some("git".to_string()));
        assert_eq!(work.identities_only, Some(true));
    }

    #[test]
    fn test_upsert_host() {
        let mut config = SshConfig::default();

        let entry = SshHostEntry::new("test.example.com")
            .with_hostname("example.com")
            .with_user("testuser");

        config.upsert_host(entry);
        assert_eq!(config.hosts.len(), 1);

        // Update
        let entry2 = SshHostEntry::new("test.example.com")
            .with_hostname("example.org")
            .with_user("newuser");

        config.upsert_host(entry2);
        assert_eq!(config.hosts.len(), 1);
        assert_eq!(
            config.get_host("test.example.com").unwrap().hostname,
            Some("example.org".to_string())
        );
    }

    #[test]
    fn test_to_string() {
        let mut config = SshConfig::default();

        let entry = SshHostEntry::new("gt-work.github.com")
            .with_hostname("github.com")
            .with_user("git")
            .with_identity_file("~/.ssh/id_gt_work")
            .with_identities_only(true);

        config.upsert_host(entry);

        let output = config.to_string();
        assert!(output.contains("Host gt-work.github.com"));
        assert!(output.contains("HostName github.com"));
        assert!(output.contains("IdentitiesOnly yes"));
    }

    #[test]
    fn test_find_gt_hosts() {
        let mut config = SshConfig::default();

        config.upsert_host(SshHostEntry::new("gt-work.github.com").with_hostname("github.com"));
        config.upsert_host(SshHostEntry::new("gt-personal.github.com").with_hostname("github.com"));
        config.upsert_host(SshHostEntry::new("other.example.com").with_hostname("example.com"));

        let gt_hosts = config.find_gt_hosts("gt");
        assert_eq!(gt_hosts.len(), 2);
    }

    #[test]
    fn test_remove_host() {
        let mut config = SshConfig::default();

        config.upsert_host(SshHostEntry::new("test.example.com").with_hostname("example.com"));
        assert_eq!(config.hosts.len(), 1);

        let removed = config.remove_host("test.example.com");
        assert!(removed.is_some());
        assert_eq!(config.hosts.len(), 0);

        let removed2 = config.remove_host("nonexistent.com");
        assert!(removed2.is_none());
    }

    #[test]
    fn test_parse_empty_config() {
        let config = SshConfig::parse("").unwrap();
        assert_eq!(config.hosts.len(), 0);
        assert!(config.global.is_empty());
    }

    #[test]
    fn test_parse_comments_and_empty_lines() {
        let content = r#"
# This is a comment

Host test.com
    # Another comment
    HostName example.com

    User testuser
"#;

        let config = SshConfig::parse(content).unwrap();
        assert_eq!(config.hosts.len(), 1);

        let host = config.get_host("test.com").unwrap();
        assert_eq!(host.hostname, Some("example.com".to_string()));
        assert_eq!(host.user, Some("testuser".to_string()));
    }
}
