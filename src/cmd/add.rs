//! Implementation of `gt id add` command (refactored for multi-strategy)

use crate::cli::args::AddOpts;
use crate::cli::output::Output;
use crate::cmd::Context;
use crate::core::identity::{Identity, SshConfig};
use crate::core::provider::Provider;
use crate::error::{Error, Result};
use crate::io::toml_config::{IdentityConfig, IdentitySshConfig, StrategyConfig};
use crate::strategy::StrategyType;
use crate::util::validate_identity_name;

/// Execute the add command
pub fn execute(opts: &AddOpts, ctx: &Context) -> Result<Output> {
    // Validate identity name
    validate_identity_name(&opts.name)?;

    // Load or create config
    let mut config = ctx.config.clone().unwrap_or_default();

    // Determine strategy type
    let strategy_type = opts
        .strategy
        .as_ref()
        .and_then(|s| StrategyType::from_str(&s.to_string()))
        .unwrap_or(StrategyType::SshAlias);

    let strategy_type_str = strategy_type.to_string();

    // Check if identity already exists
    let identity_exists = config.identities.contains_key(&opts.name);

    if identity_exists {
        ctx.info(&format!(
            "Identity '{}' exists, adding {} strategy variant",
            opts.name, strategy_type_str
        ));

        // Add strategy variant to existing identity
        add_strategy_variant(opts, ctx, &mut config, strategy_type)
    } else {
        ctx.info(&format!(
            "Creating new identity '{}' with {} strategy",
            opts.name, strategy_type_str
        ));

        // Create new identity
        create_new_identity(opts, ctx, &mut config, strategy_type)
    }
}

/// Create a new identity with its first strategy
fn create_new_identity(
    opts: &AddOpts,
    ctx: &Context,
    config: &mut crate::io::toml_config::GtConfig,
    strategy_type: StrategyType,
) -> Result<Output> {
    let provider = Provider::from_name(&opts.provider);
    let strategy_type_str = strategy_type.to_string();

    // Email is optional - defaults based on provider
    let email = opts.email.clone().unwrap_or_else(|| {
        if opts.provider.to_lowercase() == "github" {
            format!("{}@users.noreply.github.com", opts.name)
        } else {
            format!("{}@localhost", opts.name)
        }
    });

    // User name is optional - defaults to identity name
    let user_name = opts.user_name.clone().unwrap_or_else(|| opts.name.clone());

    if ctx.dry_run {
        return Ok(Output::dry_run(format!(
            "Would create identity '{}' with {} strategy",
            opts.name, strategy_type_str
        ))
        .with_detail("email", &email)
        .with_detail("user_name", &user_name));
    }

    // Create identity (for SSH key setup)
    let mut identity_builder = Identity::builder(&opts.name)
        .email(&email)
        .name(&user_name)
        .provider(provider);

    // Add SSH configuration if not --no-key
    let ssh_key_path = if !opts.no_key {
        let ssh_config = if let Some(ref key_path) = opts.key {
            SshConfig {
                key_path: Some(key_path.to_string_lossy().to_string()),
                key_type: Some(opts.key_type.to_string()),
                key_bits: None,
            }
        } else {
            SshConfig {
                key_path: None, // Will use default
                key_type: Some(opts.key_type.to_string()),
                key_bits: match opts.key_type.to_string().as_str() {
                    "rsa" => Some(4096),
                    "ecdsa" => Some(521),
                    _ => None,
                },
            }
        };
        identity_builder = identity_builder.ssh_config(ssh_config);

        let identity = identity_builder.build()?;

        // Setup SSH infrastructure (key generation, SSH config)
        use crate::strategy::ssh_alias::SshAliasStrategy;
        let ssh_strategy = SshAliasStrategy::new();
        let (key_path, newly_created) = ssh_strategy.setup_identity(&identity, ctx.force)?;

        if newly_created {
            ctx.info(&format!("Generated SSH key: {}", key_path));
        } else {
            ctx.info(&format!("Using existing SSH key: {}", key_path));
        }

        Some(key_path)
    } else {
        None
    };

    // Create the strategy configuration
    let strategy_config = create_strategy_config(opts, strategy_type, &strategy_type_str)?;

    // Create identity config
    let identity_config = IdentityConfig {
        email: email.clone(),
        name: user_name.clone(),
        provider: opts.provider.clone(),
        strategy: None, // New format doesn't use this
        ssh: ssh_key_path.as_ref().map(|path| IdentitySshConfig {
            key_path: Some(path.clone()),
            key_type: Some(opts.key_type.to_string()),
            use_hostname_alias: false, // Controlled by strategy config
        }),
        conditional: None,
        url_rewrite: None,
        strategies: vec![strategy_config],
    };

    // Setup strategy-specific infrastructure
    let mut output = setup_strategy_infrastructure(opts, ctx, &email, &user_name, strategy_type, ssh_key_path.as_deref())?;

    config.set_identity(opts.name.clone(), identity_config);

    // Set as default identity if it's the first one
    if config.identities.len() == 1 && config.defaults.identity.is_none() {
        config.defaults.identity = Some(opts.name.clone());
        ctx.info(&format!("Set '{}' as the default identity", opts.name));
    }

    // Save config to file
    config.save(&ctx.config_path)?;

    ctx.info(&format!(
        "Configuration saved to {}",
        ctx.config_path.display()
    ));

    output = output
        .with_detail("identity", &opts.name)
        .with_detail("email", &email)
        .with_detail("strategy", &strategy_type_str);

    Ok(output)
}

