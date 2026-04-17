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
