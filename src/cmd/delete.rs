//! Implementation of `gt id delete` command (refactored for multi-strategy)

use crate::cli::args::DeleteOpts;
use crate::cli::output::Output;
use crate::cmd::Context;
use crate::core::path;
use crate::core::provider::Provider;
use crate::error::{Error, Result};
use crate::io::ssh_config::SshConfig;
use std::path::PathBuf;

/// Execute the delete command
pub fn execute(opts: &DeleteOpts, ctx: &Context) -> Result<Output> {
    // Check if this is a selective strategy deletion or full identity deletion
    if opts.strategy.is_some() {
        delete_strategy_variant(opts, ctx)
    } else {
        delete_full_identity(opts, ctx)
    }
}

/// Delete a specific strategy variant from an identity
fn delete_strategy_variant(opts: &DeleteOpts, ctx: &Context) -> Result<Output> {
    let strategy_type = opts.strategy.as_ref().unwrap().to_string();

    // Must have config file for strategy deletion
    if !ctx.has_config() {
        return Err(Error::ConfigNotFound {
            path: ctx.config_path.clone(),
        });
    }

    let mut config = ctx.require_config()?.clone();

    // Check if identity exists in config
    let identity_config = config
        .identities
        .get_mut(&opts.identity)
        .ok_or_else(|| Error::IdentityNotFound {
            name: opts.identity.clone(),
        })?;

    // Migrate legacy strategies if needed
    identity_config.migrate_legacy_strategies();

    // Get discriminator based on strategy type and flags
    let discriminator = match strategy_type.as_str() {
        "conditional" => opts.directory.clone(),
        "url" => opts.scope.clone(),
        _ => None,
    };

    ctx.info(&format!(
        "Deleting {} strategy variant from identity '{}'{}",
        strategy_type,
        opts.identity,
        discriminator
            .as_ref()
            .map(|d| format!(" ({})", d))
            .unwrap_or_default()
    ));

    // Find the strategy variant to delete
    let strategy_to_delete = identity_config
        .find_strategy_variant(&strategy_type, discriminator.as_deref())
        .ok_or_else(|| Error::ConfigInvalid {
            message: format!(
                "Strategy variant '{}' not found for identity '{}'",
                strategy_type, opts.identity
            ),
        })?
        .clone();

    if ctx.dry_run {
        return Ok(Output::dry_run(format!(
            "Would delete {} strategy from identity '{}'",
            strategy_type, opts.identity
        )));
    }

    // Remove strategy-specific infrastructure
    remove_strategy_infrastructure(&opts.identity, &strategy_to_delete, ctx)?;

    // Remove the strategy from config
    let removed = identity_config.remove_strategy(&strategy_type, discriminator.as_deref());

    if !removed {
        return Err(Error::ConfigInvalid {
            message: format!("Failed to remove strategy variant"),
        });
    }

    // Check if this was the last strategy
    if identity_config.strategies.is_empty() {
        ctx.info(&format!(
            "This was the last strategy for identity '{}', removing entire identity",
            opts.identity
        ));
        config.identities.remove(&opts.identity);
    }

    // Save updated config
    config.save(&ctx.config_path)?;

    if !ctx.quiet {
        eprintln!(
            "✓ Deleted {} strategy from identity '{}'",
            strategy_type, opts.identity
        );
    }

    Ok(Output::success(format!(
        "Deleted {} strategy from identity '{}'",
        strategy_type, opts.identity
    ))
    .with_detail("strategy_deleted", &strategy_type))
}

