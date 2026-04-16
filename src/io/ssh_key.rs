//! SSH key generation and management
//!
//! This module handles generating SSH keys and managing them.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{Error, Result};

/// SSH key type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    /// Ed25519 (recommended - modern, secure, fast)
    Ed25519,
    /// RSA (traditional, widely supported)
    Rsa,
    /// ECDSA (elliptic curve, good balance)
    Ecdsa,
}

impl KeyType {
    /// Get the ssh-keygen type argument
    #[must_use]
    pub fn as_arg(&self) -> &str {
        match self {
            KeyType::Ed25519 => "ed25519",
            KeyType::Rsa => "rsa",
            KeyType::Ecdsa => "ecdsa",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "ed25519" => Some(KeyType::Ed25519),
            "rsa" => Some(KeyType::Rsa),
            "ecdsa" => Some(KeyType::Ecdsa),
            _ => None,
        }
    }

    /// Get default key type (Ed25519)
    #[must_use]
    pub fn default() -> Self {
        KeyType::Ed25519
    }

    /// Get recommended bits for key type
    #[must_use]
    pub fn default_bits(&self) -> Option<u32> {
        match self {
            KeyType::Rsa => Some(4096),
            KeyType::Ecdsa => Some(521), // ECDSA P-521
            KeyType::Ed25519 => None,     // Ed25519 has fixed key size
        }
    }
}

impl std::fmt::Display for KeyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_arg())
    }
}

/// Options for key generation
pub struct KeyGenOptions {
    /// Key type
    pub key_type: KeyType,
    /// RSA bits (only for RSA)
    pub bits: Option<u32>,
    /// Key comment
    pub comment: String,
    /// Output path (private key)
    pub path: PathBuf,
    /// Overwrite existing key
    pub force: bool,
}

impl KeyGenOptions {
    /// Create new options for Ed25519 key
    #[must_use]
    pub fn ed25519(path: PathBuf, comment: impl Into<String>) -> Self {
        Self {
            key_type: KeyType::Ed25519,
            bits: None,
            comment: comment.into(),
            path,
            force: false,
        }
    }

    /// Create new options for RSA key
    #[must_use]
    pub fn rsa(path: PathBuf, comment: impl Into<String>, bits: u32) -> Self {
        Self {
            key_type: KeyType::Rsa,
            bits: Some(bits),
            comment: comment.into(),
            path,
            force: false,
        }
    }

    /// Create new options for ECDSA key
    #[must_use]
    pub fn ecdsa(path: PathBuf, comment: impl Into<String>, bits: u32) -> Self {
        Self {
            key_type: KeyType::Ecdsa,
            bits: Some(bits),
            comment: comment.into(),
            path,
            force: false,
        }
    }

    /// Set force flag
    #[must_use]
    pub fn force(mut self) -> Self {
        self.force = true;
        self
    }
}

/// Generate an SSH key
pub fn generate_key(opts: &KeyGenOptions) -> Result<PathBuf> {
    // Check if key already exists
    if opts.path.exists() && !opts.force {
        return Err(Error::SshKeyGeneration {
            message: format!(
                "Key already exists at {}. Use --force to overwrite.",
                opts.path.display()
            ),
        });
    }

    // Ensure parent directory exists
    if let Some(parent) = opts.path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Build ssh-keygen command
    let mut cmd = Command::new("ssh-keygen");
    cmd.args(["-t", opts.key_type.as_arg()]);
    cmd.args(["-C", &opts.comment]);
    cmd.args(["-f", &opts.path.to_string_lossy()]);
    cmd.args(["-N", ""]); // Empty passphrase

    // Add -q for quiet mode (suppress output)
    cmd.arg("-q");

    // Set bits for RSA or ECDSA
    if let Some(bits) = opts.bits {
        match opts.key_type {
            KeyType::Rsa | KeyType::Ecdsa => {
                cmd.args(["-b", &bits.to_string()]);
            }
            KeyType::Ed25519 => {
                // Ed25519 has fixed key size, ignore bits
            }
        }
    }

    // NOTE: ssh-keygen will fail if key exists, unless we delete it first
    // We handle this by checking existence above and failing early if !force

    let output = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            Error::ToolNotFound {
                tool: "ssh-keygen".to_string(),
            }
        } else {
            Error::SshKeyGeneration {
                message: e.to_string(),
            }
        }
    })?;

    if !output.status.success() {
        return Err(Error::SshKeyGeneration {
            message: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    // Set secure permissions
    set_key_permissions(&opts.path)?;

    Ok(opts.path.clone())
}

/// Set secure permissions on a key file
pub fn set_key_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, perms)?;
    }

    #[cfg(windows)]
    {
        // Windows permissions are handled via ACLs
        // The key file is created with appropriate permissions by ssh-keygen
    }

    Ok(())
}

