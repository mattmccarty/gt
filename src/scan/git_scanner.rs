//! Git configuration scanner
//!
//! Scans Git config for identity-related entries.

use crate::error::Result;
use crate::io::git_config;

/// Scan results for Git config
#[derive(Debug, Default)]
pub struct GitScanResult {
    /// Conditional includes found
    pub conditionals: Vec<ConditionalInfo>,
    /// URL rewrites found
    pub url_rewrites: Vec<UrlRewriteInfo>,
    /// Warnings
    pub warnings: Vec<String>,
}

/// Information about a conditional include
#[derive(Debug)]
pub struct ConditionalInfo {
    /// Condition (e.g., "gitdir:~/work/")
    pub condition: String,
    /// Include path
    pub path: String,
    /// Extracted directory (for gitdir conditions)
    pub directory: Option<String>,
    /// Email from include file
    pub email: Option<String>,
    /// Name from include file
    pub name: Option<String>,
    /// Whether include file exists
    pub file_exists: bool,
}

/// Information about a URL rewrite rule
#[derive(Debug)]
pub struct UrlRewriteInfo {
    /// Original URL pattern
    pub original: String,
    /// Replacement URL
    pub replacement: String,
    /// Extracted provider (if detectable)
    pub provider: Option<String>,
}

/// Scan Git configuration
pub fn scan_git_config() -> Result<GitScanResult> {
    let mut result = GitScanResult::default();

    // Scan conditional includes
    let includes = git_config::find_conditional_includes()?;
    for include in includes {
        let directory = if include.condition.starts_with("gitdir:") {
            include.condition.strip_prefix("gitdir:").map(String::from)
        } else {
            None
        };

        // Try to read the include file
        let expanded_path = crate::util::expand_path(std::path::Path::new(&include.path)).ok();
        let file_exists = expanded_path.as_ref().is_some_and(|p| p.exists());

        // Read email and name from include file
        let (email, name) = if let Some(ref path) = expanded_path {
            if path.exists() {
                let content = std::fs::read_to_string(path).unwrap_or_default();
                (
                    extract_config_value(&content, "email"),
                    extract_config_value(&content, "name"),
                )
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        result.conditionals.push(ConditionalInfo {
            condition: include.condition,
            path: include.path,
            directory,
            email,
            name,
            file_exists,
        });
    }

    // Scan URL rewrites
    let rewrites = git_config::find_url_rewrites()?;
    for (original, replacement) in rewrites {
        // Try to detect provider from URL
        let provider = detect_provider_from_url(&original);

        result.url_rewrites.push(UrlRewriteInfo {
            original,
            replacement,
            provider,
        });
    }

    Ok(result)
}

/// Extract a config value from file content
fn extract_config_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.to_lowercase().starts_with(key) {
            if let Some(pos) = line.find('=') {
                return Some(line[pos + 1..].trim().to_string());
            }
        }
    }
    None
}

/// Detect provider from URL
fn detect_provider_from_url(url: &str) -> Option<String> {
    if url.contains("github.com") {
        Some("github".to_string())
    } else if url.contains("gitlab.com") {
        Some("gitlab".to_string())
    } else if url.contains("bitbucket.org") {
        Some("bitbucket".to_string())
    } else if url.contains("dev.azure.com") {
        Some("azure".to_string())
    } else if url.contains("codecommit") {
        Some("codecommit".to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_config_value() {
        let content = r#"
[user]
    email = test@example.com
    name = Test User
"#;

        assert_eq!(
            extract_config_value(content, "email"),
            Some("test@example.com".to_string())
        );
        assert_eq!(
            extract_config_value(content, "name"),
            Some("Test User".to_string())
        );
        assert_eq!(extract_config_value(content, "nonexistent"), None);
    }

    #[test]
    fn test_detect_provider() {
        assert_eq!(
            detect_provider_from_url("git@github.com:user/repo"),
            Some("github".to_string())
        );
        assert_eq!(
            detect_provider_from_url("git@gitlab.com:user/repo"),
            Some("gitlab".to_string())
        );
        assert_eq!(detect_provider_from_url("git@unknown.com:user/repo"), None);
    }
}
