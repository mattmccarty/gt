//! Implementation of `gt config id fix` command

use crate::cli::args::FixIdOpts;
use crate::cli::output::Output;
use crate::cmd::Context;
use crate::core::provider::Provider;
use crate::error::{Error, Result};
use crate::strategy::StrategyType;
use std::path::PathBuf;
use std::process::Command;

/// Execute the fix id command
pub fn execute_id(opts: &FixIdOpts, ctx: &Context) -> Result<Output> {
    let path = opts
        .path
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    ctx.info(&format!(
        "Fixing identity configuration in {}",
        path.display()
    ));

    if opts.restore {
        return restore_urls(opts, &path, ctx);
    }

    // Check if it's a git repository
    if !path.join(".git").exists() {
        return Err(Error::NotARepository);
    }

    // Determine which identity to use
    let config = ctx.require_config()?;
    let identity_name = if let Some(ref id) = opts.id {
        id.clone()
    } else {
        // Auto-detect: use default identity
        config
            .defaults
            .identity
            .clone()
            .ok_or_else(|| Error::ConfigInvalid {
                message: "No default identity set. Use --id to specify an identity.".to_string(),
            })?
    };

    // Get identity configuration
    let identity_config =
        config
            .identities
            .get(&identity_name)
            .ok_or_else(|| Error::IdentityNotFound {
                name: identity_name.clone(),
            })?;

    if ctx.dry_run {
        return Ok(Output::dry_run(format!(
            "Would fix identity configuration in {} using identity '{}'",
            path.display(),
            identity_name
        )));
    }

    // Set local git config (user.name, user.email)
    set_git_config(&path, "user.name", &identity_config.name, ctx)?;
    set_git_config(&path, "user.email", &identity_config.email, ctx)?;

    if !ctx.quiet {
        eprintln!("✓ Set local git user.name = {}", identity_config.name);
        eprintln!("✓ Set local git user.email = {}", identity_config.email);
    }

    // Update remote URLs based on strategy
    let strategy = identity_config
        .strategy
        .as_ref()
        .and_then(|s| StrategyType::from_str(s))
        .unwrap_or(StrategyType::SshAlias);

    let provider = &identity_config.provider;

    match strategy {
        StrategyType::SshAlias => {
            // SSH strategy: update URLs to use identity-specific SSH host
            update_remote_urls_ssh(&path, &identity_name, provider, ctx)?;
        }
        StrategyType::UrlRewrite => {
            // URL strategy: restore URLs to standard provider hostname
            restore_remote_urls(&path, provider, ctx)?;
        }
        StrategyType::Conditional => {
            ctx.debug("Conditional strategy - URLs managed by git conditional includes");
        }
    }

    Ok(Output::success(format!(
        "Fixed identity configuration using '{}'",
        identity_name
    ))
    .with_detail("identity", &identity_name)
    .with_detail("path", &path.display().to_string()))
}

fn restore_urls(opts: &FixIdOpts, path: &PathBuf, ctx: &Context) -> Result<Output> {
    if !path.join(".git").exists() {
        return Err(Error::NotARepository);
    }

    if ctx.dry_run {
        return Ok(Output::dry_run(format!(
            "Would restore original URLs in {}",
            path.display()
        )));
    }

    // Determine provider from identity or auto-detect
    let provider = if let Some(ref id) = opts.id {
        let config = ctx.require_config()?;
        let identity_config = config
            .identities
            .get(id)
            .ok_or_else(|| Error::IdentityNotFound { name: id.clone() })?;
        identity_config.provider.clone()
    } else {
        // Try to auto-detect from current URL
        "github".to_string() // Default to GitHub
    };

    restore_remote_urls(path, &provider, ctx)?;

    Ok(Output::success("URLs restored to original format")
        .with_detail("path", &path.display().to_string()))
}

/// Set a git config value in the repository (local, not global)
fn set_git_config(repo_path: &PathBuf, key: &str, value: &str, ctx: &Context) -> Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("config")
        .arg(key)
        .arg(value)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::GitCommand {
            message: format!("Failed to set {} to {}: {}", key, value, stderr),
        });
    }

    ctx.debug(&format!("Set git config {} = {}", key, value));
    Ok(())
}

