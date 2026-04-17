//! Implementation of `gt push` command
//!
//! This command extends git push with support for scheduled pushes when
//! commits have future dates. It automatically detects future-dated commits
//! and schedules the push for the latest commit date.

use std::env;
use std::path::PathBuf;
use std::process::Command;

use chrono::{DateTime, Utc};

use crate::cli::args::PushOpts;
use crate::cli::output::Output;
use crate::cmd::Context;
use crate::error::{Error, Result};
use crate::io::git_hooks::{install_pre_push_hook, is_git_repo, remove_pre_push_hook};
use crate::io::schedule_config::{
    LocalScheduleCache, Schedule, ScheduleConfig, ScheduleStatus,
};

/// Execute the push command
pub fn execute(opts: &PushOpts, ctx: &Context) -> Result<Output> {
    ctx.debug("Executing git push with schedule support");

    // Handle different modes
    if opts.list {
        return list_schedules(ctx);
    }

    if opts.cancel {
        return cancel_schedule(opts, ctx);
    }

    if opts.hook_check {
        return hook_check(opts, ctx);
    }

    // Normal push flow
    let repo_path = get_repo_path()?;
    let current_branch = get_current_branch(&repo_path)?;
    let branch = opts.branch.as_deref().unwrap_or(&current_branch);
    let remote = opts.remote.as_deref().unwrap_or("origin");

    // Check for future-dated commits
    if !opts.force && !ctx.force {
        if let Some(future_commit) = find_latest_future_commit(&repo_path, remote, branch)? {
            return create_schedule(
                &repo_path,
                remote,
                branch,
                &future_commit.sha,
                future_commit.date,
                ctx,
            );
        }
    }

    // No future commits or force flag used - push normally
    push_now(&repo_path, remote, branch, &opts.git_args, ctx)
}

/// List all scheduled pushes
fn list_schedules(_ctx: &Context) -> Result<Output> {
    let config = ScheduleConfig::load()?;

    if config.schedules.is_empty() {
        return Ok(Output::success("No scheduled pushes"));
    }

    let mut output = Output::success(format!("Found {} scheduled push(es)", config.schedules.len()));

    for schedule in config.list_schedules() {
        let status_str = match schedule.status {
            ScheduleStatus::Pending => "pending",
            ScheduleStatus::Failed => "failed",
        };

        let time_str = format_schedule_time(schedule.scheduled_time);
        let repo_name = schedule
            .repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let detail_key = format!(
            "{}/{} ({})",
            repo_name, schedule.branch, status_str
        );
        let detail_value = format!(
            "{} - commit {}",
            time_str,
            &schedule.commit_sha[..7]
        );

        output = output.with_detail(detail_key, detail_value);
    }

    Ok(output)
}

/// Cancel a scheduled push
fn cancel_schedule(opts: &PushOpts, ctx: &Context) -> Result<Output> {
    let repo_path = get_repo_path()?;
    let current_branch = get_current_branch(&repo_path)?;
    let branch = opts.branch.as_deref().unwrap_or(&current_branch);

    let mut config = ScheduleConfig::load()?;

    if !config.remove_schedule(&repo_path, branch) {
        return Err(Error::ScheduleNotFound);
    }

    config.save()?;

    // Remove local cache
    LocalScheduleCache::remove(&repo_path)?;

    // Remove hook if no more schedules for this repo
    let has_other_schedules = config
        .list_schedules()
        .iter()
        .any(|s| s.repo_path == repo_path);

    if !has_other_schedules {
        remove_pre_push_hook(&repo_path)?;
        ctx.info("Removed pre-push hook (no more schedules for this repository)");
    }

    Ok(Output::success(format!(
        "Cancelled scheduled push for branch '{}'",
        branch
    )))
}

/// Hook check - called by pre-push hook
fn hook_check(opts: &PushOpts, _ctx: &Context) -> Result<Output> {
    // Get remote from positional argument (git passes remote name as $1)
    let remote = match &opts.remote {
        Some(r) => r.as_str(),
        None => return Ok(Output::success("")),
    };

    let repo_path = get_repo_path()?;
    let current_branch = get_current_branch(&repo_path)?;

    // Load schedule
    let config = ScheduleConfig::load()?;
    let schedule = match config.get_schedule(&repo_path, &current_branch) {
        Some(s) => s,
        None => return Ok(Output::success("")),
    };

    // Check if schedule time has passed
    let now = Utc::now();
    if now >= schedule.scheduled_time {
        return Ok(Output::success(""));
    }

    // Check if schedule is for this remote
    if schedule.remote != *remote {
        return Ok(Output::success(""));
    }

    // Block the push
    let time_str = format_schedule_time(schedule.scheduled_time);

    Err(Error::PushScheduled {
        scheduled_time: time_str,
    })
}

