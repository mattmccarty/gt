//! Implementation of `gt id use` command

use crate::cli::args::UseOpts;
use crate::cli::output::Output;
use crate::cmd::Context;
use crate::core::identity::Identity;
use crate::core::provider::Provider;
use crate::error::{Error, Result};
use crate::io::toml_config::IdentityConfig;
use crate::strategy::conditional::ConditionalStrategy;
use std::path::PathBuf;
use std::process::Command;

/// Execute the use command
pub fn execute(opts: &UseOpts, ctx: &Context) -> Result<Output> {
    let config = ctx.require_config()?;

    // Verify identity exists
    let identity_config = config
        .identities
        .get(&opts.identity)
        .ok_or_else(|| Error::IdentityNotFound {
            name: opts.identity.clone(),
        })?;

    // Handle conditional (directory-based) setup
    if opts.directory.is_some() || opts.global {
        return execute_conditional(opts, ctx, identity_config, &opts.identity);
    }

    // Standard repository-based use
    execute_repository(opts, ctx, identity_config)
}

/// Execute the use command for a specific repository
fn execute_repository(
    opts: &UseOpts,
    ctx: &Context,
    identity_config: &IdentityConfig,
) -> Result<Output> {
    // Determine repository path
    let repo_path = if let Some(ref path) = opts.repo {
        path.clone()
    } else {
        std::env::current_dir()?
    };

    // Verify it's a git repository
    if !repo_path.join(".git").exists() {
        return Err(Error::NotARepository);
    }

    ctx.info(&format!(
        "Using identity '{}' for repository at {}",
        opts.identity,
        repo_path.display()
    ));

    if ctx.dry_run {
        return Ok(Output::dry_run(format!(
            "Would use identity '{}' for repository at {}",
            opts.identity,
            repo_path.display()
        )));
    }

    // Set local git config (user.name and user.email)
    set_git_config(&repo_path, "user.name", &identity_config.name, ctx)?;
    set_git_config(&repo_path, "user.email", &identity_config.email, ctx)?;

    if !ctx.quiet {
        eprintln!("✓ Set local git user.name = {}", identity_config.name);
        eprintln!("✓ Set local git user.email = {}", identity_config.email);
    }

    // Set local core.sshCommand so this repo authenticates with the identity's
    // SSH key, overriding any parent conditional include that would otherwise
    // pick a different key based on directory.
    let key_path = identity_config
        .ssh
        .as_ref()
        .and_then(|s| s.key_path.as_deref());

    if let Some(key_path) = key_path {
        let ssh_command = format!("ssh -i \"{}\" -o IdentitiesOnly=yes", key_path);
        set_git_config(&repo_path, "core.sshCommand", &ssh_command, ctx)?;
        if !ctx.quiet {
            eprintln!("✓ Set local git core.sshCommand to use {}", key_path);
        }
    } else {
        // No key for this identity — clear any stale override from a prior
        // `gt id use` call so we don't authenticate as the wrong identity.
        unset_git_config(&repo_path, "core.sshCommand", ctx)?;
    }

    // Update remote URLs based on SSH hostname alias configuration
    let provider = &identity_config.provider;

    // Check if identity has an SSH strategy with hostname aliasing enabled
    let use_hostname_alias = identity_config
        .find_strategy("ssh")
        .map(|s| s.use_hostname_alias)
        .unwrap_or(false);

    if use_hostname_alias {
        // SSH hostname alias enabled: update URLs to use identity-specific SSH host
        update_remote_urls_ssh(&repo_path, &opts.identity, provider, ctx)?;
    } else {
        // No hostname alias: restore URLs to standard provider hostname
        restore_remote_urls(&repo_path, provider, ctx)?;
    }

    Ok(Output::success(format!(
        "Now using identity '{}' in repository",
        opts.identity
    ))
    .with_detail("identity", &opts.identity)
    .with_detail("repository", &repo_path.display().to_string()))
}

