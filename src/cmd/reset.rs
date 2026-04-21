//! Implementation of `gt reset` command
//!
//! This command supports:
//! - `gt reset commits` - Reset to initial commit, keep changes, clear schedule & history
//! - `gt reset staged` - Unstage all files
//! - `gt reset <args>` - Passthrough to git reset

use crate::cli::args::ResetOpts;
use crate::cli::output::Output;
use crate::cmd::Context;
use crate::error::{Error, Result};
use crate::io::schedule_config::ScheduleConfig;
use crate::util::execute_git_command;
use std::process::Command;

/// Execute the reset command
pub fn execute(opts: &ResetOpts, ctx: &Context) -> Result<Output> {
    ctx.debug("Executing reset command");

    // Check if first arg is "commits" or "staged"
    let first_arg = opts.args.first().map(|s| s.as_str());

    match first_arg {
        Some("commits") => execute_reset_commits(opts, ctx),
        Some("staged") => execute_reset_staged(ctx),
        _ => execute_reset_passthrough(opts, ctx),
    }
}

/// Reset to initial commit, keep changes, clear schedule & history
fn execute_reset_commits(opts: &ResetOpts, ctx: &Context) -> Result<Output> {
    ctx.debug("Resetting commits to initial commit");

    // Check if --keep-history is in args (due to trailing_var_arg)
    let keep_history = opts.keep_history || opts.args.iter().any(|a| a == "--keep-history");

    // Get current repository path
    let repo_path = std::env::current_dir()?;

    // Get current branch
    let branch_output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .map_err(|e| Error::GitCommand {
            message: format!("Failed to get current branch: {}", e),
        })?;

    if !branch_output.status.success() {
        return Err(Error::GitCommand {
            message: "Not in a git repository".to_string(),
        });
    }

    let current_branch = String::from_utf8_lossy(&branch_output.stdout)
        .trim()
        .to_string();

    // Get initial commit SHA
    let initial_commit_output = Command::new("git")
        .args(["rev-list", "--max-parents=0", "HEAD"])
        .output()
        .map_err(|e| Error::GitCommand {
            message: format!("Failed to get initial commit: {}", e),
        })?;

    if !initial_commit_output.status.success() {
        return Err(Error::GitCommand {
            message: "Failed to find initial commit".to_string(),
        });
    }

    let initial_commit = String::from_utf8_lossy(&initial_commit_output.stdout)
        .trim()
        .to_string();

    if ctx.dry_run {
        let mut actions = vec![
            format!("git reset {}", initial_commit),
            "Clear scheduled pushes".to_string(),
        ];
        if !keep_history {
            actions.push("git reflog expire --expire=now --all".to_string());
            actions.push("git gc --prune=now".to_string());
        }
        return Ok(Output::dry_run(format!(
            "Would execute:\n  {}",
            actions.join("\n  ")
        )));
    }

    // Reset to initial commit (soft reset - keeps changes)
    ctx.info(&format!("Resetting to initial commit: {}", initial_commit));
    execute_git_command("reset", std::slice::from_ref(&initial_commit))?;

    // Clear scheduled pushes for this repo/branch
    let mut schedule_config = ScheduleConfig::load()?;
    let removed = schedule_config.remove_schedule(&repo_path, &current_branch);
    if removed {
        schedule_config.save()?;
        ctx.info("Cleared scheduled push");
    }

    // Clear reflog and gc unless --keep-history
    if !keep_history {
        ctx.info("Clearing reflog and pruning git history...");

        execute_git_command(
            "reflog",
            &[
                "expire".to_string(),
                "--expire=now".to_string(),
                "--all".to_string(),
            ],
        )?;
        execute_git_command("gc", &["--prune=now".to_string()])?;

        ctx.info("History cleared");
    }

    Ok(Output::success("Reset to initial commit"))
}

/// Unstage all files (git reset HEAD)
fn execute_reset_staged(ctx: &Context) -> Result<Output> {
    ctx.debug("Unstaging all files");

    if ctx.dry_run {
        return Ok(Output::dry_run("Would execute: git reset HEAD"));
    }

    execute_git_command("reset", &["HEAD".to_string()])?;
    Ok(Output::success("Unstaged all files"))
}

/// Passthrough to git reset with all arguments
fn execute_reset_passthrough(opts: &ResetOpts, ctx: &Context) -> Result<Output> {
    ctx.debug(&format!("Passthrough to git reset: {:?}", opts.args));

    if ctx.dry_run {
        return Ok(Output::dry_run(format!(
            "Would execute: git reset {}",
            opts.args.join(" ")
        )));
    }

    execute_git_command("reset", &opts.args)?;
    Ok(Output::success(""))
}
