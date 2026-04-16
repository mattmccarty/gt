//! Implementation of `gt id import` command

use crate::cli::args::ImportOpts;
use crate::cli::output::Output;
use crate::cmd::Context;
use crate::error::{Error, Result};
use crate::io::toml_config::{IdentityConfig, IdentitySshConfig};
use crate::scan::detector;
use crate::strategy::StrategyType;

/// Execute the import command
pub fn execute(opts: &ImportOpts, ctx: &Context) -> Result<Output> {
    ctx.info(&format!("Importing identity '{}'...", opts.name));

    // Load or create config
    let mut config = ctx.config.clone().unwrap_or_default();

    // Check if identity already exists in config
    if config.identities.contains_key(&opts.name) {
        return Err(Error::IdentityExists {
            name: opts.name.clone(),
        });
    }

    // Detect all identities from SSH config
    let detected_identities = detector::detect_identities()?;

    // Find the identity to import
    let detected = detected_identities
        .iter()
        .find(|i| i.name == opts.name)
        .ok_or_else(|| Error::IdentityNotFound {
            name: opts.name.clone(),
        })?;

    ctx.debug(&format!("Found identity in SSH config: {:?}", detected));

    // Determine email (CLI arg > detected > GitHub default)
    let email = opts
        .email
        .clone()
        .or_else(|| detected.email.clone())
        .unwrap_or_else(|| {
            // Default based on provider
            let is_github = opts
                .provider
                .as_ref()
                .map(|p| p.to_lowercase() == "github")
                .unwrap_or_else(|| {
                    detected.provider.as_ref().map_or(true, |p| {
                        matches!(p, crate::core::provider::Provider::GitHub)
                    })
                });

            if is_github {
                format!("{}@users.noreply.github.com", opts.name)
            } else {
                format!("{}@localhost", opts.name)
            }
        });

    // Determine user name (CLI arg > detected > identity name)
    let user_name = opts
        .user_name
        .clone()
        .unwrap_or_else(|| opts.name.clone());

    // Determine provider (CLI arg > detected > default)
    let provider = opts
        .provider
        .clone()
        .or_else(|| {
            detected.provider.as_ref().map(|p| match p {
                crate::core::provider::Provider::GitHub => "github".to_string(),
                crate::core::provider::Provider::GitLab => "gitlab".to_string(),
                crate::core::provider::Provider::Bitbucket => "bitbucket".to_string(),
                crate::core::provider::Provider::Azure => "azure".to_string(),
                crate::core::provider::Provider::CodeCommit => "codecommit".to_string(),
                crate::core::provider::Provider::Custom(c) => c.hostname.clone(),
            })
        })
        .unwrap_or_else(|| "github".to_string());

    // Determine strategy (CLI arg > detected)
    let strategy = opts
        .strategy
        .as_ref()
        .and_then(|s| StrategyType::from_str(&s.to_string()))
        .unwrap_or(detected.strategy);

    if ctx.dry_run {
        return Ok(Output::dry_run(format!(
            "Would import identity '{}' from SSH config",
            opts.name
        ))
        .with_detail("email", &email)
        .with_detail("user_name", &user_name)
        .with_detail("provider", &provider)
        .with_detail("strategy", &strategy.to_string()));
    }

    // Get SSH key path from detected identity
    let ssh_config = detected.key_path.as_ref().map(|key_path| {
        IdentitySshConfig {
            key_path: Some(key_path.clone()),
            key_type: None, // Will be auto-detected when needed
            use_hostname_alias: false, // Default to no URL transformation for imported identities
        }
    });

    // Create identity config
    let identity_config = IdentityConfig {
        email: email.clone(),
        name: user_name.clone(),
        provider: provider.clone(),
        strategy: Some(strategy.to_string()),
        ssh: ssh_config,
        conditional: None,
        url_rewrite: None,
        strategies: vec![], // Will be migrated on first load
    };

    // Add to config
    config.set_identity(opts.name.clone(), identity_config);

    // Set as default identity if it's the first one
    if config.identities.len() == 1 && config.defaults.identity.is_none() {
        config.defaults.identity = Some(opts.name.clone());
        ctx.info(&format!("Set '{}' as the default identity", opts.name));
    }

    // Save config
    config.save(&ctx.config_path)?;

    ctx.info(&format!(
        "Configuration saved to {}",
        ctx.config_path.display()
    ));

    let mut output = Output::success(format!("Identity '{}' imported successfully", opts.name))
        .with_detail("email", &email)
        .with_detail("user_name", &user_name)
        .with_detail("provider", &provider)
        .with_detail("strategy", &strategy.to_string());

    if let Some(key_path) = &detected.key_path {
        output = output.with_detail("ssh_key", key_path);
    }

    if let detector::DetectionSource::SshConfig { host } = &detected.source {
        output = output.with_detail("ssh_host", host);
    }

    if !ctx.quiet {
        eprintln!("✓ Imported identity '{}'", opts.name);
        eprintln!("  Email:    {}", email);
        eprintln!("  User:     {}", user_name);
        eprintln!("  Provider: {}", provider);
        eprintln!("  Strategy: {}", strategy);
        if let Some(key_path) = &detected.key_path {
            eprintln!("  SSH Key:  {}", key_path);
        }
    }

    Ok(output)
}
