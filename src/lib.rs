//! gt library - Core functionality for Git identity management
//!
//! This library provides the core functionality for gt, including:
//! - Identity management
//! - Strategy implementations (SSH alias, conditional, URL rewrite)
//! - Configuration parsing and writing
//! - SSH and Git config manipulation
//!
//! # Example
//!
//! ```rust,no_run
//! use gt::core::identity::Identity;
//! use gt::strategy::{Strategy, StrategyType};
//!
//! // Create an identity
//! let identity = Identity::builder("work")
//!     .email("work@company.com")
//!     .name("Work User")
//!     .provider("github")
//!     .build()?;
//!
//! // Apply using SSH alias strategy
//! let strategy = Strategy::create(StrategyType::SshAlias);
//! strategy.apply(&identity, &repo)?;
//! # Ok::<(), gt::error::Error>(())
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod cli;
pub mod cmd;
pub mod core;
pub mod error;
pub mod io;
pub mod scan;
pub mod strategy;
pub mod util;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::core::identity::Identity;
    pub use crate::core::provider::Provider;
    pub use crate::core::repo::Repo;
    pub use crate::error::{Error, Result};
    pub use crate::strategy::{Strategy, StrategyType};
}
