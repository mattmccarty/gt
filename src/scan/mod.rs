//! Detection and scanning functionality
//!
//! This module handles detecting existing configurations and identities.

pub mod detector;
pub mod git_scanner;
pub mod report;
pub mod ssh_scanner;

pub use detector::*;
pub use report::*;