/// Update remote URLs to use the identity-specific SSH host (for SSH strategy)
fn update_remote_urls_ssh(
    repo_path: &PathBuf,
    identity: &str,
    provider: &str,
    ctx: &Context,
) -> Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("remote")
        .output()?;

    if !output.status.success() {
        return Err(Error::GitCommand {
            message: format!(
                "Failed to list remotes: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        });
    }

    let remotes = String::from_utf8_lossy(&output.stdout);
    let remote_names: Vec<&str> = remotes.lines().collect();

    if remote_names.is_empty() {
        ctx.debug("No remotes found in repository");
        return Ok(());
    }

    let mut updated_count = 0;

    for remote_name in remote_names {
        let output = Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .arg("remote")
            .arg("get-url")
            .arg(remote_name)
            .output()?;

        if !output.status.success() {
            ctx.debug(&format!("Failed to get URL for remote '{}'", remote_name));
            continue;
        }

        let current_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        ctx.debug(&format!(
            "Current URL for '{}': {}",
            remote_name, current_url
        ));

        let new_url = transform_url(&current_url, identity, provider)?;

        if new_url != current_url {
            let output = Command::new("git")
                .arg("-C")
                .arg(repo_path)
                .arg("remote")
                .arg("set-url")
                .arg(remote_name)
                .arg(&new_url)
                .output()?;

            if !output.status.success() {
                return Err(Error::GitCommand {
                    message: format!(
                        "Failed to set remote '{}' URL to '{}': {}",
                        remote_name,
                        new_url,
                        String::from_utf8_lossy(&output.stderr)
                    ),
                });
            }

            if !ctx.quiet {
                eprintln!(
                    "✓ Updated remote '{}': {} → {}",
                    remote_name, current_url, new_url
                );
            }
            updated_count += 1;
        } else {
            ctx.debug(&format!("Remote '{}' already has correct URL", remote_name));
        }
    }

    if updated_count == 0 && !ctx.quiet {
        eprintln!("ℹ️  All remotes already use correct URLs");
    }

    Ok(())
}

/// Restore remote URLs to standard provider hostname (for URL rewrite strategy)
fn restore_remote_urls(repo_path: &PathBuf, provider: &str, ctx: &Context) -> Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("remote")
        .output()?;

    if !output.status.success() {
        return Err(Error::GitCommand {
            message: format!(
                "Failed to list remotes: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        });
    }

    let remotes = String::from_utf8_lossy(&output.stdout);
    let remote_names: Vec<&str> = remotes.lines().collect();

    if remote_names.is_empty() {
        ctx.debug("No remotes found in repository");
        return Ok(());
    }

    let mut updated_count = 0;

    for remote_name in remote_names {
        let output = Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .arg("remote")
            .arg("get-url")
            .arg(remote_name)
            .output()?;

        if !output.status.success() {
            ctx.debug(&format!("Failed to get URL for remote '{}'", remote_name));
            continue;
        }

        let current_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        ctx.debug(&format!(
            "Current URL for '{}': {}",
            remote_name, current_url
        ));

        let new_url = restore_url(&current_url, provider)?;

        if new_url != current_url {
            let output = Command::new("git")
                .arg("-C")
                .arg(repo_path)
                .arg("remote")
                .arg("set-url")
                .arg(remote_name)
                .arg(&new_url)
                .output()?;

            if !output.status.success() {
                return Err(Error::GitCommand {
                    message: format!(
                        "Failed to set remote '{}' URL to '{}': {}",
                        remote_name,
                        new_url,
                        String::from_utf8_lossy(&output.stderr)
                    ),
                });
            }

            if !ctx.quiet {
                eprintln!(
                    "✓ Restored remote '{}': {} → {}",
                    remote_name, current_url, new_url
                );
            }
            updated_count += 1;
        } else {
            ctx.debug(&format!("Remote '{}' already has correct URL", remote_name));
        }
    }

    if updated_count == 0 && !ctx.quiet {
        eprintln!("ℹ️  All remotes already use standard URLs");
    }

    Ok(())
}

/// Transform a git URL to use the identity-specific SSH host
fn transform_url(url: &str, identity: &str, provider: &str) -> Result<String> {
    if url.starts_with("git@") {
        let provider_obj = Provider::from_name(provider);
        let host_suffix = provider_obj.hostname();
        let new_host = format!("gt-{}.{}", identity, host_suffix);

        let re = regex::Regex::new(r"^git@[^:]+:(.*)$").map_err(|e| Error::UrlTransform {
            message: format!("Failed to compile regex pattern: {}", e),
        })?;

        if let Some(captures) = re.captures(url) {
            let path = captures.get(1).map_or("", |m| m.as_str());
            return Ok(format!("git@{}:{}", new_host, path));
        }
    }

    Ok(url.to_string())
}

/// Restore a git URL to use the standard provider hostname
fn restore_url(url: &str, provider: &str) -> Result<String> {
    if url.starts_with("git@") {
        let provider_obj = Provider::from_name(provider);
        let standard_host = provider_obj.hostname();

        let re = regex::Regex::new(r"^git@[^:]+:(.*)$").map_err(|e| Error::UrlTransform {
            message: format!("Failed to compile regex pattern: {}", e),
        })?;

        if let Some(captures) = re.captures(url) {
            let path = captures.get(1).map_or("", |m| m.as_str());
            return Ok(format!("git@{}:{}", standard_host, path));
        }
    }

    Ok(url.to_string())
}