/// Add a strategy variant to an existing identity
fn add_strategy_variant(
    opts: &AddOpts,
    ctx: &Context,
    config: &mut crate::io::toml_config::GtConfig,
    strategy_type: StrategyType,
) -> Result<Output> {
    let strategy_type_str = strategy_type.to_string();

    // Get existing identity
    let identity_config = config
        .identities
        .get_mut(&opts.name)
        .ok_or_else(|| Error::IdentityNotFound {
            name: opts.name.clone(),
        })?;

    // Migrate legacy strategies if needed
    identity_config.migrate_legacy_strategies();

    if ctx.dry_run {
        return Ok(Output::dry_run(format!(
            "Would add {} strategy to identity '{}'",
            strategy_type_str, opts.name
        )));
    }

    // Create the strategy configuration
    let strategy_config = create_strategy_config(opts, strategy_type, &strategy_type_str)?;

    // Check if this exact strategy variant already exists
    let discriminator = match strategy_type {
        StrategyType::Conditional => opts.directory.clone(),
        StrategyType::UrlRewrite => opts.scope.clone(),
        _ => None,
    };

    let already_exists = identity_config
        .find_strategy_variant(&strategy_type_str, discriminator.as_deref())
        .is_some();

    if already_exists {
        ctx.info(&format!(
            "Strategy variant already exists, replacing with new configuration"
        ));
    }

    // Add the strategy (will replace if discriminator matches)
    identity_config.add_strategy(strategy_config);

    // Setup strategy-specific infrastructure
    let ssh_key_path = identity_config
        .ssh
        .as_ref()
        .and_then(|s| s.key_path.as_deref());

    let mut output = setup_strategy_infrastructure(
        opts,
        ctx,
        &identity_config.email,
        &identity_config.name,
        strategy_type,
        ssh_key_path,
    )?;

    // Save config
    config.save(&ctx.config_path)?;

    ctx.info(&format!(
        "Configuration saved to {}",
        ctx.config_path.display()
    ));

    output = output
        .with_detail("identity", &opts.name)
        .with_detail("strategy", &strategy_type_str);

    Ok(output)
}

/// Create a StrategyConfig based on options
fn create_strategy_config(
    opts: &AddOpts,
    strategy_type: StrategyType,
    strategy_type_str: &str,
) -> Result<StrategyConfig> {
    let mut strategy_config = StrategyConfig {
        strategy_type: strategy_type_str.to_string(),
        priority: StrategyConfig::default_priority_for_type(strategy_type_str),
        enabled: true,
        use_hostname_alias: false,
        directory: None,
        scope: None,
        patterns: None,
    };

    // Set strategy-specific fields
    match strategy_type {
        StrategyType::SshAlias => {
            strategy_config.use_hostname_alias = true;
        }
        StrategyType::Conditional => {
            strategy_config.directory = opts.directory.clone();
            if strategy_config.directory.is_none() {
                return Err(Error::ConfigInvalid {
                    message: "Conditional strategy requires --directory flag".to_string(),
                });
            }
        }
        StrategyType::UrlRewrite => {
            strategy_config.scope = opts.scope.clone();
            // Scope is optional for URL rewrite
        }
    }

    Ok(strategy_config)
}

/// Setup strategy-specific infrastructure (git config, SSH config, etc.)
fn setup_strategy_infrastructure(
    opts: &AddOpts,
    ctx: &Context,
    email: &str,
    user_name: &str,
    strategy_type: StrategyType,
    ssh_key_path: Option<&str>,
) -> Result<Output> {
    use crate::io::git_config;

    let mut output = Output::success(format!(
        "Added {} strategy successfully",
        strategy_type
    ));

    match strategy_type {
        StrategyType::SshAlias => {
            if let Some(key_path) = ssh_key_path {
                ctx.info(&format!(
                    "SSH hostname aliasing enabled for {}",
                    opts.name
                ));
                output = output.with_detail("ssh_key", key_path);
            }
        }
        StrategyType::Conditional => {
            if let Some(ref directory) = opts.directory {
                // Normalize directory path - ensure it ends with /
                let mut normalized_dir = directory.trim().to_string();
                if !normalized_dir.ends_with('/') {
                    normalized_dir.push('/');
                }

                let condition = format!("gitdir:{}", normalized_dir);
                let include_path = format!(
                    "{}/.gitconfig.d/{}",
                    dirs::home_dir().unwrap().display(),
                    opts.name
                );

                // Create the include file
                git_config::write_include_file(
                    std::path::Path::new(&include_path),
                    email,
                    user_name,
                    ssh_key_path,
                )?;

                // Add conditional include to global gitconfig
                git_config::add_conditional_include(&condition, &include_path)?;

                ctx.info(&format!(
                    "Added conditional include: {} → {}",
                    condition, include_path
                ));

                output = output
                    .with_detail("conditional_directory", directory)
                    .with_detail("include_file", &include_path);
            }
        }
        StrategyType::UrlRewrite => {
            if let Some(ref scope) = opts.scope {
                let provider = Provider::from_name(&opts.provider);
                let provider_host = provider.hostname();

                let original_url = format!("git@{}:{}/", provider_host, scope);
                let ssh_host = if ssh_key_path.is_some() {
                    format!("gt-{}.{}", opts.name, provider_host)
                } else {
                    provider_host.to_string()
                };
                let rewrite_url = format!("git@{}:{}/", ssh_host, scope);

                git_config::add_url_rewrite(&original_url, &rewrite_url)?;

                ctx.info(&format!(
                    "Added Git URL rewrite: {} → {}",
                    original_url, rewrite_url
                ));

                output = output
                    .with_detail("url_scope", scope)
                    .with_detail("url_rewrite_from", &original_url)
                    .with_detail("url_rewrite_to", &rewrite_url);
            }
        }
    }

    Ok(output)
}