/// Execute the use command for conditional strategy (directory-based)
fn execute_conditional(
    opts: &UseOpts,
    ctx: &Context,
    identity_config: &IdentityConfig,
    identity_name: &str,
) -> Result<Output> {
    let directory = if let Some(ref dir) = opts.directory {
        dir.clone()
    } else if let Some(ref repo) = opts.repo {
        // Use the repo path as the directory
        repo.to_string_lossy().to_string()
    } else {
        // Use current directory
        std::env::current_dir()?.to_string_lossy().to_string()
    };

    ctx.info(&format!(
        "Setting up conditional identity '{}' for directory {}",
        identity_name, directory
    ));

    if ctx.dry_run {
        return Ok(Output::dry_run(format!(
            "Would set up conditional identity '{}' for directory {}",
            identity_name, directory
        ))
        .with_detail("identity", identity_name)
        .with_detail("directory", &directory));
    }

    // Create Identity struct from config
    let identity = Identity {
        name: identity_name.to_string(),
        email: identity_config.email.clone(),
        user_name: identity_config.name.clone(),
        provider: Provider::from_name(&identity_config.provider),
        ssh: identity_config.ssh.as_ref().map(|s| crate::core::identity::SshConfig {
            key_path: s.key_path.clone(),
            key_type: s.key_type.clone(),
            key_bits: None,
        }),
        strategy: Some("conditional".to_string()),
    };

    // Get SSH key path if available
    let ssh_key_path = identity_config
        .ssh
        .as_ref()
        .and_then(|s| s.key_path.as_deref());

    // Set up the conditional strategy
    let conditional = ConditionalStrategy::new();
    let result = conditional.setup_for_directory(&identity, &directory, ssh_key_path)?;

    // Print changes
    if !ctx.quiet {
        for change in &result.changes {
            eprintln!("{}", change);
        }
        for warning in &result.warnings {
            eprintln!("Warning: {}", warning);
        }
    }

    Ok(Output::success(format!(
        "Configured conditional identity '{}' for directory {}",
        identity_name, directory
    ))
    .with_detail("identity", identity_name)
    .with_detail("directory", &directory)
    .with_detail("strategy", "conditional"))
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

/// Unset a git config value in the repository (local, not global).
///
/// Returns `Ok(())` if the key was unset or did not exist. git exits with
/// status 5 when the key is absent, which we treat as success here.
fn unset_git_config(repo_path: &PathBuf, key: &str, ctx: &Context) -> Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("config")
        .arg("--local")
        .arg("--unset")
        .arg(key)
        .output()?;

    // Exit code 5 means the key was not set — not an error for our purposes.
    if !output.status.success() && output.status.code() != Some(5) {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::GitCommand {
            message: format!("Failed to unset {}: {}", key, stderr),
        });
    }

    ctx.debug(&format!("Unset git config {}", key));
    Ok(())
}

/// Update remote URLs to use the identity-specific SSH host (for SSH strategy)
fn update_remote_urls_ssh(
    repo_path: &PathBuf,
    identity: &str,
    provider: &str,
    ctx: &Context,
) -> Result<()> {
    // Get list of remotes
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

    // For each remote, update its URL
    for remote_name in remote_names {
        // Get current URL
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
        ctx.debug(&format!("Current URL for '{}': {}", remote_name, current_url));

        // Transform URL to use identity-specific SSH host
        let new_url = transform_url(&current_url, identity, provider)?;

        if new_url != current_url {
            // Set the new URL
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
                eprintln!("Updated remote '{}': {} -> {}", remote_name, current_url, new_url);
            }
            updated_count += 1;
        } else {
            ctx.debug(&format!("Remote '{}' already has correct URL", remote_name));
        }
    }

    if updated_count == 0 && !ctx.quiet {
        eprintln!("All remotes already use correct URLs");
    }

    Ok(())
}

/// Restore remote URLs to standard provider hostname (for URL rewrite strategy)
fn restore_remote_urls(
    repo_path: &PathBuf,
    provider: &str,
    ctx: &Context,
) -> Result<()> {
    // Get list of remotes
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

    // For each remote, restore its URL to standard provider hostname
    for remote_name in remote_names {
        // Get current URL
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
        ctx.debug(&format!("Current URL for '{}': {}", remote_name, current_url));

        // Restore URL to standard provider hostname
        let new_url = restore_url(&current_url, provider)?;

        if new_url != current_url {
            // Set the new URL
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
                eprintln!("Restored remote '{}': {} -> {}", remote_name, current_url, new_url);
            }
            updated_count += 1;
        } else {
            ctx.debug(&format!("Remote '{}' already has correct URL", remote_name));
        }
    }

    if updated_count == 0 && !ctx.quiet {
        eprintln!("All remotes already use standard URLs");
    }

    Ok(())
}

/// Transform a git URL to use the identity-specific SSH host
fn transform_url(url: &str, identity: &str, provider: &str) -> Result<String> {
    // Handle SSH URLs (git@...)
    if url.starts_with("git@") {
        // Extract the hostname and path
        // Examples:
        //   git@github.com:user/repo.git -> gt-{identity}.github.com
        //   git@gt-work.github.com:user/repo.git -> gt-{identity}.github.com

        let provider_obj = Provider::from_name(provider);
        let host_suffix = provider_obj.hostname();
        let new_host = format!("gt-{}.{}", identity, host_suffix);

        // Use regex to replace the hostname part
        let re = regex::Regex::new(r"^git@[^:]+:(.*)$").map_err(|e| Error::UrlTransform {
            message: format!("Failed to compile regex pattern: {}", e),
        })?;

        if let Some(captures) = re.captures(url) {
            let path = captures.get(1).map_or("", |m| m.as_str());
            return Ok(format!("git@{}:{}", new_host, path));
        }
    }

    // If we can't transform it, return as-is
    Ok(url.to_string())
}

/// Restore a git URL to use the standard provider hostname
fn restore_url(url: &str, provider: &str) -> Result<String> {
    // Handle SSH URLs (git@...)
    if url.starts_with("git@") {
        let provider_obj = Provider::from_name(provider);
        let standard_host = provider_obj.hostname();

        // Use regex to replace any hostname (including gt-* hosts) with standard provider hostname
        let re = regex::Regex::new(r"^git@[^:]+:(.*)$").map_err(|e| Error::UrlTransform {
            message: format!("Failed to compile regex pattern: {}", e),
        })?;

        if let Some(captures) = re.captures(url) {
            let path = captures.get(1).map_or("", |m| m.as_str());
            return Ok(format!("git@{}:{}", standard_host, path));
        }
    }

    // If we can't transform it, return as-is
    Ok(url.to_string())
}
