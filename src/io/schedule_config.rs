//! Push schedule configuration
//!
//! This module manages the global and local storage of scheduled push times
//! for repositories with future-dated commits.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::util::config_dir;

/// Schedule status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ScheduleStatus {
    /// Schedule is pending execution
    Pending,
    /// Schedule execution failed
    Failed,
}

/// A scheduled push for a specific repository and branch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    /// Absolute path to the repository
    pub repo_path: PathBuf,

    /// Repository identifier (derived from remote URL, e.g., "github.com/user/repo")
    pub repo_id: String,

    /// Branch name
    pub branch: String,

    /// Remote name (e.g., "origin")
    pub remote: String,

    /// Scheduled push time (UTC)
    pub scheduled_time: DateTime<Utc>,

    /// Commit SHA that triggered the schedule
    pub commit_sha: String,

    /// When this schedule was created (UTC)
    pub created_at: DateTime<Utc>,

    /// Current status of the schedule
    pub status: ScheduleStatus,

    /// Last attempt time (if any)
    pub last_attempt: Option<DateTime<Utc>>,

    /// Failure reason (if status is Failed)
    pub failure_reason: Option<String>,

    /// Number of attempts made
    pub attempt_count: u32,

    /// SSH_AUTH_SOCK at schedule time (for SSH auth)
    pub ssh_auth_sock: Option<String>,
}

/// Global schedule configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScheduleConfig {
    /// All scheduled pushes
    pub schedules: Vec<Schedule>,
}

impl ScheduleConfig {
    /// Load schedule config from the global location
    pub fn load() -> Result<Self> {
        let path = Self::global_path()?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&path).map_err(|e| Error::Io(e))?;
        let config: ScheduleConfig = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Save schedule config to the global location
    pub fn save(&self) -> Result<()> {
        let path = Self::global_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)?;
        fs::write(&path, contents)?;

        Ok(())
    }

    /// Get the global schedule config path
    pub fn global_path() -> Result<PathBuf> {
        Ok(config_dir()?.join("scheduled-pushes.toml"))
    }

    /// Add a schedule
    pub fn add_schedule(&mut self, schedule: Schedule) {
        // Remove any existing schedule for the same repo/branch
        self.schedules.retain(|s| {
            s.repo_path != schedule.repo_path || s.branch != schedule.branch
        });

        self.schedules.push(schedule);
    }

    /// Remove a schedule by repo path and branch
    pub fn remove_schedule(&mut self, repo_path: &Path, branch: &str) -> bool {
        let initial_len = self.schedules.len();
        self.schedules.retain(|s| {
            s.repo_path != repo_path || s.branch != branch
        });
        self.schedules.len() < initial_len
    }

    /// Get a schedule by repo path and branch
    pub fn get_schedule(&self, repo_path: &Path, branch: &str) -> Option<&Schedule> {
        self.schedules.iter().find(|s| {
            s.repo_path == repo_path && s.branch == branch
        })
    }

    /// Get a mutable schedule by repo path and branch
    pub fn get_schedule_mut(&mut self, repo_path: &Path, branch: &str) -> Option<&mut Schedule> {
        self.schedules.iter_mut().find(|s| {
            s.repo_path == repo_path && s.branch == branch
        })
    }

    /// List all schedules
    pub fn list_schedules(&self) -> &[Schedule] {
        &self.schedules
    }

    /// Clean up old completed/failed schedules
    pub fn cleanup_old(&mut self, days: i64) {
        let cutoff = Utc::now() - chrono::Duration::days(days);
        self.schedules.retain(|s| {
            // Keep pending schedules
            if s.status == ScheduleStatus::Pending {
                return true;
            }
            // Keep recent failed schedules
            if let Some(last_attempt) = s.last_attempt {
                last_attempt > cutoff
            } else {
                s.created_at > cutoff
            }
        });
    }
}

