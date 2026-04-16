//! I/O operations for gitid
//!
//! This module handles file system operations:
//! - SSH config parsing and writing
//! - Git config parsing and writing
//! - SSH key generation and management
//! - Backup management
//! - TOML configuration

pub mod backup;
pub mod git_config;
pub mod git_hooks;
pub mod schedule_config;
pub mod ssh_config;
pub mod ssh_key;
pub mod toml_config;
