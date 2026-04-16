//! Implementation of `gt status` command
//!
//! This is a passthrough command that forwards all arguments to `git status`.

use crate::cli::args::GitStatusOpts;
use crate::cli::output::Output;
use crate::cmd::Context;
use crate::error::Result;
use crate::util::execute_git_command;

/// Execute the status command
///
/// This command passes through all arguments to `git status`.
pub fn execute(opts: &GitStatusOpts, ctx: &Context) -> Result<Output> {
    ctx.debug("Executing git status passthrough");

    // Handle help flag - pass through to git
    if opts.help {
        execute_git_command("status", &["--help".to_string()])?;
        return Ok(Output::success(""));
    }

    if ctx.dry_run {
        let args_display = opts.git_args.join(" ");
        return Ok(Output::dry_run(format!(
            "Would execute: git status {}",
            args_display
        )));
    }

    // Execute git status with all the arguments
    execute_git_command("status", &opts.git_args)?;

    Ok(Output::success(""))
}
