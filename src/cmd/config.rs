//! Implementation of `gt config` command

use crate::cli::args::{ConfigCommands, ConfigIdCommands, ConfigOpts};
use crate::cli::output::Output;
use crate::cmd::Context;
use crate::error::{Error, Result};
use crate::io::active_id::ActiveIdentity;

/// Execute the config command
pub fn execute(opts: &ConfigOpts, ctx: &Context) -> Result<Output> {
    match &opts.command {
        Some(ConfigCommands::Validate) => validate_config(ctx),
        Some(ConfigCommands::Id(id_opts)) => {
            match &id_opts.command {
                Some(ConfigIdCommands::Add(opts)) => crate::cmd::add::execute(opts, ctx),
                Some(ConfigIdCommands::Import(opts)) => crate::cmd::import::execute(opts, ctx),
                Some(ConfigIdCommands::List(opts)) => crate::cmd::list::execute(opts, ctx),
                Some(ConfigIdCommands::Use(opts)) => crate::cmd::use_::execute(opts, ctx),
                Some(ConfigIdCommands::Migrate(opts)) => crate::cmd::migrate::execute(opts, ctx),
                Some(ConfigIdCommands::Key(opts)) => crate::cmd::key::execute(opts, ctx),
                Some(ConfigIdCommands::Status(opts)) => crate::cmd::status::execute(opts, ctx),
                Some(ConfigIdCommands::Delete(opts)) => crate::cmd::delete::execute(opts, ctx),
                Some(ConfigIdCommands::Update(opts)) => crate::cmd::update::execute(opts, ctx),
                Some(ConfigIdCommands::Fix(opts)) => crate::cmd::fix::execute_id(opts, ctx),
                Some(ConfigIdCommands::Default { name }) => {
                    match name {
                        Some(id_name) => set_default_identity(id_name, ctx),
                        None => get_default_identity(ctx),
                    }
                }
                None => {
                    // `gt config id` - show identity configuration
                    show_identity_config(ctx)
                }
            }
        }
        None => show_config_summary(ctx),
    }
}

fn show_config_summary(ctx: &Context) -> Result<Output> {
    let config = ctx.require_config()?;

    Ok(Output::success("Configuration")
        .with_detail(
            "default.identity",
            config
                .defaults
                .identity
                .as_ref()
                .unwrap_or(&"none".to_string()),
        )
        .with_detail(
            "default.strategy",
            config
                .defaults
                .strategy
                .as_ref()
                .unwrap_or(&"ssh-alias".to_string()),
        )
        .with_detail("identities", config.identities.len().to_string()))
}

fn validate_config(ctx: &Context) -> Result<Output> {
    let _config = ctx.require_config()?;
    // TODO: Implement validation
    Ok(Output::success("Configuration is valid"))
}

fn show_identity_config(ctx: &Context) -> Result<Output> {
    let config = ctx.require_config()?;

    let default_identity = config
        .defaults
        .identity
        .as_ref()
        .map_or("not set".to_string(), |id| id.clone());

    let active_identity = ActiveIdentity::load()
        .ok()
        .flatten()
        .map(|a| a.identity)
        .unwrap_or_else(|| "not set".to_string());

    Ok(Output::success("Identity Configuration")
        .with_detail("default", &default_identity)
        .with_detail("active", &active_identity)
        .with_detail("count", &config.identities.len().to_string()))
}

fn get_default_identity(ctx: &Context) -> Result<Output> {
    let config = ctx.require_config()?;

    let default_identity = config
        .defaults
        .identity
        .clone()
        .unwrap_or_else(|| "not set".to_string());

    Ok(Output::success(default_identity))
}

fn set_default_identity(identity_name: &str, ctx: &Context) -> Result<Output> {
    if ctx.dry_run {
        return Ok(Output::dry_run(format!(
            "Would set default identity to '{}'",
            identity_name
        )));
    }

    let mut config = ctx.require_config()?.clone();

    // Verify the identity exists
    if !config.identities.contains_key(identity_name) {
        return Err(Error::IdentityNotFound {
            name: identity_name.to_string(),
        });
    }

    config.defaults.identity = Some(identity_name.to_string());
    config.save(&ctx.config_path)?;

    if !ctx.quiet {
        eprintln!("✓ Set '{}' as the default identity", identity_name);
    }

    Ok(Output::success(format!(
        "Set default identity to '{}'",
        identity_name
    ))
    .with_detail("default.identity", identity_name))
}
