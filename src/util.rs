//! Shared utilities for gitid
//!
//! This module contains utility functions used across the application.

use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{Error, Result};
use chrono::{DateTime, Duration, Utc};
use rand::Rng;

/// Expands a path, replacing `~` with the home directory
///
/// # Examples
///
/// ```
/// use gt::util::expand_path;
/// use std::path::Path;
///
/// let expanded = expand_path(Path::new("~/.ssh/config")).unwrap();
/// assert!(expanded.is_absolute());
/// ```
pub fn expand_path(path: &Path) -> Result<PathBuf> {
    let path_str = path.to_string_lossy();

    if path_str.starts_with('~') {
        let home = dirs::home_dir().ok_or(Error::HomeNotFound)?;
        let remainder = path_str.strip_prefix('~').unwrap_or("");
        let remainder = remainder.strip_prefix('/').unwrap_or(remainder);
        let remainder = remainder.strip_prefix('\\').unwrap_or(remainder);

        if remainder.is_empty() {
            Ok(home)
        } else {
            Ok(home.join(remainder))
        }
    } else {
        Ok(path.to_owned())
    }
}

/// Gets the home directory
pub fn home_dir() -> Result<PathBuf> {
    dirs::home_dir().ok_or(Error::HomeNotFound)
}

/// Gets the SSH directory (~/.ssh)
pub fn ssh_dir() -> Result<PathBuf> {
    Ok(home_dir()?.join(".ssh"))
}

/// Gets the gt config directory
pub fn config_dir() -> Result<PathBuf> {
    let config = dirs::config_dir().ok_or(Error::HomeNotFound)?;
    Ok(config.join("gt"))
}

/// Gets the gt config file path
pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

/// Normalizes a path for display, using platform-native separators
#[must_use]
pub fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// Normalizes a path for SSH config (always forward slashes)
#[must_use]
pub fn ssh_path(path: &Path) -> String {
    let s = path.to_string_lossy();
    s.replace('\\', "/")
}

/// Validates an identity name
///
/// Valid names must:
/// - Be 2-32 characters long
/// - Start with a letter
/// - Contain only alphanumeric characters and hyphens
/// - Not contain "gt-" prefix (reserved)
pub fn validate_identity_name(name: &str) -> Result<()> {
    if name.len() < 2 {
        return Err(Error::IdentityNameInvalid {
            name: name.to_string(),
            reason: "must be at least 2 characters".to_string(),
        });
    }

    if name.len() > 32 {
        return Err(Error::IdentityNameInvalid {
            name: name.to_string(),
            reason: "must be at most 32 characters".to_string(),
        });
    }

    let first_char = name.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() {
        return Err(Error::IdentityNameInvalid {
            name: name.to_string(),
            reason: "must start with a letter".to_string(),
        });
    }

    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(Error::IdentityNameInvalid {
            name: name.to_string(),
            reason: "must contain only letters, numbers, and hyphens".to_string(),
        });
    }

    if name.to_lowercase().contains("gt-") {
        return Err(Error::IdentityNameInvalid {
            name: name.to_string(),
            reason: "cannot contain 'gt-' (reserved prefix)".to_string(),
        });
    }

    Ok(())
}