/// Create a schedule for a future push
fn create_schedule(
    repo_path: &PathBuf,
    remote: &str,
    branch: &str,
    commit_sha: &str,
    scheduled_time: DateTime<Utc>,
    ctx: &Context,
) -> Result<Output> {
    let repo_id = get_repo_id(repo_path, remote)?;

    // Capture SSH_AUTH_SOCK for later use
    let ssh_auth_sock = env::var("SSH_AUTH_SOCK").ok();

    let schedule = Schedule {
        repo_path: repo_path.clone(),
        repo_id,
        branch: branch.to_string(),
        remote: remote.to_string(),
        scheduled_time,
        commit_sha: commit_sha.to_string(),
        created_at: Utc::now(),
        status: ScheduleStatus::Pending,
        last_attempt: None,
        failure_reason: None,
        attempt_count: 0,
        ssh_auth_sock,
    };

    // Save to global config
    let mut config = ScheduleConfig::load()?;
    config.add_schedule(schedule);
    config.save()?;

    // Save local cache
    let local_cache = LocalScheduleCache {
        scheduled_time,
        branch: branch.to_string(),
        commit_sha: commit_sha.to_string(),
        remote: remote.to_string(),
    };
    local_cache.save(repo_path)?;

    // Install pre-push hook
    install_pre_push_hook(repo_path)?;
    ctx.info("Installed pre-push hook to prevent early pushes");

    let time_str = format_schedule_time(scheduled_time);
    let duration_str = format_duration_until(scheduled_time);

    Ok(Output::success(format!(
        "Push scheduled for {} ({})",
        time_str, duration_str
    ))
    .with_detail("branch", branch)
    .with_detail("remote", remote)
    .with_detail("commit", &commit_sha[..7])
    .with_warning("Use 'gt push --force' to push immediately")
    .with_warning("Cancel schedule: gt push --cancel"))
}

/// Push immediately
fn push_now(
    repo_path: &PathBuf,
    remote: &str,
    branch: &str,
    git_args: &[String],
    ctx: &Context,
) -> Result<Output> {
    ctx.debug(&format!("Pushing {} to {}", branch, remote));

    if ctx.dry_run {
        return Ok(Output::dry_run(format!(
            "Would push {} to {}",
            branch, remote
        )));
    }

    // Inherit stdio so git push renders colors and progress output directly.
    // Schedule cleanup only runs on success; failure short-circuits with git's
    // own exit code so shell scripts see the real push failure.
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_path);
    cmd.arg("push");
    cmd.arg(remote);
    cmd.arg(branch);

    for arg in git_args {
        cmd.arg(arg);
    }

    let status = cmd.status().map_err(|e| Error::GitCommand {
        message: format!("Failed to execute git push: {e}"),
    })?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    // Clean up schedule if it exists (only on successful push)
    let mut config = ScheduleConfig::load()?;
    if config.remove_schedule(repo_path, branch) {
        config.save()?;
        LocalScheduleCache::remove(repo_path)?;
        ctx.info("Removed push schedule");
    }

    Ok(Output::success("Push completed"))
}

/// Find the latest future-dated commit
fn find_latest_future_commit(
    repo_path: &PathBuf,
    remote: &str,
    branch: &str,
) -> Result<Option<FutureCommit>> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_path);
    cmd.arg("log");
    cmd.arg(format!("{}/{}..HEAD", remote, branch));
    cmd.arg("--format=%H %aI");

    let output = cmd.output().map_err(|e| Error::GitCommand {
        message: format!("Failed to get commit log: {}", e),
    })?;

    if !output.status.success() {
        // If remote branch doesn't exist yet, check all commits
        let mut cmd = Command::new("git");
        cmd.current_dir(repo_path);
        cmd.arg("log");
        cmd.arg("HEAD");
        cmd.arg("--format=%H %aI");

        let output = cmd.output().map_err(|e| Error::GitCommand {
            message: format!("Failed to get commit log: {}", e),
        })?;

        if !output.status.success() {
            return Ok(None);
        }

        return parse_commit_log(&output.stdout);
    }

    parse_commit_log(&output.stdout)
}

/// Parse commit log output and find the latest future commit
fn parse_commit_log(output: &[u8]) -> Result<Option<FutureCommit>> {
    let text = String::from_utf8_lossy(output);
    let now = Utc::now();
    let mut latest_future: Option<FutureCommit> = None;

    for line in text.lines() {
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() != 2 {
            continue;
        }

        let sha = parts[0].to_string();
        let date_str = parts[1];

        // Parse ISO 8601 date
        let date = DateTime::parse_from_rfc3339(date_str)
            .map_err(|e| Error::GitCommand {
                message: format!("Failed to parse commit date: {}", e),
            })?
            .with_timezone(&Utc);

        // Check if it's in the future
        if date > now {
            if let Some(ref current) = latest_future {
                if date > current.date {
                    latest_future = Some(FutureCommit { sha, date });
                }
            } else {
                latest_future = Some(FutureCommit { sha, date });
            }
        }
    }

    Ok(latest_future)
}

