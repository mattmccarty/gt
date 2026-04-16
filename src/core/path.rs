//! Cross-platform path utilities
//!
//! This module provides utilities for handling paths across platforms.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// Get the home directory
///
/// This checks the HOME environment variable first (to support test isolation),
/// then falls back to the dirs crate.
pub fn home_dir() -> Result<PathBuf> {
    // Check HOME env var first - critical for test isolation
    // The dirs crate caches the home dir on first call, so it won't
    // respect subsequent changes to the HOME environment variable
    if let Ok(home) = std::env::var("HOME") {
        let path = PathBuf::from(home);
        if path.is_absolute() {
            return Ok(path);
        }
    }

    // Windows: Check USERPROFILE
    #[cfg(windows)]
    if let Ok(home) = std::env::var("USERPROFILE") {
        let path = PathBuf::from(home);
        if path.is_absolute() {
            return Ok(path);
        }
    }

    // Fall back to dirs crate for normal usage
    dirs::home_dir().ok_or(Error::HomeNotFound)
}

/// Get the SSH directory (~/.ssh)
pub fn ssh_dir() -> Result<PathBuf> {
    Ok(home_dir()?.join(".ssh"))
}

/// Get the SSH config file path
pub fn ssh_config_path() -> Result<PathBuf> {
    Ok(ssh_dir()?.join("config"))
}

/// Get the gt config directory
pub fn config_dir() -> Result<PathBuf> {
    let config = dirs::config_dir().ok_or(Error::HomeNotFound)?;
    Ok(config.join("gt"))
}

/// Get the gt config file path
pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

/// Expand ~ in a path to the home directory
pub fn expand_tilde(path: &Path) -> Result<PathBuf> {
    let path_str = path.to_string_lossy();

    if path_str.starts_with('~') {
        let home = home_dir()?;
        let remainder = path_str
            .strip_prefix('~')
            .unwrap_or("")
            .trim_start_matches(['/', '\\']);

        if remainder.is_empty() {
            Ok(home)
        } else {
            Ok(home.join(remainder))
        }
    } else {
        Ok(path.to_owned())
    }
}

/// Contract a path by replacing home directory with ~
pub fn contract_tilde(path: &Path) -> String {
    if let Ok(home) = home_dir() {
        if let Ok(suffix) = path.strip_prefix(&home) {
            return format!("~/{}", suffix.display());
        }
    }
    path.display().to_string()
}

/// Normalize path separators for the current platform
#[must_use]
pub fn normalize_separators(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        PathBuf::from(path.to_string_lossy().replace('/', "\\"))
    }

    #[cfg(not(windows))]
    {
        path.to_owned()
    }
}

/// Convert path to SSH config format (forward slashes)
#[must_use]
pub fn to_ssh_format(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde() {
        let result = expand_tilde(Path::new("~/.ssh/config")).unwrap();
        assert!(result.is_absolute());
        assert!(result.to_string_lossy().contains(".ssh"));
    }

    #[test]
    fn test_expand_tilde_no_change() {
        let result = expand_tilde(Path::new("/etc/hosts")).unwrap();
        assert_eq!(result, PathBuf::from("/etc/hosts"));
    }

    #[test]
    fn test_contract_tilde() {
        let home = home_dir().unwrap();
        let path = home.join(".ssh").join("config");
        let result = contract_tilde(&path);
        assert!(result.starts_with("~/"));
    }

    #[test]
    fn test_to_ssh_format() {
        let result = to_ssh_format(Path::new("C:\\Users\\test\\.ssh\\config"));
        assert_eq!(result, "C:/Users/test/.ssh/config");
    }
}
