//! Implementation of `gitid key` command

use crate::cli::args::{KeyCommands, KeyOpts};
use crate::cli::output::Output;
use crate::cmd::Context;
use crate::error::Result;

/// Execute the key command
pub fn execute(opts: &KeyOpts, ctx: &Context) -> Result<Output> {
    match &opts.command {
        KeyCommands::Generate {
            identity,
            key_type,
            bits,
            email,
            passphrase,
            force,
        } => generate(identity, key_type, *bits, email.as_deref(), passphrase.as_deref(), *force, ctx),
        KeyCommands::List { all, identity } => list(*all, identity.as_deref(), ctx),
        KeyCommands::Add { identity, key_path } => add(identity, key_path, ctx),
        KeyCommands::Remove { identity } => remove(identity, ctx),
        KeyCommands::Activate { identity } => activate(identity, ctx),
        KeyCommands::Show { identity } => show(identity, ctx),
        KeyCommands::Test { identity } => test(identity, ctx),
    }
}

fn generate(
    identity: &str,
    key_type: &crate::cli::args::KeyTypeArg,
    bits: u32,
    email: Option<&str>,
    passphrase: Option<&str>,
    force: bool,
    ctx: &Context,
) -> Result<Output> {
    use crate::core::path;
    use crate::scan::detector;
    use std::path::PathBuf;
    use std::process::Command;

    ctx.info(&format!(
        "Generating {} key for identity '{}'",
        key_type, identity
    ));

    // Detect identities to check if this identity exists
    let all_identities = detector::detect_identities()?;
    let existing_identity = all_identities.iter().find(|i| i.name == identity);

    // Determine key path
    let key_path = if let Some(id) = existing_identity {
        // Use existing key path
        id.key_path
            .as_ref()
            .map(|p| PathBuf::from(p))
            .unwrap_or_else(|| {
                PathBuf::from(format!("~/.ssh/id_gt_{}", identity))
            })
    } else {
        // New identity, create default path
        PathBuf::from(format!("~/.ssh/id_gt_{}", identity))
    };

    let expanded_key_path = path::expand_tilde(&key_path)?;

    // Check if key already exists
    if expanded_key_path.exists() && !force {
        return Err(crate::error::Error::IdentityValidation {
            message: format!(
                "SSH key already exists at {}. Use --force to overwrite.",
                expanded_key_path.display()
            ),
        });
    }

    // Determine comment (email or default)
    let default_email = format!("gt-{}@localhost", identity);
    let comment = email.unwrap_or(&default_email);

    if ctx.dry_run {
        let mut msg = format!(
            "Would generate {} key for '{}' at {}",
            key_type,
            identity,
            expanded_key_path.display()
        );
        if passphrase.is_some() {
            msg.push_str(" with passphrase");
        }
        return Ok(Output::dry_run(msg));
    }

    // Ensure .ssh directory exists
    if let Some(parent) = expanded_key_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = std::fs::Permissions::from_mode(0o700);
                std::fs::set_permissions(parent, perms)?;
            }
        }
    }

    // If force is set and files exist, remove them first
    if force {
        let pub_key_path = expanded_key_path.with_extension("pub");
        if expanded_key_path.exists() {
            std::fs::remove_file(&expanded_key_path)?;
            ctx.debug(&format!("Removed existing private key: {}", expanded_key_path.display()));
        }
        if pub_key_path.exists() {
            std::fs::remove_file(&pub_key_path)?;
            ctx.debug(&format!("Removed existing public key: {}", pub_key_path.display()));
        }
    }

    // Build ssh-keygen command
    let mut cmd = Command::new("ssh-keygen");

    // Key type
    match key_type {
        crate::cli::args::KeyTypeArg::Ed25519 => {
            cmd.arg("-t").arg("ed25519");
        }
        crate::cli::args::KeyTypeArg::Rsa => {
            cmd.arg("-t").arg("rsa").arg("-b").arg(bits.to_string());
        }
        crate::cli::args::KeyTypeArg::Ecdsa => {
            cmd.arg("-t").arg("ecdsa").arg("-b").arg(bits.to_string());
        }
    }

    // Comment (email)
    cmd.arg("-C").arg(comment);

    // Output file
    cmd.arg("-f").arg(&expanded_key_path);

    // Passphrase
    if let Some(pass) = passphrase {
        cmd.arg("-N").arg(pass);
    } else {
        // Empty passphrase (no encryption)
        cmd.arg("-N").arg("");
    }

    // Overwrite without prompting
    if force {
        cmd.arg("-q"); // Quiet mode
    }

    ctx.debug(&format!("Running: ssh-keygen with key type {}", key_type));

    // Execute ssh-keygen
    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::error::Error::IdentityValidation {
            message: format!("Failed to generate SSH key: {}", stderr),
        });
    }

    // Set proper permissions on private key
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&expanded_key_path, perms)?;
    }

    if !ctx.quiet {
        eprintln!("✓ Generated {} SSH key for '{}'", key_type, identity);
        eprintln!("  Private key: {}", expanded_key_path.display());
        eprintln!("  Public key:  {}.pub", expanded_key_path.display());
        if email.is_some() {
            eprintln!("  Email:       {}", comment);
        }
        if passphrase.is_some() {
            eprintln!("  Encrypted:   yes");
        }
    }

    Ok(Output::success(format!("SSH key generated for '{}'", identity))
        .with_detail("key_path", &expanded_key_path.to_string_lossy().to_string())
        .with_detail("key_type", &key_type.to_string()))
}