/// Get the current repository path
fn get_repo_path() -> Result<PathBuf> {
    let mut cmd = Command::new("git");
    cmd.arg("rev-parse");
    cmd.arg("--show-toplevel");

    let output = cmd.output().map_err(|e| Error::GitCommand {
        message: format!("Failed to get repository path: {}", e),
    })?;

    if !output.status.success() {
        return Err(Error::NotARepository);
    }

    let path_str = String::from_utf8_lossy(&output.stdout);
    let path = PathBuf::from(path_str.trim());

    if !is_git_repo(&path) {
        return Err(Error::NotARepository);
    }

    Ok(path)
}

/// Get the current branch name
fn get_current_branch(repo_path: &PathBuf) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_path);
    cmd.arg("rev-parse");
    cmd.arg("--abbrev-ref");
    cmd.arg("HEAD");

    let output = cmd.output().map_err(|e| Error::GitCommand {
        message: format!("Failed to get current branch: {}", e),
    })?;

    if !output.status.success() {
        return Err(Error::GitCommand {
            message: "Failed to get current branch".to_string(),
        });
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(branch)
}

/// Get repository ID from remote URL
fn get_repo_id(repo_path: &PathBuf, remote: &str) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_path);
    cmd.arg("remote");
    cmd.arg("get-url");
    cmd.arg(remote);

    let output = cmd.output().map_err(|e| Error::GitCommand {
        message: format!("Failed to get remote URL: {}", e),
    })?;

    if !output.status.success() {
        return Err(Error::NoRemote {
            remote: remote.to_string(),
        });
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(parse_repo_id_from_url(&url))
}

/// Parse repository ID from URL
fn parse_repo_id_from_url(url: &str) -> String {
    // Handle SSH URLs: git@github.com:user/repo.git
    if let Some(ssh_part) = url.strip_prefix("git@") {
        let parts: Vec<&str> = ssh_part.split(':').collect();
        if parts.len() == 2 {
            let host = parts[0];
            let path = parts[1].trim_end_matches(".git");
            return format!("{}/{}", host, path);
        }
    }

    // Handle HTTPS URLs: https://github.com/user/repo.git
    if url.starts_with("http://") || url.starts_with("https://") {
        let without_protocol = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))
            .unwrap_or(url);
        let path = without_protocol.trim_end_matches(".git");
        return path.to_string();
    }

    // Fallback: use as-is
    url.trim_end_matches(".git").to_string()
}

/// Format a scheduled time for display
fn format_schedule_time(time: DateTime<Utc>) -> String {
    // Convert to local time for display
    let local_time = time.with_timezone(&chrono::Local);
    local_time.format("%Y-%m-%d %H:%M:%S %Z").to_string()
}

/// Format duration until scheduled time
fn format_duration_until(time: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = time.signed_duration_since(now);

    if duration.num_days() > 0 {
        let days = duration.num_days();
        let hours = duration.num_hours() % 24;
        if hours > 0 {
            format!("in {} day(s), {} hour(s)", days, hours)
        } else {
            format!("in {} day(s)", days)
        }
    } else if duration.num_hours() > 0 {
        let hours = duration.num_hours();
        let minutes = duration.num_minutes() % 60;
        if minutes > 0 {
            format!("in {} hour(s), {} minute(s)", hours, minutes)
        } else {
            format!("in {} hour(s)", hours)
        }
    } else if duration.num_minutes() > 0 {
        format!("in {} minute(s)", duration.num_minutes())
    } else {
        "very soon".to_string()
    }
}

/// A commit with a future date
struct FutureCommit {
    sha: String,
    date: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_repo_id_ssh() {
        let url = "git@github.com:user/repo.git";
        let id = parse_repo_id_from_url(url);
        assert_eq!(id, "github.com/user/repo");
    }

    #[test]
    fn test_parse_repo_id_https() {
        let url = "https://github.com/user/repo.git";
        let id = parse_repo_id_from_url(url);
        assert_eq!(id, "github.com/user/repo");
    }

    #[test]
    fn test_parse_repo_id_no_git_suffix() {
        let url = "git@github.com:user/repo";
        let id = parse_repo_id_from_url(url);
        assert_eq!(id, "github.com/user/repo");
    }

    #[test]
    fn test_format_duration() {
        let now = Utc::now();
        let future = now + chrono::Duration::days(2) + chrono::Duration::hours(4);
        let duration_str = format_duration_until(future);
        assert!(duration_str.contains("day"));
    }
}
