//! CLI layer for gitid
//!
//! This module handles all command-line interface concerns:
//! - Argument parsing with clap
//! - Output formatting (terminal, JSON, CSV)
//! - Interactive prompts with dialoguer

pub mod args;
pub mod interactive;
pub mod output;
