//! Implementation of `gt id list` command

use crate::cli::args::ListOpts;
use crate::cli::output::{Output, TableBuilder};
use crate::cmd::Context;
use crate::core::provider::Provider;
use crate::error::Result;
use crate::scan::detector;

/// Execute the list command
pub fn execute(opts: &ListOpts, ctx: &Context) -> Result<Output> {
    // Validate SSH config if requested
    if opts.validate {
        validate_ssh_config(ctx)?;
    }

    let has_config = ctx.has_config();

    // Always detect identities from SSH config
    let detected_identities = detector::detect_identities()?;

    // Show detailed breakdown if requested
    if opts.details {
        show_detailed_breakdown(&detected_identities, ctx);
    }

    if has_config && !opts.all {
        // Show only config-managed identities (default behavior with config)
        let config = ctx.require_config()?;
        let default_id = config.defaults.identity.as_ref();
        list_from_config(&config.identities, default_id, opts)
    } else if has_config && opts.all {
        // Show all identities (config + SSH-only)
        let config = ctx.require_config()?;
        let default_id = config.defaults.identity.as_ref();
        list_merged(&config.identities, &detected_identities, default_id, opts)
    } else {
        // No config file - show auto-detected identities
        ctx.info("No configuration file found. Scanning for existing identities...");

        if detected_identities.is_empty() {
            return Ok(Output::success(
                "No identities found. Use 'gt id add' to create one.",
            ));
        }

        list_from_detected(&detected_identities, opts)
    }
}

/// Validate SSH config and show warnings
fn validate_ssh_config(ctx: &Context) -> Result<()> {
    let ssh_config_path = crate::core::path::ssh_config_path()?;
    if !ssh_config_path.exists() {
        if !ctx.quiet {
            eprintln!("ℹ️  No SSH config found at {}", ssh_config_path.display());
        }
        return Ok(());
    }

    let ssh_config = crate::io::ssh_config::SshConfig::load(&ssh_config_path)?;

    if ssh_config.has_warnings() {
        if !ctx.quiet {
            eprintln!("\n⚠️  SSH Config Warnings:");
            for warning in ssh_config.get_warnings() {
                eprintln!(
                    "  Line {}: {} - {}",
                    warning.line_number, warning.directive, warning.message
                );
            }
            eprintln!();
        }
    } else if !ctx.quiet {
        eprintln!("✓ SSH config is valid\n");
    }

    Ok(())
}

/// Show detailed breakdown by strategy
fn show_detailed_breakdown(identities: &[detector::DetectedIdentity], ctx: &Context) {
    if ctx.quiet {
        return;
    }

    use crate::strategy::StrategyType;
    use std::collections::HashMap;

    let mut by_strategy: HashMap<StrategyType, Vec<&detector::DetectedIdentity>> = HashMap::new();

    for identity in identities {
        by_strategy
            .entry(identity.strategy)
            .or_default()
            .push(identity);
    }

    eprintln!("\nIdentities by Strategy:\n");

    for (strategy, ids) in &by_strategy {
        eprintln!("  {} ({} identities):", strategy, ids.len());
        for id in ids {
            let legacy_note = if id.is_legacy { " [legacy]" } else { "" };
            if let detector::DetectionSource::SshConfig { host } = &id.source {
                eprintln!("    - {} ({}){}", id.name, host, legacy_note);
            } else {
                eprintln!("    - {}{}", id.name, legacy_note);
            }
        }
        eprintln!();
    }
}