/// Checks if a string looks like it might be a secret
#[must_use]
pub fn looks_like_secret(s: &str) -> bool {
    let lower = s.to_lowercase();

    // Check for common secret patterns
    lower.contains("password")
        || lower.contains("secret")
        || lower.contains("token")
        || lower.contains("api_key")
        || lower.contains("apikey")
        || s.len() > 40 && s.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Generates a timestamp string for backups
#[must_use]
pub fn backup_timestamp() -> String {
    chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string()
}

/// Parses shorthand date syntax and converts to ISO 8601 format for git
///
/// Supported formats:
/// - `now` - current time
/// - `-Ns` or `Ns` - N seconds ago or from now (e.g., `-30s`, `45s`)
/// - `-Nm` or `Nm` - N minutes ago or from now (e.g., `-15m`, `30m`)
/// - `-Nh` or `Nh` - N hours ago or from now (e.g., `-1h`, `2h`)
/// - `-Nd` or `Nd` - N days ago or from now (e.g., `-1d`, `1d`)
/// - `-Nw` or `Nw` - N weeks ago or from now (e.g., `-1w`, `1w`)
///
/// Returns the date in ISO 8601 format suitable for `git commit --date`
///
/// # Examples
///
/// ```
/// use gt::util::parse_shorthand_date;
///
/// let now = parse_shorthand_date("now").unwrap();
/// let one_hour_ago = parse_shorthand_date("-1h").unwrap();
/// let thirty_minutes = parse_shorthand_date("30m").unwrap();
/// let two_days_from_now = parse_shorthand_date("2d").unwrap();
/// ```
pub fn parse_shorthand_date(input: &str) -> Result<String> {
    let input = input.trim();

    // Handle "now" special case
    if input.eq_ignore_ascii_case("now") {
        return Ok(Utc::now().to_rfc3339());
    }

    // Parse the pattern: optional minus sign, number, unit
    let re = regex::Regex::new(r"^(-)?(\d+)([smhdw])$").map_err(|e| Error::ConfigInvalid {
        message: format!("Invalid regex: {}", e),
    })?;

    let caps = re.captures(input).ok_or_else(|| Error::ConfigInvalid {
        message: format!(
            "Invalid date format '{}'. Use: now, -Ns, Ns, -Nm, Nm, -Nh, Nh, -Nd, Nd, -Nw, Nw",
            input
        ),
    })?;

    let is_past = caps.get(1).is_some();
    let amount: i64 = caps[2].parse().map_err(|e| Error::ConfigInvalid {
        message: format!("Invalid number in date: {}", e),
    })?;
    let unit = &caps[3];

    // Calculate the duration
    let duration = match unit {
        "s" => Duration::seconds(amount),
        "m" => Duration::minutes(amount),
        "h" => Duration::hours(amount),
        "d" => Duration::days(amount),
        "w" => Duration::weeks(amount),
        _ => unreachable!("Regex should only match s, m, h, d, w"),
    };

    // Calculate the target datetime
    let target: DateTime<Utc> = if is_past {
        Utc::now() - duration
    } else {
        Utc::now() + duration
    };

    // Format as ISO 8601
    Ok(target.to_rfc3339())
}

/// Executes a git command with the given arguments
///
/// This is a passthrough utility that runs git commands and captures their output.
/// Both stdout and stderr are captured and displayed.
///
/// # Arguments
///
/// * `subcommand` - The git subcommand (e.g., "commit", "push")
/// * `args` - Additional arguments to pass to git
///
/// # Examples
///
/// ```no_run
/// use gt::util::execute_git_command;
///
/// // Execute: git commit -m "message"
/// execute_git_command("commit", &["-m", "message"]).unwrap();
/// ```
pub fn execute_git_command(subcommand: &str, args: &[String]) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.arg(subcommand);

    for arg in args {
        cmd.arg(arg);
    }

    // Execute the command
    let output = cmd.output().map_err(|e| Error::GitCommand {
        message: format!("Failed to execute git command: {}", e),
    })?;

    // Print stdout
    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }

    // Print stderr
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }

    // Check if command was successful
    if !output.status.success() {
        return Err(Error::GitCommand {
            message: format!(
                "git {} failed with exit code: {}",
                subcommand,
                output.status.code().unwrap_or(-1)
            ),
        });
    }

    Ok(())
}

