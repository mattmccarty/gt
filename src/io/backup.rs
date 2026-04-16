//! Backup management for gitid
//!
//! This module handles creating and rotating backups of configuration files.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::util::backup_timestamp;

/// Backup manager for configuration files
pub struct BackupManager {
    /// Maximum number of backups to keep per file
    max_backups: usize,
    /// Backup directory (None = same as original file)
    backup_dir: Option<PathBuf>,
    /// Whether backups are enabled
    enabled: bool,
}

impl BackupManager {
    /// Create a new backup manager
    #[must_use]
    pub fn new(max_backups: usize) -> Self {
        Self {
            max_backups,
            backup_dir: None,
            enabled: true,
        }
    }

    /// Set the backup directory
    #[must_use]
    pub fn with_backup_dir(mut self, dir: PathBuf) -> Self {
        self.backup_dir = Some(dir);
        self
    }

    /// Disable backups
    #[must_use]
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Create a backup of a file
    ///
    /// Returns the path to the backup file, or None if backups are disabled.
    pub fn backup(&self, original: &Path) -> Result<Option<PathBuf>> {
        if !self.enabled {
            return Ok(None);
        }

        if !original.exists() {
            return Ok(None);
        }

        let backup_dir = self
            .backup_dir
            .clone()
            .or_else(|| original.parent().map(PathBuf::from))
            .ok_or(Error::BackupFailed {
                path: original.to_owned(),
                message: "Cannot determine backup directory".to_string(),
            })?;

        // Ensure backup directory exists
        std::fs::create_dir_all(&backup_dir)?;

        // Generate backup filename
        let original_name = original
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or(Error::BackupFailed {
                path: original.to_owned(),
                message: "Invalid filename".to_string(),
            })?;

        let backup_name = format!("{}.{}.bak", original_name, backup_timestamp());
        let backup_path = backup_dir.join(&backup_name);

        // Copy file
        std::fs::copy(original, &backup_path)?;

        // Set secure permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&backup_path, perms)?;
        }

        // Rotate old backups
        self.rotate(&backup_dir, original_name)?;

        Ok(Some(backup_path))
    }

    /// Rotate old backups, keeping only the most recent ones
    fn rotate(&self, backup_dir: &Path, original_name: &str) -> Result<()> {
        let pattern = format!("{}.*.bak", original_name);

        let mut backups: Vec<_> = std::fs::read_dir(backup_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .map_or(false, |n| n.starts_with(original_name) && n.ends_with(".bak"))
            })
            .collect();

        // Sort by modification time (newest first)
        backups.sort_by(|a, b| {
            let a_time = a.metadata().and_then(|m| m.modified()).ok();
            let b_time = b.metadata().and_then(|m| m.modified()).ok();
            b_time.cmp(&a_time)
        });

        // Remove excess backups
        for backup in backups.into_iter().skip(self.max_backups) {
            let _ = std::fs::remove_file(backup.path());
            log::debug!("Removed old backup: {}", backup.path().display());
        }

        Ok(())
    }

    /// List all backups for a file
    pub fn list_backups(&self, original: &Path) -> Result<Vec<PathBuf>> {
        let backup_dir = self
            .backup_dir
            .clone()
            .or_else(|| original.parent().map(PathBuf::from))
            .ok_or(Error::BackupFailed {
                path: original.to_owned(),
                message: "Cannot determine backup directory".to_string(),
            })?;

        let original_name = original
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or(Error::BackupFailed {
                path: original.to_owned(),
                message: "Invalid filename".to_string(),
            })?;

        let mut backups: Vec<_> = std::fs::read_dir(&backup_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .map_or(false, |n| n.starts_with(original_name) && n.ends_with(".bak"))
            })
            .map(|e| e.path())
            .collect();

        // Sort by modification time (newest first)
        backups.sort_by(|a, b| {
            let a_time = a.metadata().and_then(|m| m.modified()).ok();
            let b_time = b.metadata().and_then(|m| m.modified()).ok();
            b_time.cmp(&a_time)
        });

        Ok(backups)
    }

    /// Restore a file from a backup
    pub fn restore(&self, original: &Path, backup: &Path) -> Result<()> {
        if !backup.exists() {
            return Err(Error::BackupFailed {
                path: backup.to_owned(),
                message: "Backup file not found".to_string(),
            });
        }

        // Create a backup of the current file first
        self.backup(original)?;

        // Copy backup to original
        std::fs::copy(backup, original)?;

        Ok(())
    }
}

impl Default for BackupManager {
    fn default() -> Self {
        Self::new(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_backup_creation() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("config");
        std::fs::write(&file_path, "test content").unwrap();

        let manager = BackupManager::new(2);
        let backup = manager.backup(&file_path).unwrap();

        assert!(backup.is_some());
        let backup_path = backup.unwrap();
        assert!(backup_path.exists());
        assert!(backup_path.to_string_lossy().contains(".bak"));
    }

    #[test]
    fn test_backup_rotation() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("config");

        let manager = BackupManager::new(2);

        // Create 3 backups
        for i in 0..3 {
            std::fs::write(&file_path, format!("content {}", i)).unwrap();
            manager.backup(&file_path).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Should only have 2 backups
        let backups = manager.list_backups(&file_path).unwrap();
        assert_eq!(backups.len(), 2);
    }

    #[test]
    fn test_backup_disabled() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("config");
        std::fs::write(&file_path, "test content").unwrap();

        let manager = BackupManager::new(2).disabled();
        let backup = manager.backup(&file_path).unwrap();

        assert!(backup.is_none());
    }
}
