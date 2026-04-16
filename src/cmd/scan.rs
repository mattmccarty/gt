//! Implementation of `gt id scan` command

use crate::cli::args::ScanOpts;
use crate::cli::output::Output;
use crate::cmd::Context;
use crate::error::Result;
use crate::scan::detector;

/// Execute the scan command
pub fn execute(opts: &ScanOpts, ctx: &Context) -> Result<Output> {
    ctx.info("Scanning for existing identity configurations...");

    let mut output = Output::success("Scan complete");

    // Check SSH config for parse errors/warnings first
    if opts.ssh_only || !opts.git_only {
        let ssh_config_path = crate::core::path::ssh_config_path()?;
        if ssh_config_path.exists() {
            let ssh_config = crate::io::ssh_config::SshConfig::load(&ssh_config_path)?;

            if ssh_config.has_warnings() {
                if !ctx.quiet {
                    eprintln!("\n⚠️  SSH Config Warnings:");
                    for warning in ssh_config.get_warnings() {
                        eprintln!(
                            "  Line {}: {} - {}",
                            warning.line_number,
                            warning.directive,
                            warning.message
                        );
                    }
                    eprintln!("\nTo fix: Review ~/.ssh/config and ensure all host-specific");
                    eprintln!("directives are under a 'Host' block.");
                }

                output = output.with_detail("ssh_warnings", &ssh_config.get_warnings().len().to_string());
            }
        }
    }

    // Scan for identities from existing configuration
    if opts.ssh_only || !opts.git_only {
        ctx.debug("Scanning SSH config...");

        let identities = detector::detect_identities()?;

        let ssh_identities: Vec<_> = identities
            .iter()
            .filter(|i| i.strategy == crate::strategy::StrategyType::SshAlias)
            .collect();

        output = output.with_detail("ssh_entries", &ssh_identities.len().to_string());

        if !ctx.quiet && !ssh_identities.is_empty() {
            eprintln!("\nFound {} SSH-based identities:", ssh_identities.len());
            for identity in &ssh_identities {
                if let crate::scan::detector::DetectionSource::SshConfig { host } = &identity.source
                {
                    let legacy_note = if identity.is_legacy {
                        " [legacy gitid-*]"
                    } else {
                        ""
                    };
                    eprintln!("  - {} ({}){}", identity.name, host, legacy_note);
                    if let Some(ref key_path) = identity.key_path {
                        eprintln!("    Key: {}", key_path);
                    }
                }
            }
        }
    }

    if opts.git_only || !opts.ssh_only {
        ctx.debug("Scanning Git config...");

        let identities = detector::detect_identities()?;

        let conditional_identities: Vec<_> = identities
            .iter()
            .filter(|i| i.strategy == crate::strategy::StrategyType::Conditional)
            .collect();

        let url_rewrite_identities: Vec<_> = identities
            .iter()
            .filter(|i| i.strategy == crate::strategy::StrategyType::UrlRewrite)
            .collect();

        output = output.with_detail("git_conditionals", &conditional_identities.len().to_string());
        output = output.with_detail("url_rewrites", &url_rewrite_identities.len().to_string());

        if !ctx.quiet {
            if !conditional_identities.is_empty() {
                eprintln!(
                    "\nFound {} conditional include identities:",
                    conditional_identities.len()
                );
                for identity in &conditional_identities {
                    if let crate::scan::detector::DetectionSource::GitConditional {
                        condition,
                        path,
                    } = &identity.source
                    {
                        eprintln!("  - {} ({})", identity.name, condition);
                        eprintln!("    Include: {}", path);
                    }
                }
            }

            if !url_rewrite_identities.is_empty() {
                eprintln!(
                    "\nFound {} URL rewrite identities:",
                    url_rewrite_identities.len()
                );
                for identity in &url_rewrite_identities {
                    if let crate::scan::detector::DetectionSource::GitUrlRewrite {
                        original,
                        replacement,
                    } = &identity.source
                    {
                        eprintln!("  - {}", identity.name);
                        eprintln!("    {} -> {}", original, replacement);
                    }
                }
            }
        }
    }

    Ok(output)
}