/// Paginates output line by line with user control
///
/// Displays content in pages, prompting the user to press Enter to see more lines.
/// This is useful for long output that shouldn't scroll past the user's view.
///
/// # Arguments
///
/// * `lines` - Iterator of lines to display
/// * `lines_per_page` - Number of lines to show before prompting (default: 20)
/// * `initial_lines` - Lines to always show at the top without pagination
///
/// # Examples
///
/// ```no_run
/// use gt::util::paginate_output;
///
/// let header = vec!["Header line 1", "Header line 2"];
/// let content = vec!["Line 1", "Line 2", "Line 3"];
/// let all_lines: Vec<String> = content.iter().map(|s| s.to_string()).collect();
///
/// paginate_output(all_lines.into_iter(), 20, Some(header.iter().map(|s| s.to_string()).collect())).unwrap();
/// ```
pub fn paginate_output<I>(lines: I, lines_per_page: usize, initial_lines: Option<Vec<String>>) -> Result<()>
where
    I: Iterator<Item = String>,
{
    let mut stdout = io::stdout();
    let stdin = io::stdin();

    // Show initial lines without pagination
    if let Some(header) = initial_lines {
        for line in header {
            writeln!(stdout, "{}", line).map_err(|e| Error::GitCommand {
                message: format!("Failed to write to stdout: {}", e),
            })?;
        }
    }

    let mut line_count = 0;
    let mut lines_vec: Vec<String> = lines.collect();
    let total_lines = lines_vec.len();

    // If there are only a few lines, just print them all
    if total_lines <= lines_per_page {
        for line in lines_vec {
            writeln!(stdout, "{}", line).map_err(|e| Error::GitCommand {
                message: format!("Failed to write to stdout: {}", e),
            })?;
        }
        return Ok(());
    }

    let mut idx = 0;
    while idx < total_lines {
        // Display a page of lines
        let end_idx = std::cmp::min(idx + lines_per_page, total_lines);
        for i in idx..end_idx {
            writeln!(stdout, "{}", lines_vec[i]).map_err(|e| Error::GitCommand {
                message: format!("Failed to write to stdout: {}", e),
            })?;
        }
        idx = end_idx;
        line_count += end_idx - (end_idx - lines_per_page).max(0);

        // If we've shown all lines, break
        if idx >= total_lines {
            break;
        }

        // Prompt for more
        let remaining = total_lines - idx;
        eprint!("\n--- Press Enter for more ({} lines remaining), or Ctrl+C to quit --- ", remaining);
        stdout.flush().map_err(|e| Error::GitCommand {
            message: format!("Failed to flush stdout: {}", e),
        })?;
        io::stderr().flush().map_err(|e| Error::GitCommand {
            message: format!("Failed to flush stderr: {}", e),
        })?;

        // Wait for Enter
        let mut input = String::new();
        stdin.lock().read_line(&mut input).map_err(|e| Error::GitCommand {
            message: format!("Failed to read from stdin: {}", e),
        })?;
    }

    Ok(())
}

/// Executes a git command and paginates its output
///
/// Similar to `execute_git_command`, but paginates the output for better readability.
/// Useful for commands with long output like `git log` or `git help`.
///
/// # Arguments
///
/// * `subcommand` - The git subcommand (e.g., "commit", "log")
/// * `args` - Additional arguments to pass to git
/// * `lines_per_page` - Number of lines to show per page (default: 20)
/// * `initial_lines` - Optional header lines to show before pagination starts
///
/// # Examples
///
/// ```no_run
/// use gt::util::execute_git_command_paginated;
///
/// let header = vec!["Custom header".to_string()];
/// execute_git_command_paginated("help", &["commit".to_string()], 20, Some(header)).unwrap();
/// ```
pub fn execute_git_command_paginated(
    subcommand: &str,
    args: &[String],
    lines_per_page: usize,
    initial_lines: Option<Vec<String>>,
) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.arg(subcommand);

    for arg in args {
        cmd.arg(arg);
    }

    // Execute the command and capture output
    let output = cmd.output().map_err(|e| Error::GitCommand {
        message: format!("Failed to execute git command: {}", e),
    })?;

    // Check if command was successful
    if !output.status.success() {
        // Print stderr directly without pagination
        if !output.stderr.is_empty() {
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        }
        return Err(Error::GitCommand {
            message: format!(
                "git {} failed with exit code: {}",
                subcommand,
                output.status.code().unwrap_or(-1)
            ),
        });
    }

    // Combine stdout and stderr
    let mut all_output = String::from_utf8_lossy(&output.stdout).to_string();
    if !output.stderr.is_empty() {
        all_output.push_str(&String::from_utf8_lossy(&output.stderr));
    }

    // Split into lines and paginate
    let lines: Vec<String> = all_output.lines().map(|s| s.to_string()).collect();
    paginate_output(lines.into_iter(), lines_per_page, initial_lines)?;

    Ok(())
}

/// Gets the author date of the HEAD commit
///
/// Returns None if not in a git repository or if there are no commits.
///
/// # Examples
///
/// ```no_run
/// use gt::util::get_head_commit_date;
///
/// if let Some(date) = get_head_commit_date().unwrap() {
///     println!("HEAD commit date: {}", date);
/// }
/// ```
pub fn get_head_commit_date() -> Result<Option<DateTime<Utc>>> {
    let mut cmd = Command::new("git");
    cmd.arg("log");
    cmd.arg("-1");
    cmd.arg("--format=%aI");  // Author date in ISO 8601 format
    cmd.arg("HEAD");

    let output = cmd.output().map_err(|e| Error::GitCommand {
        message: format!("Failed to get HEAD commit date: {}", e),
    })?;

    if !output.status.success() {
        // Not a git repo or no commits
        return Ok(None);
    }

    let date_str = String::from_utf8_lossy(&output.stdout);
    let date_str = date_str.trim();

    if date_str.is_empty() {
        return Ok(None);
    }

    // Parse the ISO 8601 date
    match DateTime::parse_from_rfc3339(date_str) {
        Ok(dt) => Ok(Some(dt.with_timezone(&Utc))),
        Err(e) => Err(Error::ConfigInvalid {
            message: format!("Failed to parse commit date '{}': {}", date_str, e),
        }),
    }
}

