//! CLI module for DotState command-line interface.
//!
//! This module provides a modular structure for CLI commands.
//! The legacy CLI implementation is re-exported for backwards compatibility
//! while new commands are added incrementally.

mod common;
mod legacy;

// Re-export common utilities for use by CLI commands
pub use common::*;

// Re-export legacy CLI for backwards compatibility
// This includes Cli struct, Commands enum, and all existing command implementations
pub use legacy::{Cli, Commands};