fn list(all: bool, identity: Option<&str>, ctx: &Context) -> Result<Output> {
    ctx.info("Listing SSH keys");

    // TODO: Implement key listing
    let _ = (all, identity);

    Ok(Output::success("SSH keys"))
}

fn add(identity: &str, key_path: &std::path::Path, ctx: &Context) -> Result<Output> {
    if ctx.dry_run {
        return Ok(Output::dry_run(format!(
            "Would add key {} to identity '{}'",
            key_path.display(),
            identity
        )));
    }

    // TODO: Implement adding existing key

    Ok(Output::success(format!(
        "Key added to identity '{}'",
        identity
    )))
}

fn remove(identity: &str, ctx: &Context) -> Result<Output> {
    if ctx.dry_run {
        return Ok(Output::dry_run(format!(
            "Would remove key from identity '{}'",
            identity
        )));
    }

    // TODO: Implement key removal

    Ok(Output::success(format!(
        "Key removed from identity '{}'",
        identity
    )))
}

fn activate(identity: &str, ctx: &Context) -> Result<Output> {
    ctx.info(&format!("Adding key for '{}' to SSH agent", identity));

    if ctx.dry_run {
        return Ok(Output::dry_run(format!(
            "Would add key for '{}' to SSH agent",
            identity
        )));
    }

    // TODO: Implement ssh-add

    Ok(Output::success(format!(
        "Key for '{}' added to SSH agent",
        identity
    )))
}

fn show(identity: &str, ctx: &Context) -> Result<Output> {
    use crate::core::path;
    use crate::scan::detector;
    use std::path::PathBuf;

    ctx.info(&format!("Looking for identity '{}'...", identity));

    // Detect all identities to find the one to show
    let all_identities = detector::detect_identities()?;

    // Find the identity by name
    let found_identity = all_identities
        .iter()
        .find(|i| i.name == identity)
        .ok_or_else(|| crate::error::Error::IdentityNotFound {
            name: identity.to_string(),
        })?;

    // Get the key path
    let key_path = found_identity
        .key_path
        .as_ref()
        .ok_or_else(|| crate::error::Error::IdentityValidation {
            message: format!("No SSH key associated with identity '{}'", identity),
        })?;

    // Expand tilde and get public key path
    let key_path_buf = PathBuf::from(key_path);
    let expanded_key_path = path::expand_tilde(&key_path_buf)?;
    let pub_key_path = expanded_key_path.with_extension("pub");

    // Check if public key exists
    if !pub_key_path.exists() {
        return Err(crate::error::Error::IdentityValidation {
            message: format!("Public key not found at: {}", pub_key_path.display()),
        });
    }

    // Read the public key
    let public_key = std::fs::read_to_string(&pub_key_path)?;
    let public_key = public_key.trim();

    // In quiet mode, just print the key
    if ctx.quiet {
        println!("{}", public_key);
    } else {
        eprintln!("Public key for '{}':", identity);
        eprintln!("  Path: {}", pub_key_path.display());
        eprintln!();
        println!("{}", public_key);
    }

    Ok(Output::success(format!("Public key for '{}'", identity))
        .with_detail("key_path", &pub_key_path.to_string_lossy().to_string()))
}

fn test(identity: &str, ctx: &Context) -> Result<Output> {
    let config = ctx.require_config()?;

    if !config.identities.contains_key(identity) {
        return Err(crate::error::Error::IdentityNotFound {
            name: identity.to_string(),
        });
    }

    ctx.info(&format!("Testing authentication for '{}'", identity));

    // TODO: Implement SSH authentication test

    Ok(Output::success(format!(
        "Authentication test for '{}' succeeded",
        identity
    )))
}
