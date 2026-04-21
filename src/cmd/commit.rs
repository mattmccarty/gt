//! Implementation of `gt commit` command
//!
//! This is a passthrough command that forwards all arguments to `git commit`,
//! but with support for shorthand date syntax in the --date flag.

use crate::cli::args::{CommitListOpts, CommitOpts};
use crate::cli::output::Output;
use crate::cmd::Context;
use crate::error::{Error, Result};
use crate::io::schedule_config::ScheduleConfig;
use crate::util::{
    execute_git_command, execute_git_command_paginated, get_head_commit_date, paginate_output,
    parse_shorthand_date, random_date_after,
};
use chrono::{DateTime, Local, Utc};
use std::process::Command;

/// Execute the commit command
///
/// This command passes through all arguments to `git commit`, but transforms
/// the --date flag to support shorthand syntax like "-1h", "2d", "-1w", etc.
pub fn execute(opts: &CommitOpts, ctx: &Context) -> Result<Output> {
    ctx.debug("Executing git commit passthrough");

    // Handle help flag
    if opts.help {
        // Build header lines for our custom help
        let header = vec![
            "gt commit - Git commit with enhanced date support".to_string(),
            String::new(),
            "This command passes through all arguments to 'git commit', but adds support"
                .to_string(),
            "for shorthand date syntax in the --date flag.".to_string(),
            String::new(),
            "Shorthand date formats:".to_string(),
            "  now       - current time".to_string(),
            "  -Ns, Ns   - N seconds ago or from now (e.g., -30s, 45s)".to_string(),
            "  -Nm, Nm   - N minutes ago or from now (e.g., -15m, 30m)".to_string(),
            "  -Nh, Nh   - N hours ago or from now (e.g., -1h, 2h)".to_string(),
            "  -Nd, Nd   - N days ago or from now (e.g., -1d, 1d)".to_string(),
            "  -Nw, Nw   - N weeks ago or from now (e.g., -1w, 1w)".to_string(),
            String::new(),
            "Examples:".to_string(),
            "  gt commit -m \"message\" --date=-1h    # Commit with timestamp 1 hour ago"
                .to_string(),
            "  gt commit -m \"message\" --date=30m    # Commit with timestamp 30 minutes from now"
                .to_string(),
            "  gt commit -m \"message\" --date=2d     # Commit with timestamp 2 days from now"
                .to_string(),
            "  gt commit -m \"message\" --date=now    # Commit with current timestamp".to_string(),
            String::new(),
            "=".repeat(80),
            String::new(),
        ];

        // Pass through to git commit --help with pagination
        execute_git_command_paginated("commit", &["--help".to_string()], 20, Some(header))?;
        return Ok(Output::success(""));
    }

    // Build the arguments to pass to git commit
    let mut git_args: Vec<String> = Vec::new();
    let mut skip_next = false;
    let mut date_value: Option<String> = None;
    let mut force_from_args = false;
    let mut auto_from_args = false;

    // First, extract the --date, --force, and --auto values from git_args if present
    for (i, arg) in opts.git_args.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }

        if arg == "--date" {
            // --date value is in next argument
            if i + 1 < opts.git_args.len() {
                date_value = Some(opts.git_args[i + 1].clone());
                skip_next = true;
            }
        } else if arg.starts_with("--date=") {
            // --date=value format
            date_value = Some(arg.strip_prefix("--date=").unwrap().to_string());
        } else if arg == "--force" {
            // --force flag found in git_args
            force_from_args = true;
        } else if arg == "--auto" {
            // --auto flag found in git_args
            auto_from_args = true;
        } else {
            git_args.push(arg.clone());
        }
    }

    // Override with explicit --date from CommitOpts if present
    if opts.date.is_some() {
        date_value = opts.date.clone();
    }

    // Check both global and local flags (including those found in git_args)
    let force = ctx.force || opts.force || force_from_args;
    let auto = ctx.auto || opts.auto || auto_from_args;

    // Chronological order handling: check if HEAD has future date
    if let Some(head_date) = get_head_commit_date()? {
        let now = Utc::now();

        if head_date > now {
            // HEAD has future date
            ctx.debug(&format!(
                "HEAD commit has future date: {} (current time: {})",
                head_date, now
            ));

            if let Some(user_date_str) = date_value.clone() {
                if !force {
                    // User specified --date, validate it's not before HEAD.
                    let user_date = if let Ok(parsed) = parse_shorthand_date(&user_date_str) {
                        DateTime::parse_from_rfc3339(&parsed)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    } else {
                        DateTime::parse_from_rfc3339(&user_date_str)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    };

                    if let Some(user_dt) = user_date {
                        if user_dt < head_date {
                            let local_head = head_date.with_timezone(&chrono::Local);
                            let local_user = user_dt.with_timezone(&chrono::Local);
                            return Err(Error::ConfigInvalid {
                                message: format!(
                                    "Specified date ({}) is before HEAD commit date ({})\n\n\
                                    This would create commits out of chronological order.\n\n\
                                    To commit with earlier date anyway, use --force:\n  \
                                    gt --force commit -m \"message\" --date={}",
                                    local_user.format("%Y-%m-%d %I:%M:%S%P %z"),
                                    local_head.format("%Y-%m-%d %I:%M:%S%P %z"),
                                    user_date_str
                                ),
                            });
                        }
                    }
                }
                // else: --force set, skip the chronology check.
            } else {
                // No --date specified
                if force {
                    // --force: use current time (may create out-of-order commits)
                    ctx.info("Using current time (--force specified)");
                    date_value = Some(now.to_rfc3339());
                } else if auto {
                    // --auto: pick random time within 2 hours after HEAD
                    let auto_date = random_date_after(&head_date);
                    let local_auto = auto_date.with_timezone(&chrono::Local);
                    ctx.info(&format!(
                        "Auto-picked date: {} (randomly chosen within 2 hours after HEAD)",
                        local_auto.format("%Y-%m-%d %I:%M:%S%P %z")
                    ));
                    date_value = Some(auto_date.to_rfc3339());
                } else {
                    // Default: warn and exit
                    let local_head = head_date.with_timezone(&chrono::Local);
                    return Err(Error::ConfigInvalid {
                        message: format!(
                            "HEAD commit has a future date: {}\n\n\
                            To commit with chronological ordering, use one of:\n  \
                            --date=<date>  Specify a date after {}\n  \
                            --force        Use current time (may create out-of-order commits)\n  \
                            --auto         Randomly pick a date within 2 hours after HEAD\n\n\
                            Example: gt commit -m \"message\" --auto",
                            local_head.format("%Y-%m-%d %I:%M:%S%P %z"),
                            local_head.format("%Y-%m-%d %I:%M:%S%P %z")
                        ),
                    });
                }
            }
        }
    }

    // Handle the --date flag specially if present
    if let Some(date_val) = date_value {
        ctx.debug(&format!("Processing --date flag with value: {}", date_val));

        // Try to parse as shorthand date
        match parse_shorthand_date(&date_val) {
            Ok(parsed_date) => {
                ctx.debug(&format!(
                    "Parsed shorthand date '{}' to '{}'",
                    date_val, parsed_date
                ));
                // Add the transformed date
                git_args.push("--date".to_string());
                git_args.push(parsed_date);
            }
            Err(_) => {
                // Not a shorthand date, pass it through as-is
                ctx.debug(&format!("Using date as-is (not shorthand): {}", date_val));
                // Add the date value
                git_args.push("--date".to_string());
                git_args.push(date_val);
            }
        }
    }

    if ctx.dry_run {
        let args_display = git_args.join(" ");
        return Ok(Output::dry_run(format!(
            "Would execute: git commit {}",
            args_display
        )));
    }

    // Execute git commit with all the arguments
    execute_git_command("commit", &git_args)?;

    Ok(Output::success("Commit completed"))
}

