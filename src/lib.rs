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

// `missing_docs` and `clippy::pedantic` are allowed pre-1.0 while the public
// API is still shifting. The pedantic group subsumes `missing_errors_doc`,
// `missing_panics_doc`, `must_use_candidate`, `uninlined_format_args`, and
// many other subjective lints that collectively produce hundreds of warnings
// on the current codebase. Re-enable at 1.0 when the surface freezes and
// back-fill docs and pedantic cleanups at that time. See issue #17 for
// history.
#![allow(missing_docs)]
#![warn(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
// Inherent `to_string`, `from_str`, and `default` methods on a handful of
// types shadow the standard `Display`, `FromStr`, and `Default` traits. The
// types are still in flux pre-1.0; converting them requires API decisions
// (infallible `Default`, `FromStr::Err` type, etc.) that are deferred until
// the surface freezes. Tracked alongside the rest of issue #17.
#![allow(clippy::inherent_to_string)]
#![allow(clippy::should_implement_trait)]

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
