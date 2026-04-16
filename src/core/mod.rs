//! Core domain logic for gitid
//!
//! This module contains the core business logic and domain models:
//! - Identity model and operations
//! - Repository detection and manipulation
//! - URL parsing and transformation
//! - Provider definitions
//! - Cross-platform path utilities

pub mod identity;
pub mod path;
pub mod provider;
pub mod repo;
pub mod url;
