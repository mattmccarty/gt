//! SSH configuration scanner
//!
//! Scans SSH config for identity-related entries.

use crate::core::path::ssh_config_path;
use crate::error::Result;
use crate::io::ssh_config::SshConfig;

/// Scan results for SSH config
#[derive(Debug, Default)]
pub struct SshScanResult {
    /// Found host entries
    pub hosts: Vec<SshHostInfo>,
    /// SSH keys found
    pub keys: Vec<SshKeyInfo>,
    /// Warnings
    pub warnings: Vec<String>,
}

/// Information about an SSH host entry
#[derive(Debug)]
pub struct SshHostInfo {
    /// Host pattern
    pub host: String,
    /// Whether this looks like a gitid entry
    pub is_gitid: bool,
    /// Extracted identity name (if gitid)
    pub identity: Option<String>,
    /// Provider hostname
    pub provider_host: Option<String>,
    /// Key path
    pub key_path: Option<String>,
}

/// Information about an SSH key
#[derive(Debug)]
pub struct SshKeyInfo {
    /// Key path
    pub path: String,
    /// Key type (ed25519, rsa, etc.)
    pub key_type: Option<String>,
    /// Whether this key is referenced in SSH config
    pub in_config: bool,
    /// Whether this looks like a gitid key
    pub is_gitid: bool,
    /// Extracted identity name
    pub identity: Option<String>,
}

/// Scan SSH configuration
pub fn scan_ssh_config() -> Result<SshScanResult> {
    let mut result = SshScanResult::default();

    let config_path = ssh_config_path()?;
    if !config_path.exists() {
        result
            .warnings
            .push("SSH config file does not exist".to_string());
        return Ok(result);
    }

    let config = SshConfig::load(&config_path)?;

    for host in &config.hosts {
        let is_gitid = host.host.starts_with("gitid-");

        let (identity, provider_host) = if is_gitid {
            let rest = host.host.strip_prefix("gitid-").unwrap_or(&host.host);
            let parts: Vec<&str> = rest.splitn(2, '.').collect();
            if parts.len() == 2 {
                (Some(parts[0].to_string()), Some(parts[1].to_string()))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        result.hosts.push(SshHostInfo {
            host: host.host.clone(),
            is_gitid,
            identity,
            provider_host,
            key_path: host.identity_file.clone(),
        });
    }

    // Scan SSH directory for keys
    let ssh_dir = crate::core::path::ssh_dir()?;
    if ssh_dir.exists() {
        let referenced_keys: Vec<_> = result
            .hosts
            .iter()
            .filter_map(|h| h.key_path.as_ref())
            .collect();

        for entry in std::fs::read_dir(&ssh_dir)? {
            let entry = entry?;
            let path = entry.path();

            // Skip non-files and public keys
            if !path.is_file() {
                continue;
            }
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.ends_with(".pub") || name == "config" || name == "known_hosts" {
                continue;
            }

            // Check if it's a key file
            if !name.starts_with("id_") {
                continue;
            }

            let path_str = path.to_string_lossy().to_string();
            let in_config = referenced_keys.iter().any(|k| path_str.contains(*k));
            let is_gitid = name.contains("gitid");
            let identity = if is_gitid {
                name.strip_prefix("id_gitid_").map(String::from)
            } else {
                None
            };

            // Detect key type from filename or content
            let key_type = if name.contains("ed25519") {
                Some("ed25519".to_string())
            } else if name.contains("rsa") {
                Some("rsa".to_string())
            } else if name.contains("ecdsa") {
                Some("ecdsa".to_string())
            } else {
                None
            };

            result.keys.push(SshKeyInfo {
                path: path_str,
                key_type,
                in_config,
                is_gitid,
                identity,
            });
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_result_default() {
        let result = SshScanResult::default();
        assert!(result.hosts.is_empty());
        assert!(result.keys.is_empty());
    }
}