/// Generates a random date/time within 2 hours after the given date
///
/// # Examples
///
/// ```
/// use gt::util::random_date_after;
/// use chrono::Utc;
///
/// let base_date = Utc::now();
/// let random_date = random_date_after(&base_date);
/// assert!(random_date > base_date);
/// ```
pub fn random_date_after(base_date: &DateTime<Utc>) -> DateTime<Utc> {
    let mut rng = rand::thread_rng();

    // Random number of seconds between 1 second and 2 hours (7200 seconds)
    let random_seconds = rng.gen_range(1..=7200);

    *base_date + Duration::seconds(random_seconds)
}

/// Execute a generic git passthrough command
///
/// This function handles passthrough for git commands, delegating to
/// `execute_git_command` or `execute_git_command_paginated` for help.
///
/// # Arguments
///
/// * `command_name` - The git subcommand (e.g., "add", "pull", "fetch")
/// * `opts` - The GitPassthroughOpts containing args and help flag
///
/// # Returns
///
/// Returns an Output indicating success or passes through git errors
pub fn execute_git_passthrough(
    command_name: &str,
    opts: &crate::cli::args::GitPassthroughOpts,
) -> Result<crate::cli::output::Output> {
    use crate::cli::output::Output;

    // Handle help flag with pagination
    if opts.help {
        execute_git_command_paginated(
            command_name,
            &["--help".to_string()],
            20,
            None,
        )?;
        return Ok(Output::success(""));
    }

    // Execute git command with all arguments
    execute_git_command(command_name, &opts.args)?;

    Ok(Output::success(""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_path_home() {
        let result = expand_path(Path::new("~/.ssh/config"));
        assert!(result.is_ok());
        let expanded = result.unwrap();
        assert!(expanded.is_absolute());
        assert!(expanded.to_string_lossy().contains(".ssh"));
    }

    #[test]
    fn test_expand_path_absolute() {
        let result = expand_path(Path::new("/etc/hosts"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("/etc/hosts"));
    }

    #[test]
    fn test_validate_identity_name_valid() {
        assert!(validate_identity_name("work").is_ok());
        assert!(validate_identity_name("personal").is_ok());
        assert!(validate_identity_name("my-work").is_ok());
        assert!(validate_identity_name("a1").is_ok());
    }

    #[test]
    fn test_validate_identity_name_too_short() {
        assert!(validate_identity_name("a").is_err());
        assert!(validate_identity_name("").is_err());
    }

    #[test]
    fn test_validate_identity_name_invalid_chars() {
        assert!(validate_identity_name("my_work").is_err());
        assert!(validate_identity_name("my.work").is_err());
        assert!(validate_identity_name("my work").is_err());
    }

    #[test]
    fn test_validate_identity_name_starts_with_number() {
        assert!(validate_identity_name("1work").is_err());
    }

    #[test]
    fn test_validate_identity_name_reserved() {
        assert!(validate_identity_name("gt-work").is_err());
        assert!(validate_identity_name("GT-test").is_err());
    }

    #[test]
    fn test_looks_like_secret() {
        assert!(looks_like_secret("my_password"));
        assert!(looks_like_secret("API_KEY_12345"));
        assert!(!looks_like_secret("work@company.com"));
    }

    #[test]
    fn test_parse_shorthand_date_now() {
        let result = parse_shorthand_date("now");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_shorthand_date_hours_ago() {
        let result = parse_shorthand_date("-1h");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_shorthand_date_days_from_now() {
        let result = parse_shorthand_date("2d");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_shorthand_date_weeks() {
        let result = parse_shorthand_date("-1w");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_shorthand_date_minutes() {
        let result = parse_shorthand_date("30m");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_shorthand_date_seconds() {
        let result = parse_shorthand_date("45s");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_shorthand_date_invalid() {
        assert!(parse_shorthand_date("invalid").is_err());
        assert!(parse_shorthand_date("1x").is_err());
        assert!(parse_shorthand_date("abc").is_err());
    }
}
