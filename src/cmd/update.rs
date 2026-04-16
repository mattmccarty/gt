//! Implementation of `gt id update` command

use crate::cli::args::UpdateOpts;
use crate::cli::output::Output;
use crate::cmd::Context;
use crate::error::{Error, Result};
use crate::io::{git_config, ssh_config::SshConfig};
use crate::scan::detector;
use crate::strategy::StrategyType;
use crate::util::validate_identity_name;

/// Execute the update command
pub fn execute(opts: &UpdateOpts, ctx: &Context) -> Result<Output> {
    ctx.info(&format!("Updating identity '{}'...", opts.identity));

    // Load config
    let mut config = ctx.require_config()?.clone();

    // Handle rename first if requested (this is a major operation)
    if let Some(ref new_name) = opts.name {
        return handle_rename(&opts.identity, new_name, opts, ctx, config);
    }

    // Check if identity exists in config
    let identity_config = config
        .identities
        .get_mut(&opts.identity)
        .ok_or_else(|| Error::IdentityNotFound {
            name: opts.identity.clone(),
        })?;

    // Detect current identity to get SSH host info
    let all_identities = detector::detect_identities()?;
    let current_identity = all_identities
        .iter()
        .find(|i| i.name == opts.identity)
        .ok_or_else(|| Error::IdentityNotFound {
            name: opts.identity.clone(),
        })?;

    // Get current SSH host
    let current_ssh_host = if let detector::DetectionSource::SshConfig { host } =
        &current_identity.source
    {
        host.clone()
    } else {
        format!("gt-{}.github.com", opts.identity)
    };

    // Parse current and new strategy
    let current_strategy = identity_config
        .strategy
        .as_ref()
        .and_then(|s| StrategyType::from_str(s))
        .unwrap_or(StrategyType::SshAlias);

    let new_strategy = opts
        .strategy
        .as_ref()
        .map(|s| StrategyType::from_str(&s.to_string()))
        .flatten()
        .unwrap_or(current_strategy);

    let strategy_changed = current_strategy != new_strategy;

    if ctx.dry_run {
        let mut changes = Vec::new();
        if let Some(ref email) = opts.email {
            changes.push(format!("email: {} → {}", identity_config.email, email));
        }
        if let Some(ref user) = opts.user {
            changes.push(format!("user: {} → {}", identity_config.name, user));
        }
        if strategy_changed {
            changes.push(format!("strategy: {} → {}", current_strategy, new_strategy));
        }
        return Ok(Output::dry_run(format!(
            "Would update identity '{}': {}",
            opts.identity,
            changes.join(", ")
        )));
    }

    // Update basic fields
    if let Some(ref email) = opts.email {
        // Check if user wants to use default email
        if email.to_lowercase() == "default" {
            // Use GitHub anonymous email format for GitHub, localhost for others
            let provider_str = identity_config.provider.to_lowercase();
            let identity_name = &opts.identity;

            let default_email = if provider_str == "github" {
                format!("{}@users.noreply.github.com", identity_name)
            } else {
                format!("{}@localhost", identity_name)
            };

            identity_config.email = default_email;
        } else {
            identity_config.email = email.clone();
        }
    }
    if let Some(ref user) = opts.user {
        identity_config.name = user.clone();
    }

    // Handle strategy change
    if strategy_changed {
        ctx.info(&format!(
            "Changing strategy: {} → {}",
            current_strategy, new_strategy
        ));

        // Clean up old strategy
        match current_strategy {
            StrategyType::UrlRewrite => {
                // Remove URL rewrites
                let rewrites = git_config::find_url_rewrites()?;
                for (original, replacement) in &rewrites {
                    if replacement.contains(&current_ssh_host) {
                        git_config::remove_url_rewrite(replacement)?;
                        ctx.debug(&format!("Removed URL rewrite: {} → {}", original, replacement));
                        if !ctx.quiet {
                            eprintln!("✓ Removed Git URL rewrite");
                        }
                    }
                }
            }
            _ => {}
        }

        // Set up new strategy
        match new_strategy {
            StrategyType::UrlRewrite => {
                // Add URL rewrites
                let provider = current_identity
                    .provider
                    .as_ref()
                    .map(|p| p.hostname())
                    .unwrap_or("github.com");

                let (original_url, rewrite_url) = if let Some(ref scope) = opts.scope {
                    (
                        format!("git@{}:{}/", provider, scope),
                        format!("git@{}:{}/", current_ssh_host, scope),
                    )
                } else {
                    (
                        format!("git@{}:", provider),
                        format!("git@{}:", current_ssh_host),
                    )
                };

                git_config::add_url_rewrite(&original_url, &rewrite_url)?;

                ctx.debug(&format!(
                    "Added Git URL rewrite: {} → {}",
                    original_url, rewrite_url
                ));

                if !ctx.quiet {
                    eprintln!("✓ Added Git URL rewrite");
                    eprintln!("  {} → {}", original_url, rewrite_url);
                }

                if opts.scope.is_none() {
                    eprintln!("⚠️  Warning: Full provider rewrite enabled. This will affect ALL repositories for {}.", provider);
                    eprintln!("   Consider using --scope to limit rewrites to specific organizations.");
                }
            }
            _ => {}
        }

        // Update strategy in config
        identity_config.strategy = Some(new_strategy.to_string());
    }

    // Clone values before saving (to avoid borrow checker issues)
    let final_email = identity_config.email.clone();
    let final_user = identity_config.name.clone();

    // Save updated config
    config.save(&ctx.config_path)?;

    let mut output = Output::success(format!("Updated identity '{}'", opts.identity))
        .with_detail("strategy", &new_strategy.to_string());

    if opts.email.is_some() {
        output = output.with_detail("email", &final_email);
    }
    if opts.user.is_some() {
        output = output.with_detail("user", &final_user);
    }

    if !ctx.quiet {
        eprintln!("✓ Updated identity '{}'", opts.identity);
        if opts.email.is_some() {
            eprintln!("  Email:    {}", final_email);
        }
        if let Some(ref user) = opts.user {
            eprintln!("  User:     {}", user);
        }
        if strategy_changed {
            eprintln!("  Strategy: {} → {}", current_strategy, new_strategy);
        }
    }

    Ok(output)
}

