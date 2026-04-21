//! Implementation of `gt clone` command

use crate::cli::args::CloneOpts;
use crate::cli::output::Output;
use crate::cmd::Context;
use crate::error::{Error, Result};
use crate::io::active_id::ActiveIdentity;

/// Execute the clone command
pub fn execute(opts: &CloneOpts, ctx: &Context) -> Result<Output> {
    let config = ctx.require_config()?;

    ctx.info(&format!("Cloning {}", opts.url));

    // Resolve identity with three-tier precedence:
    //   1. `--id` flag (explicit override)
    //   2. Active identity set via `gt config id use` outside a repo
    //   3. URL-based auto-detection (falls back to default internally)
    let identity_name = if let Some(ref id) = opts.id {
        ctx.info(&format!("Using identity '{}' (override)", id));
        id.clone()
    } else if let Some(active) = resolve_active_identity(ctx) {
        ctx.info(&format!("Using active identity '{}'", active));
        active
    } else {
        ctx.debug("Auto-detecting identity from URL and configuration");
        detect_identity_from_url(&opts.url, ctx)?
    };

    // Get identity configuration
    let identity_config = config.get_identity(&identity_name)?;

    if ctx.dry_run {
        return Ok(Output::dry_run(format!(
            "Would clone {} with identity '{}'",
            opts.url, identity_name
        )));
    }

    // Determine the clone destination path
    let dest_path = if let Some(ref path) = opts.path {
        path.clone()
    } else {
        // Extract repository name from URL for default destination
        extract_repo_name(&opts.url)
            .map(std::path::PathBuf::from)
            .ok_or_else(|| Error::UrlTransform {
                message: "Could not extract repository name from URL".to_string(),
            })?
    };

    // Determine the URL to clone based on SSH hostname alias configuration
    let clone_url = if opts.no_transform {
        ctx.info("Using original URL (--no-transform)");
        opts.url.clone()
    } else {
        let provider = &identity_config.provider;

        // Check if SSH hostname alias is enabled for this identity
        let use_hostname_alias = identity_config
            .ssh
            .as_ref()
            .map(|ssh| ssh.use_hostname_alias)
            .unwrap_or(false);

        if use_hostname_alias {
            // Transform URL to use identity-specific SSH host
            ctx.debug(&format!(
                "Transforming URL for SSH hostname alias (identity: {})",
                identity_name
            ));
            transform_url(&opts.url, &identity_name, provider)?
        } else {
            // Use standard provider hostname
            ctx.debug("Using standard provider hostname");
            restore_url(&opts.url, provider)?
        }
    };

    // Execute git clone, injecting the identity so authentication and
    // attribution come from it rather than the ambient cwd. `-c user.*`
    // sets attribution for any in-process git work. `GIT_SSH_COMMAND`
    // selects the identity's key on the clone subprocess, which matters
    // when the URL hasn't been rewritten to an SSH host alias (for example
    // conditional-strategy identities with no SSH alias).
    ctx.info(&format!("Cloning to {}", dest_path.display()));
    let mut cmd = std::process::Command::new("git");
    cmd.arg("-c")
        .arg(format!("user.name={}", identity_config.name))
        .arg("-c")
        .arg(format!("user.email={}", identity_config.email))
        .arg("clone")
        .arg(&clone_url)
        .arg(&dest_path);

    if let Some(key_path) = identity_config
        .ssh
        .as_ref()
        .and_then(|s| s.key_path.as_deref())
    {
        let ssh_command = build_git_ssh_command(key_path);
        ctx.debug(&format!(
            "Setting GIT_SSH_COMMAND for clone: {}",
            ssh_command
        ));
        cmd.env("GIT_SSH_COMMAND", ssh_command);
    }

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::GitCommand {
            message: format!("Failed to clone repository: {}", stderr),
        });
    }

    if !ctx.quiet {
        eprintln!("✓ Repository cloned to {}", dest_path.display());
    }

    // Set local git config for the cloned repository
    set_git_config(&dest_path, "user.name", &identity_config.name, ctx)?;
    set_git_config(&dest_path, "user.email", &identity_config.email, ctx)?;

    if !ctx.quiet {
        eprintln!("✓ Configured git user.name = {}", identity_config.name);
        eprintln!("✓ Configured git user.email = {}", identity_config.email);
    }

    Ok(Output::success(format!(
        "Repository cloned successfully with identity '{}'",
        identity_name
    ))
    .with_detail("identity", &identity_name)
    .with_detail("url", &opts.url)
    .with_detail("path", dest_path.display().to_string()))
}

/// Detect identity from URL using smart heuristics
fn detect_identity_from_url(url: &str, ctx: &Context) -> Result<String> {
    let config = ctx.require_config()?;

    // Extract username/organization from URL
    // Examples:
    //   git@github.com:mattmccartyllc/test.git -> mattmccartyllc
    //   https://github.com/mattmccartyllc/test.git -> mattmccartyllc
    //   git@gitlab.com:myorg/myrepo.git -> myorg
    let username = extract_username_from_url(url);

    if let Some(ref user) = username {
        ctx.debug(&format!("Extracted username from URL: {}", user));

        // Try to find matching identity
        for (identity_name, identity_config) in &config.identities {
            // Check if identity name matches username
            if identity_name.to_lowercase() == user.to_lowercase() {
                ctx.info(&format!(
                    "Auto-detected identity '{}' (name match)",
                    identity_name
                ));
                return Ok(identity_name.clone());
            }

            // Check if email contains username
            if identity_config
                .email
                .to_lowercase()
                .contains(&user.to_lowercase())
            {
                ctx.info(&format!(
                    "Auto-detected identity '{}' (email match)",
                    identity_name
                ));
                return Ok(identity_name.clone());
            }

            // Check if git user name matches
            if identity_config.name.to_lowercase() == user.to_lowercase() {
                ctx.info(&format!(
                    "Auto-detected identity '{}' (user name match)",
                    identity_name
                ));
                return Ok(identity_name.clone());
            }
        }

        ctx.debug(&format!("No identity matched username '{}'", user));
    }

    // Fall back to default identity
    let default = config.defaults.identity.clone().unwrap_or_else(|| {
        // If no default set, use the first identity
        config
            .identities
            .keys()
            .next()
            .cloned()
            .unwrap_or_else(|| "default".to_string())
    });

    ctx.info(&format!("Using default identity '{}'", default));
    Ok(default)
}