/// Delete an entire identity (all strategy variants)
fn delete_full_identity(opts: &DeleteOpts, ctx: &Context) -> Result<Output> {
    ctx.info(&format!("Deleting all strategies for identity '{}'...", opts.identity));

    // Check if identity exists in config file
    let has_config = ctx.has_config();
    let in_config = if has_config {
        let config = ctx.require_config()?;
        config.identities.contains_key(&opts.identity)
    } else {
        false
    };

    if !in_config {
        return Err(Error::IdentityNotFound {
            name: opts.identity.clone(),
        });
    }

    // Get identity config before deletion
    let config = ctx.require_config()?;
    let mut identity_config = config
        .identities
        .get(&opts.identity)
        .ok_or_else(|| Error::IdentityNotFound {
            name: opts.identity.clone(),
        })?
        .clone();

    // Migrate legacy strategies if needed
    identity_config.migrate_legacy_strategies();

    let key_path_opt = identity_config
        .ssh
        .as_ref()
        .and_then(|s| s.key_path.clone());

    // Display what will be deleted
    if !ctx.quiet {
        eprintln!("\nIdentity to delete:");
        eprintln!("  Name: {}", opts.identity);
        eprintln!("  Strategies: {}", identity_config.strategies.len());
        for strategy in &identity_config.strategies {
            let scope_info = match strategy.strategy_type.as_str() {
                "conditional" => strategy
                    .directory
                    .as_ref()
                    .map(|d| format!(" ({})", d))
                    .unwrap_or_default(),
                "url" => strategy
                    .scope
                    .as_ref()
                    .map(|s| format!(" ({})", s))
                    .unwrap_or_default(),
                _ => String::new(),
            };
            eprintln!("    - {}{}", strategy.strategy_type, scope_info);
        }
        if let Some(ref key) = key_path_opt {
            eprintln!("  Key:  {}", key);
        }
        eprintln!();
    }

    // Confirm identity deletion (unless --force)
    if !ctx.force && !opts.delete_key && !opts.keep_key {
        #[cfg(feature = "interactive")]
        {
            use dialoguer::Confirm;
            if !Confirm::new()
                .with_prompt("Delete this identity and all its strategy variants?")
                .default(false)
                .interact()
                .unwrap_or(false)
            {
                return Err(Error::Cancelled);
            }
        }
        #[cfg(not(feature = "interactive"))]
        {
            return Err(Error::InputRequired {
                field: "confirmation (use --force to skip)".to_string(),
            });
        }
    }

    if ctx.dry_run {
        let mut msg = format!("Would delete identity '{}' with {} strategies", opts.identity, identity_config.strategies.len());
        if let Some(ref key) = key_path_opt {
            if !opts.keep_key {
                msg.push_str(&format!(" and SSH key '{}'", key));
            }
        }
        return Ok(Output::dry_run(msg));
    }

    // Remove all strategy-specific infrastructure
    for strategy in &identity_config.strategies {
        remove_strategy_infrastructure(&opts.identity, strategy, ctx)?;
    }

    // Remove SSH config entry for the identity
    let provider = Provider::from_name(&identity_config.provider);
    let ssh_host = format!("gt-{}.{}", opts.identity, provider.hostname());
    let ssh_config_path = path::ssh_config_path()?;
    if ssh_config_path.exists() {
        let mut ssh_config = SshConfig::load(&ssh_config_path)?;
        if ssh_config.remove_host(&ssh_host).is_some() {
            ssh_config.save(&ssh_config_path)?;
            ctx.debug(&format!("Removed SSH config entry: {}", ssh_host));
            if !ctx.quiet {
                eprintln!("✓ Removed identity '{}' from SSH config", opts.identity);
            }
        }
    }

    // Remove from gt config file
    let mut gt_config = ctx.require_config()?.clone();
    gt_config.identities.remove(&opts.identity);
    gt_config.save(&ctx.config_path)?;

    if !ctx.quiet {
        eprintln!(
            "✓ Removed identity '{}' from configuration file",
            opts.identity
        );
    }

    // Handle SSH key deletion
    let key_deleted = if let Some(ref key) = key_path_opt {
        if opts.keep_key {
            false
        } else {
            let should_delete_key = if ctx.force || opts.delete_key {
                true
            } else {
                #[cfg(feature = "interactive")]
                {
                    use dialoguer::Confirm;
                    Confirm::new()
                        .with_prompt(format!(
                            "Also delete SSH key '{}'? (This cannot be undone)",
                            key
                        ))
                        .default(false)
                        .interact()
                        .unwrap_or(false)
                }
                #[cfg(not(feature = "interactive"))]
                {
                    eprintln!(
                        "⚠️  SSH key '{}' was not deleted. Use --delete-key or --keep-key to specify.",
                        key
                    );
                    false
                }
            };

            if should_delete_key {
                delete_ssh_key(key, ctx)?;
                if !ctx.quiet {
                    eprintln!("✓ Deleted SSH key '{}'", key);
                }
                true
            } else {
                if !ctx.quiet {
                    eprintln!("ℹ️  Kept SSH key '{}'", key);
                }
                false
            }
        }
    } else {
        false
    };

    Ok(Output::success(format!("Deleted identity '{}'", opts.identity))
        .with_detail("identity_deleted", "true")
        .with_detail("strategies_deleted", &identity_config.strategies.len().to_string())
        .with_detail("key_deleted", &key_deleted.to_string()))
}