/// List identities from configuration file
fn list_from_config(
    identities: &std::collections::HashMap<String, crate::io::toml_config::IdentityConfig>,
    default_id: Option<&String>,
    opts: &ListOpts,
) -> Result<Output> {
    if identities.is_empty() {
        return Ok(Output::success(
            "No identities configured. Use 'gt id add' to create one.",
        ));
    }

    let mut builder = if opts.show_keys {
        TableBuilder::new(vec![
            "name",
            "email",
            "strategy",
            "scope/directory",
            "provider",
            "key",
        ])
    } else {
        TableBuilder::new(vec![
            "name",
            "email",
            "strategy",
            "scope/directory",
            "provider",
        ])
    };

    let mut total_configs = 0;
    let mut sorted_identities: Vec<_> = identities.iter().collect();
    sorted_identities.sort_by_key(|(name, _)| *name);

    for (name, identity) in sorted_identities {
        // Clone identity to migrate without modifying original
        let mut identity_clone = identity.clone();
        identity_clone.migrate_legacy_strategies();

        let is_default = default_id == Some(name);
        let provider_display = Provider::from_name(&identity_clone.provider).to_string();

        // If no strategies, show a single row with "none"
        if identity_clone.strategies.is_empty() {
            let name_display = if is_default {
                format!("{} *", name)
            } else {
                name.clone()
            };

            let mut row = vec![
                name_display,
                identity_clone.email.clone(),
                "none".to_string(),
                "-".to_string(),
                provider_display,
            ];

            if opts.show_keys {
                row.push(
                    identity_clone
                        .ssh
                        .as_ref()
                        .and_then(|s| s.key_path.clone())
                        .unwrap_or_else(|| "none".to_string()),
                );
            }

            builder = builder.row(row);
            total_configs += 1;
        } else {
            // Sort strategies by priority (lower = higher priority)
            let mut strategies = identity_clone.strategies.clone();
            strategies.sort_by_key(|s| s.priority);

            for (idx, strategy) in strategies.iter().enumerate() {
                // Only show name on first row
                let name_display = if idx == 0 {
                    if is_default {
                        format!("{} *", name)
                    } else {
                        name.clone()
                    }
                } else {
                    "".to_string()
                };

                // Get scope/directory based on strategy type
                let scope_display = match strategy.strategy_type.as_str() {
                    "conditional" => strategy
                        .directory
                        .clone()
                        .unwrap_or_else(|| "-".to_string()),
                    "url" => strategy
                        .scope
                        .clone()
                        .unwrap_or_else(|| "(all)".to_string()),
                    "ssh" => {
                        if strategy.use_hostname_alias {
                            "(hostname alias)".to_string()
                        } else {
                            "-".to_string()
                        }
                    }
                    _ => "-".to_string(),
                };

                let mut row = vec![
                    name_display,
                    if idx == 0 {
                        identity_clone.email.clone()
                    } else {
                        "".to_string()
                    },
                    strategy.strategy_type.clone(),
                    scope_display,
                    if idx == 0 {
                        provider_display.clone()
                    } else {
                        "".to_string()
                    },
                ];

                if opts.show_keys {
                    row.push(if idx == 0 {
                        identity_clone
                            .ssh
                            .as_ref()
                            .and_then(|s| s.key_path.clone())
                            .unwrap_or_else(|| "none".to_string())
                    } else {
                        "".to_string()
                    });
                }

                builder = builder.row(row);
                total_configs += 1;
            }
        }
    }

    let summary = if total_configs == identities.len() {
        format!("Found {} identities", identities.len())
    } else {
        format!(
            "Found {} identities ({} configurations)",
            identities.len(),
            total_configs
        )
    };

    Ok(builder.build(summary))
}

