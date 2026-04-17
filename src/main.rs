//! gt - Cross-platform Git tool
//!
//! A CLI tool for managing multiple Git identities with support for:
//! - SSH hostname aliases
//! - Git conditional includes
//! - URL rewriting with insteadOf

use anyhow::Result;
use clap::Parser;

use gt::cli::args::{Cli, Commands};
use gt::cli::output::Output;
use gt::cmd;

fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn"),
    )
    .init();

    // Forward `gt config <non-native>` (and flag-style invocations) to `git config`
    // before clap sees them, so gt behaves as a superset of git config. clap still
    // handles the gt-native subcommands: list, edit, validate, id, help.
    let raw_args: Vec<String> = std::env::args().collect();
    if let Some(git_args) = detect_git_config_passthrough(&raw_args) {
        let status = std::process::Command::new("git")
            .arg("config")
            .args(&git_args)
            .status()
            .map_err(|e| anyhow::anyhow!("Failed to execute git config: {e}"))?;
        std::process::exit(status.code().unwrap_or(1));
    }

    // Parse CLI arguments
    let cli = Cli::parse();

    // Create execution context
    let ctx = cmd::Context::new(&cli)?;

    // Execute command
    let result = match &cli.command {
        Commands::Config(opts) => cmd::config::execute(opts, &ctx),
        Commands::Clone(opts) => cmd::clone::execute(opts, &ctx),
        Commands::Commit(opts) => match &opts.command {
            Some(gt::cli::args::CommitCommands::List(list_opts)) => {
                cmd::commit::execute_list(list_opts, opts, &ctx)
            }
            None => cmd::commit::execute(opts, &ctx),
        },
        Commands::Status(opts) => cmd::git_status::execute(opts, &ctx),
        Commands::Push(opts) => cmd::push::execute(opts, &ctx),
        Commands::Reset(opts) => cmd::reset::execute(opts, &ctx),
        Commands::Add(opts) => gt::util::execute_git_passthrough("add", opts),
        Commands::Pull(opts) => gt::util::execute_git_passthrough("pull", opts),
        Commands::Fetch(opts) => gt::util::execute_git_passthrough("fetch", opts),
        Commands::Checkout(opts) => gt::util::execute_git_passthrough("checkout", opts),
        Commands::Branch(opts) => gt::util::execute_git_passthrough("branch", opts),
        Commands::Merge(opts) => gt::util::execute_git_passthrough("merge", opts),
        Commands::Rebase(opts) => gt::util::execute_git_passthrough("rebase", opts),
        Commands::Diff(opts) => gt::util::execute_git_passthrough("diff", opts),
        Commands::Log(opts) => gt::util::execute_git_passthrough("log", opts),
        Commands::Stash(opts) => gt::util::execute_git_passthrough("stash", opts),
        Commands::Tag(opts) => gt::util::execute_git_passthrough("tag", opts),
        Commands::Remote(opts) => gt::util::execute_git_passthrough("remote", opts),
    };

    // Handle result and output
    match result {
        Ok(output) => {
            output.print(&ctx)?;
            std::process::exit(0);
        }
        Err(e) => {
            Output::error(&e).print(&ctx)?;
            std::process::exit(e.exit_code());
        }
    }
}

/// Returns the trailing args to forward to `git config`, or `None` if clap
/// should handle the invocation.
///
/// gt-native under `config`: `validate`, `id`, `help`, plus bare `gt config`
/// (shows the gt config summary) and `gt config --help`. Anything else after
/// `config` — including `list`, `edit`, every other `git config` subcommand,
/// and flag-style invocations like `gt config --global user.email x` — is
/// forwarded verbatim.
fn detect_git_config_passthrough(args: &[String]) -> Option<Vec<String>> {
    const NATIVE: &[&str] = &["validate", "id", "help"];

    let config_pos = args
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(i, a)| (a == "config").then_some(i))?;

    let after = &args[config_pos + 1..];

    if after.is_empty() {
        return None;
    }

    let first = after[0].as_str();
    if first == "--help" || first == "-h" || NATIVE.contains(&first) {
        return None;
    }

    Some(after.to_vec())
}

#[cfg(test)]
mod tests {
    use super::detect_git_config_passthrough;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn bare_config_is_not_passthrough() {
        assert!(detect_git_config_passthrough(&args(&["gt", "config"])).is_none());
    }

    #[test]
    fn config_help_is_not_passthrough() {
        assert!(detect_git_config_passthrough(&args(&["gt", "config", "--help"])).is_none());
        assert!(detect_git_config_passthrough(&args(&["gt", "config", "-h"])).is_none());
    }

    #[test]
    fn native_subcommands_are_not_passthrough() {
        for sub in &["validate", "id", "help"] {
            assert!(
                detect_git_config_passthrough(&args(&["gt", "config", sub])).is_none(),
                "native subcommand {sub} should not passthrough"
            );
        }
    }

    #[test]
    fn list_and_edit_are_now_passthrough() {
        assert_eq!(
            detect_git_config_passthrough(&args(&["gt", "config", "list"])),
            Some(vec!["list".into()]),
        );
        assert_eq!(
            detect_git_config_passthrough(&args(&["gt", "config", "edit"])),
            Some(vec!["edit".into()]),
        );
    }

    #[test]
    fn unknown_subcommand_passes_through() {
        assert_eq!(
            detect_git_config_passthrough(&args(&["gt", "config", "get", "user.email"])),
            Some(vec!["get".into(), "user.email".into()]),
        );
    }

    #[test]
    fn flag_style_passes_through() {
        assert_eq!(
            detect_git_config_passthrough(&args(&[
                "gt",
                "config",
                "--global",
                "user.email",
                "me@example.com",
            ])),
            Some(vec![
                "--global".into(),
                "user.email".into(),
                "me@example.com".into(),
            ]),
        );
    }

    #[test]
    fn gt_native_subcommand_with_trailing_help_is_native() {
        // `gt config id --help` should be handled by clap, not forwarded.
        assert!(
            detect_git_config_passthrough(&args(&["gt", "config", "id", "--help"])).is_none()
        );
    }

    #[test]
    fn no_config_token_returns_none() {
        assert!(detect_git_config_passthrough(&args(&["gt", "commit", "-m", "hi"])).is_none());
    }

    #[test]
    fn global_flags_before_config_are_skipped() {
        assert_eq!(
            detect_git_config_passthrough(&args(&["gt", "--verbose", "config", "get", "user.email"])),
            Some(vec!["get".into(), "user.email".into()]),
        );
    }
}