/// Extract username/organization from git URL
fn extract_username_from_url(url: &str) -> Option<String> {
    // Handle SSH URLs: git@github.com:username/repo.git
    if url.starts_with("git@") {
        let re = regex::Regex::new(r"git@[^:]+:([^/]+)/").ok()?;
        if let Some(captures) = re.captures(url) {
            return captures.get(1).map(|m| m.as_str().to_string());
        }
    }

    // Handle HTTPS URLs: https://github.com/username/repo.git
    if url.starts_with("http://") || url.starts_with("https://") {
        let re = regex::Regex::new(r"https?://[^/]+/([^/]+)/").ok()?;
        if let Some(captures) = re.captures(url) {
            return captures.get(1).map(|m| m.as_str().to_string());
        }
    }

    None
}

/// Extract repository name from git URL
fn extract_repo_name(url: &str) -> Option<String> {
    // Handle SSH URLs: git@github.com:username/repo.git -> repo
    if url.starts_with("git@") {
        let re = regex::Regex::new(r"git@[^:]+:[^/]+/([^/]+?)(?:\.git)?$").ok()?;
        if let Some(captures) = re.captures(url) {
            return captures.get(1).map(|m| m.as_str().to_string());
        }
    }

    // Handle HTTPS URLs: https://github.com/username/repo.git -> repo
    if url.starts_with("http://") || url.starts_with("https://") {
        let re = regex::Regex::new(r"https?://[^/]+/[^/]+/([^/]+?)(?:\.git)?$").ok()?;
        if let Some(captures) = re.captures(url) {
            return captures.get(1).map(|m| m.as_str().to_string());
        }
    }

    None
}

/// Set a git config value in the repository (local, not global)
fn set_git_config(
    repo_path: &std::path::PathBuf,
    key: &str,
    value: &str,
    ctx: &Context,
) -> Result<()> {
    let output = std::process::Command::new("git")
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

/// Transform a git URL to use the identity-specific SSH host
fn transform_url(url: &str, identity: &str, provider: &str) -> Result<String> {
    use crate::core::provider::Provider;

    // Handle SSH URLs (git@...)
    if url.starts_with("git@") {
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
    use crate::core::provider::Provider;

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

/// Load the active identity from the state file, if set and still valid.
///
/// Returns `None` when the file is absent, unreadable, or names an identity
/// that no longer exists in the main config (for example after the user
/// deleted the identity without running `--clear`). A warning is logged in
/// the stale case so users understand why their declared intent isn't being
/// applied.
fn resolve_active_identity(ctx: &Context) -> Option<String> {
    let active = ActiveIdentity::load().ok().flatten()?;

    let config = ctx.config.as_ref()?;
    if config.identities.contains_key(&active.identity) {
        Some(active.identity)
    } else {
        ctx.info(&format!(
            "Active identity '{}' no longer exists in config; ignoring. \
             Clear with `gt config id use --clear`.",
            active.identity
        ));
        None
    }
}

/// Build the `GIT_SSH_COMMAND` value for a clone using the given key path.
///
/// Paths are normalized to forward slashes so the command works on Windows,
/// where backslashes in the key path break shell-parsed arguments inside
/// `ssh -i "..."`. Git-for-Windows accepts forward slashes natively.
fn build_git_ssh_command(key_path: &str) -> String {
    let normalized = key_path.replace('\\', "/");
    format!("ssh -i \"{}\" -o IdentitiesOnly=yes", normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_ssh_command_wraps_unix_path() {
        let cmd = build_git_ssh_command("/home/user/.ssh/id_gt_work");
        assert_eq!(
            cmd,
            "ssh -i \"/home/user/.ssh/id_gt_work\" -o IdentitiesOnly=yes"
        );
    }

    #[test]
    fn git_ssh_command_normalizes_windows_backslashes() {
        let cmd = build_git_ssh_command(r"C:\Users\matt\.ssh\id_gt_work");
        assert_eq!(
            cmd,
            "ssh -i \"C:/Users/matt/.ssh/id_gt_work\" -o IdentitiesOnly=yes"
        );
    }

    #[test]
    fn git_ssh_command_handles_path_with_spaces() {
        // Spaces survive because the key path is quoted; a subsequent shell
        // split on the outer command string will keep the quoted segment intact.
        let cmd = build_git_ssh_command(r"C:\Users\Matt McCarty\.ssh\id_gt_work");
        assert_eq!(
            cmd,
            "ssh -i \"C:/Users/Matt McCarty/.ssh/id_gt_work\" -o IdentitiesOnly=yes"
        );
    }
}