/// List identities from auto-detection
fn list_from_detected(
    identities: &[detector::DetectedIdentity],
    opts: &ListOpts,
) -> Result<Output> {
    let mut builder = if opts.show_keys {
        TableBuilder::new(vec![
            "name", "provider", "email", "strategy", "key", "source",
        ])
    } else {
        TableBuilder::new(vec!["name", "provider", "email", "strategy", "source"])
    };

    for identity in identities {
        let provider = identity
            .provider
            .as_ref()
            .map(|p| p.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let email = identity.email.clone().unwrap_or_else(|| "-".to_string());

        let source = match &identity.source {
            detector::DetectionSource::SshConfig { host } => format!("ssh:{}", host),
            detector::DetectionSource::GitConditional { condition, .. } => {
                format!("git:{}", condition)
            }
            detector::DetectionSource::GitUrlRewrite { .. } => "git:rewrite".to_string(),
            detector::DetectionSource::RepoUrl { .. } => "repo".to_string(),
        };

        let name_display = if identity.is_legacy {
            format!("{} (legacy)", identity.name)
        } else {
            identity.name.clone()
        };

        let mut row = vec![
            name_display,
            provider,
            email,
            identity.strategy.to_string(),
            source,
        ];

        if opts.show_keys {
            row.insert(
                4,
                identity.key_path.clone().unwrap_or_else(|| "-".to_string()),
            );
        }

        builder = builder.row(row);
    }

    Ok(builder.build(format!(
        "Found {} identities (auto-detected)",
        identities.len()
    )))
}

/// List identities merged from config and auto-detection
fn list_merged(
    config_identities: &std::collections::HashMap<String, crate::io::toml_config::IdentityConfig>,
    detected_identities: &[detector::DetectedIdentity],
    default_id: Option<&String>,
    opts: &ListOpts,
) -> Result<Output> {
    use std::collections::HashSet;

    let mut builder = if opts.show_keys {
        TableBuilder::new(vec![
            "name",
            "email",
            "strategy",
            "scope/directory",
            "provider",
            "key",
            "source",
        ])
    } else {
        TableBuilder::new(vec![
            "name",
            "email",
            "strategy",
            "scope/directory",
            "provider",
            "source",
        ])
    };

    let mut seen_names = HashSet::new();
    let mut unmanaged_count = 0;
    let mut total_configs = 0;
    let mut sorted_identities: Vec<_> = config_identities.iter().collect();
    sorted_identities.sort_by_key(|(name, _)| *name);

    // First, add config-managed identities
    for (name, identity) in sorted_identities {
        // Clone identity to migrate without modifying original
        let mut identity_clone = identity.clone();
        identity_clone.migrate_legacy_strategies();

        let is_default = default_id == Some(name);
        let provider_display = Provider::from_name(&identity_clone.provider).to_string();

        // If no strategies, show a single row with "none"
        if identity_clone.strategies.is_empty() {
            let name_display = if is_default {
                format!("{} *", name)
            } else {
                name.clone()
            };

            let mut row = vec![
                name_display,
                identity_clone.email.clone(),
                "none".to_string(),
                "-".to_string(),
                provider_display,
                "config".to_string(),
            ];

            if opts.show_keys {
                row.insert(
                    5,
                    identity_clone
                        .ssh
                        .as_ref()
                        .and_then(|s| s.key_path.clone())
                        .unwrap_or_else(|| "none".to_string()),
                );
            }

            builder = builder.row(row);
            total_configs += 1;
        } else {
            // Sort strategies by priority (lower = higher priority)
            let mut strategies = identity_clone.strategies.clone();
            strategies.sort_by_key(|s| s.priority);

            for (idx, strategy) in strategies.iter().enumerate() {
                // Only show name on first row
                let name_display = if idx == 0 {
                    if is_default {
                        format!("{} *", name)
                    } else {
                        name.clone()
                    }
                } else {
                    "".to_string()
                };

                // Get scope/directory based on strategy type
                let scope_display = match strategy.strategy_type.as_str() {
                    "conditional" => strategy
                        .directory
                        .clone()
                        .unwrap_or_else(|| "-".to_string()),
                    "url" => strategy
                        .scope
                        .clone()
                        .unwrap_or_else(|| "(all)".to_string()),
                    "ssh" => {
                        if strategy.use_hostname_alias {
                            "(hostname alias)".to_string()
                        } else {
                            "-".to_string()
                        }
                    }
                    _ => "-".to_string(),
                };

                let mut row = vec![
                    name_display,
                    if idx == 0 {
                        identity_clone.email.clone()
                    } else {
                        "".to_string()
                    },
                    strategy.strategy_type.clone(),
                    scope_display,
                    if idx == 0 {
                        provider_display.clone()
                    } else {
                        "".to_string()
                    },
                    if idx == 0 {
                        "config".to_string()
                    } else {
                        "".to_string()
                    },
                ];

                if opts.show_keys {
                    row.insert(
                        5,
                        if idx == 0 {
                            identity_clone
                                .ssh
                                .as_ref()
                                .and_then(|s| s.key_path.clone())
                                .unwrap_or_else(|| format!("~/.ssh/id_gt_{}", name))
                        } else {
                            "".to_string()
                        },
                    );
                }

                builder = builder.row(row);
                total_configs += 1;
            }
        }

        seen_names.insert(name.clone());
    }

    // Then, add SSH-only identities (not in config)
    for identity in detected_identities {
        if seen_names.contains(&identity.name) {
            continue; // Skip if already in config
        }

        let provider = identity
            .provider
            .as_ref()
            .map(|p| p.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let email = identity.email.clone().unwrap_or_else(|| "-".to_string());

        let source = match &identity.source {
            detector::DetectionSource::SshConfig { host } => format!("ssh:{}", host),
            detector::DetectionSource::GitConditional { condition, .. } => {
                format!("git:{}", condition)
            }
            detector::DetectionSource::GitUrlRewrite { .. } => "git:rewrite".to_string(),
            detector::DetectionSource::RepoUrl { .. } => "repo".to_string(),
        };

        let name_display = if identity.is_legacy {
            format!("{} (legacy)", identity.name)
        } else {
            format!("{} (unmanaged)", identity.name)
        };

        let mut row = vec![
            name_display,
            email,
            identity.strategy.to_string(),
            "-".to_string(), // unmanaged identities don't have scope/directory
            provider,
            source,
        ];

        if opts.show_keys {
            row.insert(
                5,
                identity.key_path.clone().unwrap_or_else(|| "-".to_string()),
            );
        }

        builder = builder.row(row);
        unmanaged_count += 1;
        total_configs += 1;
    }

    let total_identities = config_identities.len() + unmanaged_count;
    Ok(builder.build(format!(
        "Found {} identities ({} configurations, {} managed, {} unmanaged)",
        total_identities,
        total_configs,
        config_identities.len(),
        unmanaged_count
    )))
}
