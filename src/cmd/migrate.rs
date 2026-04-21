//! Implementation of `gt id migrate` command

use crate::cli::args::MigrateOpts;
use crate::cli::output::Output;
use crate::cmd::Context;
use crate::core::path;
use crate::error::{Error, Result};
use crate::io::ssh_config::SshConfig;
use crate::scan::detector;
use std::path::PathBuf;

/// Execute the migrate command
pub fn execute(opts: &MigrateOpts, ctx: &Context) -> Result<Output> {
    // If target strategy specified, handle strategy migration
    if let Some(_target) = &opts.target {
        return migrate_strategy(opts, ctx);
    }

    // Otherwise, handle legacy gitid->gt migration
    migrate_legacy(opts, ctx)
}

/// Migrate from legacy gitid-* naming to gt-* naming
fn migrate_legacy(opts: &MigrateOpts, ctx: &Context) -> Result<Output> {
    ctx.info("Scanning for legacy identities...");

    // Detect all identities
    let all_identities = detector::detect_identities()?;

    // Filter to only legacy SSH-based identities
    let legacy_identities: Vec<_> = all_identities
        .iter()
        .filter(|i| i.is_legacy && i.strategy == crate::strategy::StrategyType::SshAlias)
        .collect();

    if legacy_identities.is_empty() {
        return Ok(Output::success("No legacy identities found to migrate"));
    }

    // Determine which identity/identities to migrate
    let to_migrate = if opts.all {
        legacy_identities
    } else if let Some(ref name) = opts.identity {
        // Find specific identity by name
        let identity = legacy_identities
            .iter()
            .find(|i| i.name == *name)
            .ok_or_else(|| Error::IdentityNotFound { name: name.clone() })?;
        vec![*identity]
    } else {
        // Interactive selection
        select_identity_interactive(&legacy_identities, ctx)?
    };

    // Show what will be migrated
    if !ctx.quiet {
        eprintln!(
            "\nWill migrate {} identit{}:",
            to_migrate.len(),
            if to_migrate.len() == 1 { "y" } else { "ies" }
        );
        for identity in &to_migrate {
            if let detector::DetectionSource::SshConfig { host } = &identity.source {
                eprintln!("  {} ({})", identity.name, host);
                if let Some(ref key_path) = identity.key_path {
                    eprintln!("    Key: {}", key_path);
                }
            }
        }
        eprintln!();
    }

    // Confirm unless --yes flag
    if !opts.yes && !ctx.force {
        #[cfg(feature = "interactive")]
        {
            use dialoguer::Confirm;
            if !Confirm::new()
                .with_prompt("Continue with migration?")
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
                field: "confirmation (use --yes to skip)".to_string(),
            });
        }
    }

    if ctx.dry_run {
        return Ok(Output::dry_run(format!(
            "Would migrate {} identit{}",
            to_migrate.len(),
            if to_migrate.len() == 1 { "y" } else { "ies" }
        )));
    }

    // Perform migration for each identity
    let mut migrated_count = 0;
    for identity in to_migrate {
        if let Err(e) = migrate_single_identity(identity, ctx) {
            eprintln!("⚠️  Failed to migrate {}: {}", identity.name, e);
            continue;
        }
        migrated_count += 1;
        if !ctx.quiet {
            eprintln!("✓ Migrated {} successfully", identity.name);
        }
    }

    Ok(Output::success(format!(
        "Migrated {} identit{}",
        migrated_count,
        if migrated_count == 1 { "y" } else { "ies" }
    ))
    .with_detail("migrated", migrated_count.to_string()))
}