/// Local schedule cache (stored in .git/gt-push-schedule)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalScheduleCache {
    /// Scheduled push time for the current branch
    pub scheduled_time: DateTime<Utc>,

    /// Branch name
    pub branch: String,

    /// Commit SHA that triggered the schedule
    pub commit_sha: String,

    /// Remote name
    pub remote: String,
}

impl LocalScheduleCache {
    /// Get the local cache path for a repository
    pub fn local_path(repo_path: &Path) -> PathBuf {
        repo_path.join(".git").join("gt-push-schedule")
    }

    /// Load local schedule cache
    pub fn load(repo_path: &Path) -> Result<Option<Self>> {
        let path = Self::local_path(repo_path);

        if !path.exists() {
            return Ok(None);
        }

        let contents = fs::read_to_string(&path)?;
        let cache: LocalScheduleCache = toml::from_str(&contents)?;
        Ok(Some(cache))
    }

    /// Save local schedule cache
    pub fn save(&self, repo_path: &Path) -> Result<()> {
        let path = Self::local_path(repo_path);

        // Ensure .git directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Err(Error::NotARepository);
            }
        }

        let contents = toml::to_string_pretty(self)?;
        fs::write(&path, contents)?;

        Ok(())
    }

    /// Remove local schedule cache
    pub fn remove(repo_path: &Path) -> Result<()> {
        let path = Self::local_path(repo_path);
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_config_default() {
        let config = ScheduleConfig::default();
        assert_eq!(config.schedules.len(), 0);
    }

    #[test]
    fn test_add_schedule() {
        let mut config = ScheduleConfig::default();

        let schedule = Schedule {
            repo_path: PathBuf::from("/tmp/repo"),
            repo_id: "github.com/user/repo".to_string(),
            branch: "main".to_string(),
            remote: "origin".to_string(),
            scheduled_time: Utc::now(),
            commit_sha: "abc123".to_string(),
            created_at: Utc::now(),
            status: ScheduleStatus::Pending,
            last_attempt: None,
            failure_reason: None,
            attempt_count: 0,
            ssh_auth_sock: None,
        };

        config.add_schedule(schedule.clone());
        assert_eq!(config.schedules.len(), 1);

        // Adding another schedule for the same repo/branch should replace
        config.add_schedule(schedule);
        assert_eq!(config.schedules.len(), 1);
    }

    #[test]
    fn test_remove_schedule() {
        let mut config = ScheduleConfig::default();

        let schedule = Schedule {
            repo_path: PathBuf::from("/tmp/repo"),
            repo_id: "github.com/user/repo".to_string(),
            branch: "main".to_string(),
            remote: "origin".to_string(),
            scheduled_time: Utc::now(),
            commit_sha: "abc123".to_string(),
            created_at: Utc::now(),
            status: ScheduleStatus::Pending,
            last_attempt: None,
            failure_reason: None,
            attempt_count: 0,
            ssh_auth_sock: None,
        };

        config.add_schedule(schedule);
        assert_eq!(config.schedules.len(), 1);

        let removed = config.remove_schedule(&PathBuf::from("/tmp/repo"), "main");
        assert!(removed);
        assert_eq!(config.schedules.len(), 0);
    }

    #[test]
    fn test_get_schedule() {
        let mut config = ScheduleConfig::default();

        let schedule = Schedule {
            repo_path: PathBuf::from("/tmp/repo"),
            repo_id: "github.com/user/repo".to_string(),
            branch: "main".to_string(),
            remote: "origin".to_string(),
            scheduled_time: Utc::now(),
            commit_sha: "abc123".to_string(),
            created_at: Utc::now(),
            status: ScheduleStatus::Pending,
            last_attempt: None,
            failure_reason: None,
            attempt_count: 0,
            ssh_auth_sock: None,
        };

        config.add_schedule(schedule);

        let found = config.get_schedule(&PathBuf::from("/tmp/repo"), "main");
        assert!(found.is_some());
        assert_eq!(found.unwrap().commit_sha, "abc123");

        let not_found = config.get_schedule(&PathBuf::from("/tmp/other"), "main");
        assert!(not_found.is_none());
    }
}
