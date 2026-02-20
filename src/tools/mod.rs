//! Tool sets for CATS
//!
//! This module provides different tool sets that can be selected via feature flags:
//!
//! - `old` (default): Original CATS tools
//! - `opencode`: OpenCode-compatible tools
//! - `gemini-cli`: Google Gemini CLI tools (coming soon)
//! - `claude-code`: Claude Code-compatible tools (coming soon)

#[cfg(feature = "old")]
pub mod old;

#[cfg(feature = "opencode")]
pub mod opencode;

// Re-export based on feature flag (prefer opencode if both are enabled)
#[cfg(all(feature = "old", not(feature = "opencode")))]
pub use old::*;

#[cfg(feature = "opencode")]
pub use opencode::*;
