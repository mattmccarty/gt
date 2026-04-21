//! Repository detection and operations
//!
//! This module handles detecting Git repositories and reading their configuration.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::core::url::GitUrl;
use crate::error::{Error, Result};

/// A Git repository
#[derive(Debug, Clone)]
pub struct Repo {
    /// Path to the repository root
    pub path: PathBuf,
    /// Remote URL (origin)
    pub remote_url: Option<String>,
    /// Parsed remote URL
    pub parsed_url: Option<GitUrl>,
}

impl Repo {
    /// Detect a repository at the given path (or current directory)
    pub fn detect(path: Option<&PathBuf>) -> Result<Self> {
        let path = path
            .cloned()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        // Check if this is a Git repository
        let repo_root = Self::find_repo_root(&path)?;

        // Get the remote URL
        let remote_url = Self::get_remote_url(&repo_root, "origin").ok();

        // Parse the URL
        let parsed_url = remote_url.as_ref().and_then(|url| GitUrl::parse(url).ok());

        Ok(Repo {
            path: repo_root,
            remote_url,
            parsed_url,
        })
    }

    /// Find the repository root from a path
    fn find_repo_root(path: &Path) -> Result<PathBuf> {
        let output = Command::new("git")
            .current_dir(path)
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .map_err(|e| Error::GitCommand {
                message: e.to_string(),
            })?;

        if !output.status.success() {
            return Err(Error::NotARepository);
        }

        let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(PathBuf::from(root))
    }

    /// Get the URL of a remote
    fn get_remote_url(repo_path: &Path, remote: &str) -> Result<String> {
        let output = Command::new("git")
            .current_dir(repo_path)
            .args(["remote", "get-url", remote])
            .output()
            .map_err(|e| Error::GitCommand {
                message: e.to_string(),
            })?;

        if !output.status.success() {
            return Err(Error::NoRemote {
                remote: remote.to_string(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Set the URL of a remote
    pub fn set_remote_url(&self, remote: &str, url: &str) -> Result<()> {
        let output = Command::new("git")
            .current_dir(&self.path)
            .args(["remote", "set-url", remote, url])
            .output()
            .map_err(|e| Error::GitCommand {
                message: e.to_string(),
            })?;

        if !output.status.success() {
            return Err(Error::GitCommand {
                message: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        Ok(())
    }

    /// Get Git config value for this repository
    pub fn get_config(&self, key: &str) -> Result<Option<String>> {
        let output = Command::new("git")
            .current_dir(&self.path)
            .args(["config", "--local", key])
            .output()
            .map_err(|e| Error::GitCommand {
                message: e.to_string(),
            })?;

        if output.status.success() {
            Ok(Some(
                String::from_utf8_lossy(&output.stdout).trim().to_string(),
            ))
        } else {
            Ok(None)
        }
    }

    /// Set Git config value for this repository
    pub fn set_config(&self, key: &str, value: &str) -> Result<()> {
        let output = Command::new("git")
            .current_dir(&self.path)
            .args(["config", "--local", key, value])
            .output()
            .map_err(|e| Error::GitCommand {
                message: e.to_string(),
            })?;

        if !output.status.success() {
            return Err(Error::GitCommand {
                message: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        Ok(())
    }

    /// Check if the remote URL has been modified with a gitid identity
    #[must_use]
    pub fn is_url_modified(&self) -> bool {
        self.parsed_url
            .as_ref()
            .map_or(false, |url| url.is_modified())
    }

    /// Get the detected identity from the URL
    #[must_use]
    pub fn detected_identity(&self) -> Option<&str> {
        self.parsed_url
            .as_ref()
            .and_then(|url| url.identity.as_deref())
    }
}

#[cfg(test)]
mod tests {
    // Tests would require a real Git repository
    // Use integration tests for full testing
}
