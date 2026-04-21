//! Git config parsing and writing
//!
//! This module handles parsing and modifying Git configuration files.

use std::path::Path;
use std::process::Command;

use crate::error::{Error, Result};

/// Get a Git config value
pub fn get_config(key: &str, scope: ConfigScope) -> Result<Option<String>> {
    let mut cmd = Command::new("git");
    cmd.arg("config");

    match scope {
        ConfigScope::Global => cmd.arg("--global"),
        ConfigScope::System => cmd.arg("--system"),
        ConfigScope::Local => cmd.arg("--local"),
    };

    cmd.arg(key);

    let output = cmd.output().map_err(|e| Error::GitCommand {
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

/// Set a Git config value
pub fn set_config(key: &str, value: &str, scope: ConfigScope) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.arg("config");

    match scope {
        ConfigScope::Global => cmd.arg("--global"),
        ConfigScope::System => cmd.arg("--system"),
        ConfigScope::Local => cmd.arg("--local"),
    };

    cmd.args([key, value]);

    let output = cmd.output().map_err(|e| Error::GitCommand {
        message: e.to_string(),
    })?;

    if output.status.success() {
        Ok(())
    } else {
        Err(Error::GitCommand {
            message: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

/// Unset a Git config value
pub fn unset_config(key: &str, scope: ConfigScope) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.arg("config");

    match scope {
        ConfigScope::Global => cmd.arg("--global"),
        ConfigScope::System => cmd.arg("--system"),
        ConfigScope::Local => cmd.arg("--local"),
    };

    cmd.args(["--unset", key]);

    let output = cmd.output().map_err(|e| Error::GitCommand {
        message: e.to_string(),
    })?;

    // --unset returns error if key doesn't exist, which is fine
    if output.status.success() || output.status.code() == Some(5) {
        Ok(())
    } else {
        Err(Error::GitCommand {
            message: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

/// Git config scope
#[derive(Debug, Clone, Copy)]
pub enum ConfigScope {
    /// Global (~/.gitconfig)
    Global,
    /// System (/etc/gitconfig)
    System,
    /// Local (.git/config)
    Local,
}

/// A conditional include entry
#[derive(Debug, Clone)]
pub struct ConditionalInclude {
    /// Condition (e.g., "gitdir:~/work/")
    pub condition: String,
    /// Path to include file
    pub path: String,
}

/// Find all conditional includes in global config
pub fn find_conditional_includes() -> Result<Vec<ConditionalInclude>> {
    let output = Command::new("git")
        .args(["config", "--global", "--get-regexp", r"^includeIf\."])
        .output()
        .map_err(|e| Error::GitCommand {
            message: e.to_string(),
        })?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut includes = Vec::new();

    for line in stdout.lines() {
        // Format: includeif.gitdir:~/work/.path ~/.gitconfig.d/work
        // Note: git config returns keys in lowercase
        if let Some((key, value)) = line.split_once(' ') {
            if key.ends_with(".path") {
                // Extract condition from key (case-insensitive)
                let condition = key
                    .strip_prefix("includeif.")
                    .and_then(|s| s.strip_suffix(".path"))
                    .unwrap_or("")
                    .to_string();

                includes.push(ConditionalInclude {
                    condition,
                    path: value.to_string(),
                });
            }
        }
    }

    Ok(includes)
}

/// Add a conditional include
pub fn add_conditional_include(condition: &str, include_path: &str) -> Result<()> {
    let key = format!("includeIf.{}.path", condition);
    set_config(&key, include_path, ConfigScope::Global)
}

/// Remove a conditional include
pub fn remove_conditional_include(condition: &str) -> Result<()> {
    let key = format!("includeIf.{}.path", condition);
    unset_config(&key, ConfigScope::Global)
}

/// Find all URL rewrite rules
pub fn find_url_rewrites() -> Result<Vec<(String, String)>> {
    let output = Command::new("git")
        .args([
            "config",
            "--global",
            "--get-regexp",
            r"^url\..*\.insteadof$",
        ])
        .output()
        .map_err(|e| Error::GitCommand {
            message: e.to_string(),
        })?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut rewrites = Vec::new();

    for line in stdout.lines() {
        // Format: url.git@work:.insteadof git@github.com:company/
        // Note: Git config returns keys in lowercase
        if let Some((key, value)) = line.split_once(' ') {
            if let Some(new_url) = key
                .strip_prefix("url.")
                .and_then(|s| s.strip_suffix(".insteadof"))
            {
                rewrites.push((value.to_string(), new_url.to_string()));
            }
        }
    }

    Ok(rewrites)
}

/// Add a URL rewrite rule
pub fn add_url_rewrite(original: &str, replacement: &str) -> Result<()> {
    let key = format!("url.{}.insteadOf", replacement);
    set_config(&key, original, ConfigScope::Global)
}

/// Remove a URL rewrite rule
pub fn remove_url_rewrite(replacement: &str) -> Result<()> {
    let key = format!("url.{}.insteadOf", replacement);
    unset_config(&key, ConfigScope::Global)
}

/// Write an include file for conditional includes
pub fn write_include_file(
    path: &Path,
    email: &str,
    name: &str,
    ssh_key: Option<&str>,
) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut content = format!(
        r#"[user]
    email = {}
    name = {}
"#,
        email, name
    );

    if let Some(key) = ssh_key {
        content.push_str(&format!(
            r#"
[core]
    sshCommand = ssh -i {} -o IdentitiesOnly=yes
"#,
            key
        ));
    }

    std::fs::write(path, content)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_scope() {
        // These tests require git to be installed
        // Integration tests would verify actual behavior
    }
}