/// Handle renaming an identity
fn handle_rename(
    old_name: &str,
    new_name: &str,
    opts: &UpdateOpts,
    ctx: &Context,
    mut config: crate::io::toml_config::GtConfig,
) -> Result<Output> {
    // If the name isn't changing, just do a regular update
    if old_name == new_name {
        ctx.info(&format!("Updating identity '{}'...", old_name));

        // Get the identity config
        let identity_config = config
            .identities
            .get_mut(old_name)
            .ok_or_else(|| Error::IdentityNotFound {
                name: old_name.to_string(),
            })?;

        // Apply updates
        if let Some(ref email) = opts.email {
            if email.to_lowercase() == "default" {
                let provider_str = identity_config.provider.to_lowercase();
                let default_email = if provider_str == "github" {
                    format!("{}@users.noreply.github.com", old_name)
                } else {
                    format!("{}@localhost", old_name)
                };
                identity_config.email = default_email.clone();
            } else {
                identity_config.email = email.clone();
            }
        }

        if let Some(ref user) = opts.user {
            identity_config.name = user.clone();
        }

        // Handle strategy change
        let current_strategy = identity_config
            .strategy
            .as_ref()
            .and_then(|s| StrategyType::from_str(s))
            .unwrap_or(StrategyType::SshAlias);

        let new_strategy = opts
            .strategy
            .as_ref()
            .and_then(|s| StrategyType::from_str(&s.to_string()))
            .unwrap_or(current_strategy);

        if current_strategy != new_strategy {
            // This is handled by the regular update path - just update the config
            identity_config.strategy = Some(new_strategy.to_string());

            // Note: URL rewrites and other strategy-specific changes would need
            // to be handled here if we want to support strategy changes without rename
            ctx.info(&format!("Strategy changed to {}. Use regular update command for full strategy migration.", new_strategy));
        }

        // Clone values before save (borrow checker)
        let final_email = identity_config.email.clone();
        let final_user = identity_config.name.clone();

        // Save config
        config.save(&ctx.config_path)?;

        let mut output = Output::success(format!("Updated identity '{}'", old_name))
            .with_detail("strategy", &new_strategy.to_string());

        if opts.email.is_some() {
            output = output.with_detail("email", &final_email);
        }
        if opts.user.is_some() {
            output = output.with_detail("user", &final_user);
        }

        if !ctx.quiet {
            eprintln!("✓ Updated identity '{}'", old_name);
            if opts.email.is_some() {
                eprintln!("  Email:    {}", final_email);
            }
            if opts.user.is_some() {
                eprintln!("  User:     {}", final_user);
            }
        }

        return Ok(output);
    }

    ctx.info(&format!("Renaming identity '{}' to '{}'...", old_name, new_name));

    // Validate new name
    validate_identity_name(new_name)?;

    // Check if new name already exists (unless it's the same as the old name)
    if old_name != new_name && config.identities.contains_key(new_name) {
        return Err(Error::IdentityExists {
            name: new_name.to_string(),
        });
    }

    // Check if old identity exists
    let identity_config = config
        .identities
        .get(old_name)
        .ok_or_else(|| Error::IdentityNotFound {
            name: old_name.to_string(),
        })?
        .clone();

    // Detect current identity to get SSH host info and strategy
    let all_identities = detector::detect_identities()?;
    let current_identity = all_identities
        .iter()
        .find(|i| i.name == old_name)
        .ok_or_else(|| Error::IdentityNotFound {
            name: old_name.to_string(),
        })?;

    // Get current and new SSH hosts
    let old_ssh_host = if let detector::DetectionSource::SshConfig { host } =
        &current_identity.source
    {
        host.clone()
    } else {
        format!("gt-{}.github.com", old_name)
    };

    let new_ssh_host = format!("gt-{}.github.com", new_name);

    // Get strategy
    let strategy = identity_config
        .strategy
        .as_ref()
        .and_then(|s| StrategyType::from_str(s))
        .unwrap_or(StrategyType::SshAlias);

    if ctx.dry_run {
        let mut changes = vec![format!("rename: {} → {}", old_name, new_name)];
        if let Some(ref email) = opts.email {
            changes.push(format!("email: {}", email));
        }
        if let Some(ref user) = opts.user {
            changes.push(format!("user: {}", user));
        }
        if let Some(ref new_strat) = opts.strategy {
            changes.push(format!("strategy: {} → {}", strategy, new_strat));
        }
        return Ok(Output::dry_run(format!(
            "Would update identity '{}': {}",
            old_name,
            changes.join(", ")
        ))
        .with_detail("old_ssh_host", &old_ssh_host)
        .with_detail("new_ssh_host", &new_ssh_host));
    }

    // 1. Update SSH config
    ctx.info("Updating SSH config...");
    let ssh_config_path = crate::core::path::ssh_config_path()?;
    if ssh_config_path.exists() {
        let mut ssh_config = SshConfig::load(&ssh_config_path)?;

        // Remove old host entry
        if let Some(old_entry) = ssh_config.remove_host(&old_ssh_host) {
            // Create new entry with updated host
            let mut new_entry = crate::io::ssh_config::SshHostEntry::new(&new_ssh_host);

            if let Some(ref hostname) = old_entry.hostname {
                new_entry = new_entry.with_hostname(hostname);
            }

            // Update identity file path if it contains the old name
            if let Some(ref identity_file) = old_entry.identity_file {
                let updated_identity_file = if identity_file.contains(old_name) {
                    identity_file.replace(old_name, new_name)
                } else {
                    identity_file.clone()
                };
                new_entry = new_entry.with_identity_file(updated_identity_file);
            }

            if let Some(ref user) = old_entry.user {
                new_entry = new_entry.with_user(user);
            }

            if let Some(identities_only) = old_entry.identities_only {
                new_entry = new_entry.with_identities_only(identities_only);
            }

            ssh_config.upsert_host(new_entry);
            ssh_config.save(&ssh_config_path)?;

            if !ctx.quiet {
                eprintln!("✓ Updated SSH config: {} → {}", old_ssh_host, new_ssh_host);
            }
        }
    }

    // 2. Rename SSH key files (if they exist) and update config
    let mut updated_identity_config = identity_config.clone();
    if let Some(ref mut ssh) = updated_identity_config.ssh {
        if let Some(ref key_path) = ssh.key_path {
            let old_key_path = crate::util::expand_path(std::path::Path::new(key_path))?;

            // Determine new key path
            let new_key_path = if key_path.contains(old_name) {
                key_path.replace(old_name, new_name)
            } else {
                format!("~/.ssh/id_gt_{}", new_name)
            };
            let new_key_path_expanded = crate::util::expand_path(std::path::Path::new(&new_key_path))?;

            // Rename private key
            if old_key_path.exists() {
                std::fs::rename(&old_key_path, &new_key_path_expanded)?;
                if !ctx.quiet {
                    eprintln!("✓ Renamed SSH key: {} → {}", key_path, new_key_path);
                }

                // Rename public key (append .pub to the file path)
                let old_pub_path = format!("{}.pub", old_key_path.display());
                let new_pub_path = format!("{}.pub", new_key_path_expanded.display());
                if std::path::Path::new(&old_pub_path).exists() {
                    std::fs::rename(&old_pub_path, &new_pub_path)?;
                }

                // Update key path in config
                ssh.key_path = Some(new_key_path);
            }
        }
    }

    // 3. Update Git URL rewrites if using URL rewrite strategy
    if strategy == StrategyType::UrlRewrite {
        ctx.info("Updating Git URL rewrites...");
        let rewrites = git_config::find_url_rewrites()?;

        for (original, replacement) in &rewrites {
            if replacement.contains(&old_ssh_host) {
                // Remove old rewrite
                git_config::remove_url_rewrite(replacement)?;

                // Add new rewrite with updated host
                let new_replacement = replacement.replace(&old_ssh_host, &new_ssh_host);
                git_config::add_url_rewrite(original, &new_replacement)?;

                if !ctx.quiet {
                    eprintln!("✓ Updated Git URL rewrite");
                    eprintln!("  {} → {} (was {})", original, new_replacement, replacement);
                }
            }
        }
    }

    // 4. Apply other updates (email, user, strategy) if requested
    if let Some(ref email) = opts.email {
        // Check if user wants to use default email
        if email.to_lowercase() == "default" {
            // Use GitHub anonymous email format for GitHub, localhost for others
            let provider_str = updated_identity_config.provider.to_lowercase();

            let default_email = if provider_str == "github" {
                format!("{}@users.noreply.github.com", new_name)
            } else {
                format!("{}@localhost", new_name)
            };

            updated_identity_config.email = default_email;
        } else {
            updated_identity_config.email = email.clone();
        }
    }
    if let Some(ref user) = opts.user {
        updated_identity_config.name = user.clone();
    }

    // Handle strategy change if requested
    let new_strategy = opts
        .strategy
        .as_ref()
        .and_then(|s| StrategyType::from_str(&s.to_string()))
        .unwrap_or(strategy);

    let strategy_changed = strategy != new_strategy;

    if strategy_changed {
        ctx.info(&format!(
            "Changing strategy: {} → {}",
            strategy, new_strategy
        ));

        // Clean up old URL rewrites if changing from URL strategy
        if strategy == StrategyType::UrlRewrite {
            let rewrites = git_config::find_url_rewrites()?;
            for (original, replacement) in &rewrites {
                if replacement.contains(&new_ssh_host) {
                    git_config::remove_url_rewrite(replacement)?;
                    ctx.debug(&format!("Removed URL rewrite: {} → {}", original, replacement));
                    if !ctx.quiet {
                        eprintln!("✓ Removed old Git URL rewrite");
                    }
                }
            }
        }

        // Set up new URL rewrites if changing to URL strategy
        if new_strategy == StrategyType::UrlRewrite {
            let provider = current_identity
                .provider
                .as_ref()
                .map(|p| p.hostname())
                .unwrap_or("github.com");

            let (original_url, rewrite_url) = if let Some(ref scope) = opts.scope {
                (
                    format!("git@{}:{}/", provider, scope),
                    format!("git@{}:{}/", new_ssh_host, scope),
                )
            } else {
                (
                    format!("git@{}:", provider),
                    format!("git@{}:", new_ssh_host),
                )
            };

            git_config::add_url_rewrite(&original_url, &rewrite_url)?;

            ctx.debug(&format!(
                "Added Git URL rewrite: {} → {}",
                original_url, rewrite_url
            ));

            if !ctx.quiet {
                eprintln!("✓ Added Git URL rewrite");
                eprintln!("  {} → {}", original_url, rewrite_url);
            }

            if opts.scope.is_none() {
                eprintln!("⚠️  Warning: Full provider rewrite enabled. This will affect ALL repositories for {}.", provider);
                eprintln!("   Consider using --scope to limit rewrites to specific organizations.");
            }
        }

        // Update strategy in config
        updated_identity_config.strategy = Some(new_strategy.to_string());
    }

    // 5. Update config file (remove old, add new)
    config.identities.remove(old_name);
    config.set_identity(new_name.to_string(), updated_identity_config.clone());
    config.save(&ctx.config_path)?;

    if !ctx.quiet {
        eprintln!("✓ Updated configuration file");

        // Show what was updated
        if opts.email.is_some() {
            eprintln!("  Email:    {}", updated_identity_config.email);
        }
        if opts.user.is_some() {
            eprintln!("  User:     {}", updated_identity_config.name);
        }
        if strategy_changed {
            eprintln!("  Strategy: {} → {}", strategy, new_strategy);
        }
    }

    let mut output = Output::success(format!(
        "Renamed identity '{}' to '{}'",
        old_name, new_name
    ))
    .with_detail("old_name", old_name)
    .with_detail("new_name", new_name)
    .with_detail("old_ssh_host", &old_ssh_host)
    .with_detail("new_ssh_host", &new_ssh_host)
    .with_detail("strategy", &new_strategy.to_string());

    if opts.email.is_some() {
        output = output.with_detail("email", &updated_identity_config.email);
    }
    if opts.user.is_some() {
        output = output.with_detail("user", &updated_identity_config.name);
    }

    Ok(output)
}