/// Select identity interactively
fn select_identity_interactive<'a>(
    identities: &'a [&detector::DetectedIdentity],
    ctx: &Context,
) -> Result<Vec<&'a detector::DetectedIdentity>> {
    if identities.len() == 1 {
        // Only one identity, select it automatically
        ctx.info("Found one legacy identity, selecting automatically");
        return Ok(vec![identities[0]]);
    }

    #[cfg(feature = "interactive")]
    {
        use dialoguer::MultiSelect;

        let items: Vec<String> = identities
            .iter()
            .map(|i| {
                if let detector::DetectionSource::SshConfig { host } = &i.source {
                    format!("{} ({})", i.name, host)
                } else {
                    i.name.clone()
                }
            })
            .collect();

        let selections = MultiSelect::new()
            .with_prompt("Select identities to migrate (use spacebar to select, enter to confirm)")
            .items(&items)
            .interact()
            .map_err(|_| Error::Cancelled)?;

        if selections.is_empty() {
            return Err(Error::Cancelled);
        }

        Ok(selections.iter().map(|&i| identities[i]).collect())
    }

    #[cfg(not(feature = "interactive"))]
    {
        Err(Error::InputRequired {
            field: "identity selection (specify --identity or --all)".to_string(),
        })
    }
}

/// Migrate a single identity from gitid-* to gt-*
fn migrate_single_identity(identity: &detector::DetectedIdentity, ctx: &Context) -> Result<()> {
    let ssh_config_path = path::ssh_config_path()?;

    // Load SSH config
    let mut config = SshConfig::load(&ssh_config_path)?;

    // Get the old host entry
    let old_host = if let detector::DetectionSource::SshConfig { host } = &identity.source {
        host.clone()
    } else {
        return Err(Error::IdentityValidation {
            message: "Identity is not SSH-based".to_string(),
        });
    };

    let old_entry = config
        .get_host(&old_host)
        .ok_or_else(|| Error::IdentityNotFound {
            name: identity.name.clone(),
        })?
        .clone();

    // Create new host name: gitid-NAME.provider -> gt-NAME.provider
    let new_host = old_host.replace("gitid-", "gt-");

    ctx.debug(&format!("Renaming SSH host: {} -> {}", old_host, new_host));

    // Rename SSH keys if they exist
    let new_key_path = if let Some(ref key_path) = identity.key_path {
        let key_path_buf = PathBuf::from(key_path);
        let key_path_expanded = path::expand_tilde(&key_path_buf)?;

        if key_path_expanded.exists() {
            // Rename from id_gitid_NAME to id_gt_NAME
            let new_key_name = key_path_expanded
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.replace("id_gitid_", "id_gt_"))
                .ok_or_else(|| Error::IdentityValidation {
                    message: "Invalid key path".to_string(),
                })?;

            let new_key_path = key_path_expanded.parent().unwrap().join(&new_key_name);

            ctx.debug(&format!(
                "Renaming SSH key: {} -> {}",
                key_path_expanded.display(),
                new_key_path.display()
            ));

            // Rename private key
            std::fs::rename(&key_path_expanded, &new_key_path)?;

            // Rename public key if it exists
            let old_pub = key_path_expanded.with_extension("pub");
            if old_pub.exists() {
                let new_pub = new_key_path.with_extension("pub");
                std::fs::rename(&old_pub, &new_pub)?;
            }

            // Return new path with ~ if original had it
            if key_path.starts_with("~/") {
                format!("~/.ssh/{}", new_key_name)
            } else {
                new_key_path.to_string_lossy().to_string()
            }
        } else {
            ctx.debug(&format!(
                "SSH key not found at {}, updating path only",
                key_path
            ));
            key_path.replace("id_gitid_", "id_gt_")
        }
    } else {
        return Err(Error::IdentityValidation {
            message: "No SSH key path found".to_string(),
        });
    };

    // Create new SSH host entry with updated key path
    let mut new_entry = old_entry;
    new_entry.host = new_host.clone();
    new_entry.identity_file = Some(new_key_path);

    // Remove old entry and add new one
    config.remove_host(&old_host);
    config.upsert_host(new_entry);

    // Save SSH config
    config.save(&ssh_config_path)?;

    ctx.debug(&format!("Updated SSH config: {} -> {}", old_host, new_host));

    Ok(())
}

/// Migrate identity strategy (future feature)
fn migrate_strategy(_opts: &MigrateOpts, ctx: &Context) -> Result<Output> {
    if ctx.dry_run {
        return Ok(Output::dry_run("Would migrate strategy"));
    }

    // TODO: Implement strategy migration
    // For now, return error
    Err(Error::StrategyValidation {
        message: "Strategy migration not yet implemented. Use legacy migration (omit --target) to migrate gitid-* to gt-*".to_string(),
    })
}