/// Execute the commit list command
///
/// Shows commits sorted from earliest to latest with schedule information
pub fn execute_list(
    opts: &CommitListOpts,
    parent_opts: &CommitOpts,
    ctx: &Context,
) -> Result<Output> {
    ctx.debug("Executing commit list");

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

    // Check for --all flag (from ctx, parent opts, or git_args)
    let mut show_all = ctx.all || parent_opts.all;
    if !show_all {
        for arg in &parent_opts.git_args {
            if arg == "--all" || arg == "-a" {
                show_all = true;
                break;
            }
        }
    }

    // Build git log command
    let mut args = vec!["log".to_string(), "--format=%H|%aI|%s".to_string()];

    // If not showing all, only show unpushed commits
    if !show_all {
        // Get upstream branch
        let upstream_output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
            .output();

        if let Ok(output) = upstream_output {
            if output.status.success() {
                let upstream = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !upstream.is_empty() {
                    // Show commits ahead of upstream
                    args.push(format!("{}..HEAD", upstream));
                }
            }
        }
        // If no upstream or error, show all commits (nothing has been pushed)
    }

    // Add limit if specified and not 0
    if opts.limit > 0 {
        args.push(format!("-{}", opts.limit));
    }

    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    let log_output =
        Command::new("git")
            .args(&args_refs)
            .output()
            .map_err(|e| Error::GitCommand {
                message: format!("Failed to get commit log: {}", e),
            })?;

    if !log_output.status.success() {
        return Err(Error::GitCommand {
            message: format!(
                "git log failed: {}",
                String::from_utf8_lossy(&log_output.stderr)
            ),
        });
    }

    // Parse commits
    let log_str = String::from_utf8_lossy(&log_output.stdout);
    let mut commits: Vec<(String, DateTime<Utc>, String)> = Vec::new();

    for line in log_str.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 3 {
            let sha = parts[0].to_string();
            let date_str = parts[1];
            let subject = parts[2].to_string();

            if let Ok(date) = DateTime::parse_from_rfc3339(date_str) {
                commits.push((sha, date.with_timezone(&Utc), subject));
            }
        }
    }

    // Sort by date (earliest first)
    commits.sort_by_key(|(_, date, _)| *date);

    // Load scheduled pushes
    let schedule_config = ScheduleConfig::load()?;
    let schedule = schedule_config.get_schedule(&repo_path, &current_branch);

    // Build output lines
    let mut lines = Vec::new();

    for (sha, date, subject) in commits {
        let short_sha = &sha[..7];
        let local_date = date.with_timezone(&Local);
        let formatted_date = local_date.format("%Y-%m-%d %I:%M:%S%P %z");

        // Check if this commit is scheduled
        let schedule_info = if let Some(sched) = &schedule {
            if sched.commit_sha.starts_with(short_sha) || sha.starts_with(&sched.commit_sha) {
                let sched_local = sched.scheduled_time.with_timezone(&Local);
                format!(
                    "  SCHEDULED: {}",
                    sched_local.format("%Y-%m-%d %I:%M:%S%P %z")
                )
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        lines.push(format!("{} {} {}", short_sha, formatted_date, subject));

        if !schedule_info.is_empty() {
            lines.push(schedule_info);
        }
    }

    if lines.is_empty() {
        lines.push("No commits found".to_string());
    }

    // Use pagination for long output (more than 20 lines)
    if lines.len() > 20 {
        paginate_output(lines.into_iter(), 20, None)?;
        Ok(Output::success(""))
    } else {
        Ok(Output::success(lines.join("\n")))
    }
}