/// Remove strategy-specific infrastructure (git config, SSH config, etc.)
fn remove_strategy_infrastructure(
    identity_name: &str,
    strategy: &crate::io::toml_config::StrategyConfig,
    ctx: &Context,
) -> Result<()> {
    use crate::io::git_config;

    match strategy.strategy_type.as_str() {
        "conditional" => {
            if let Some(ref directory) = strategy.directory {
                // Normalize directory path - ensure it ends with /
                let mut normalized_dir = directory.trim().to_string();
                if !normalized_dir.ends_with('/') {
                    normalized_dir.push('/');
                }

                // Remove conditional include from global gitconfig
                let condition = format!("gitdir:{}", normalized_dir);
                let include_path = format!(
                    "{}/.gitconfig.d/{}",
                    dirs::home_dir().unwrap().display(),
                    identity_name
                );

                git_config::remove_conditional_include(&condition)?;
                ctx.debug(&format!("Removed conditional include: {}", condition));

                // Remove the include file if it exists
                let include_file = PathBuf::from(&include_path);
                if include_file.exists() {
                    std::fs::remove_file(&include_file)?;
                    ctx.debug(&format!("Removed include file: {}", include_path));
                }

                if !ctx.quiet {
                    eprintln!("  ✓ Removed conditional include for {}", directory);
                }
            }
        }
        "url" => {
            if let Some(ref scope) = strategy.scope {
                // Find and remove URL rewrites for this scope
                let rewrites = git_config::find_url_rewrites()?;
                for (original, replacement) in &rewrites {
                    // Check if this rewrite matches the scope
                    if original.contains(scope) && replacement.contains(identity_name) {
                        git_config::remove_url_rewrite(replacement)?;
                        ctx.debug(&format!("Removed URL rewrite: {} → {}", original, replacement));
                        if !ctx.quiet {
                            eprintln!("  ✓ Removed URL rewrite for scope '{}'", scope);
                        }
                    }
                }
            }
        }
        "ssh" => {
            // SSH cleanup is handled by the main delete function
            ctx.debug("SSH strategy cleanup handled by main delete");
        }
        _ => {
            ctx.debug(&format!("Unknown strategy type: {}", strategy.strategy_type));
        }
    }

    Ok(())
}

/// Delete an SSH key file (private and public)
fn delete_ssh_key(key_path: &str, ctx: &Context) -> Result<()> {
    let key_path_buf = PathBuf::from(key_path);
    let expanded_key_path = path::expand_tilde(&key_path_buf)?;

    // Delete private key
    if expanded_key_path.exists() {
        std::fs::remove_file(&expanded_key_path)?;
        ctx.debug(&format!(
            "Deleted private key: {}",
            expanded_key_path.display()
        ));
    } else {
        ctx.debug(&format!(
            "Private key not found: {}",
            expanded_key_path.display()
        ));
    }

    // Delete public key if it exists
    let pub_key_path = expanded_key_path.with_extension("pub");
    if pub_key_path.exists() {
        std::fs::remove_file(&pub_key_path)?;
        ctx.debug(&format!("Deleted public key: {}", pub_key_path.display()));
    }

    Ok(())
}