/// Check if a key file exists and is valid
pub fn verify_key(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }

    // Try to read the public key
    let pub_path = path.with_extension("pub");
    if !pub_path.exists() {
        return Ok(false);
    }

    // Verify key format using ssh-keygen
    let output = Command::new("ssh-keygen")
        .args(["-l", "-f"])
        .arg(path)
        .output()
        .map_err(|e| Error::SshKeyGeneration {
            message: e.to_string(),
        })?;

    Ok(output.status.success())
}

/// Read the public key content
pub fn read_public_key(path: &Path) -> Result<String> {
    let pub_path = if path.extension().map_or(false, |e| e == "pub") {
        path.to_owned()
    } else {
        path.with_extension("pub")
    };

    if !pub_path.exists() {
        return Err(Error::SshKeyNotFound {
            path: pub_path,
            identity: "unknown".to_string(),
        });
    }

    let content = std::fs::read_to_string(&pub_path)?;
    Ok(content.trim().to_string())
}

/// Add a key to the SSH agent
pub fn add_to_agent(path: &Path) -> Result<()> {
    let output = Command::new("ssh-add")
        .arg(path)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::ToolNotFound {
                    tool: "ssh-add".to_string(),
                }
            } else {
                Error::SshAgent {
                    message: e.to_string(),
                }
            }
        })?;

    if !output.status.success() {
        return Err(Error::SshAgent {
            message: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    Ok(())
}

/// List keys in the SSH agent
pub fn list_agent_keys() -> Result<Vec<String>> {
    let output = Command::new("ssh-add")
        .arg("-l")
        .output()
        .map_err(|e| Error::SshAgent {
            message: e.to_string(),
        })?;

    if !output.status.success() {
        // Exit code 1 means no keys, which is fine
        if output.status.code() == Some(1) {
            return Ok(Vec::new());
        }
        return Err(Error::SshAgent {
            message: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().map(String::from).collect())
}

/// Test SSH authentication with a provider
pub fn test_authentication(host: &str, key_path: &Path) -> Result<bool> {
    let output = Command::new("ssh")
        .args(["-T", "-o", "StrictHostKeyChecking=no"])
        .args(["-i", &key_path.to_string_lossy()])
        .args(["-o", "IdentitiesOnly=yes"])
        .arg(format!("git@{}", host))
        .output()
        .map_err(|e| Error::SshAuthFailed {
            identity: "test".to_string(),
            provider: host.to_string(),
            message: e.to_string(),
        })?;

    // GitHub returns exit code 1 but prints success message
    // GitLab returns exit code 0
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let success = stdout.contains("successfully authenticated")
        || stderr.contains("successfully authenticated")
        || stdout.contains("Welcome to GitLab")
        || stderr.contains("Welcome to GitLab")
        || output.status.success();

    Ok(success)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_type_parsing() {
        assert_eq!(KeyType::from_str("ed25519"), Some(KeyType::Ed25519));
        assert_eq!(KeyType::from_str("RSA"), Some(KeyType::Rsa));
        assert_eq!(KeyType::from_str("ecdsa"), Some(KeyType::Ecdsa));
        assert_eq!(KeyType::from_str("ECDSA"), Some(KeyType::Ecdsa));
        assert_eq!(KeyType::from_str("unknown"), None);
    }

    #[test]
    fn test_key_type_display() {
        assert_eq!(KeyType::Ed25519.to_string(), "ed25519");
        assert_eq!(KeyType::Rsa.to_string(), "rsa");
        assert_eq!(KeyType::Ecdsa.to_string(), "ecdsa");
    }

    #[test]
    fn test_key_type_default() {
        assert_eq!(KeyType::default(), KeyType::Ed25519);
    }

    #[test]
    fn test_key_type_default_bits() {
        assert_eq!(KeyType::Ed25519.default_bits(), None);
        assert_eq!(KeyType::Rsa.default_bits(), Some(4096));
        assert_eq!(KeyType::Ecdsa.default_bits(), Some(521));
    }
}
