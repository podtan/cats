//! Tool sets for CATS
//!
//! This module provides different tool sets that can be selected via feature flags:
//!
//! - `old` (default): Original CATS tools
//! - `opencode`: OpenCode-compatible tools (coming soon)
//! - `gemini-cli`: Google Gemini CLI tools (coming soon)
//! - `claude-code`: Claude Code-compatible tools (coming soon)

#[cfg(feature = "old")]
pub mod old;

#[cfg(feature = "opencode")]
pub mod opencode;

// Re-export based on feature flag
#[cfg(feature = "old")]
pub use old::*;

#[cfg(feature = "opencode")]
pub use opencode::*;

// Provide a unified create_tool_registry function
#[cfg(feature = "old")]
pub use old::create_tool_registry;

#[cfg(feature = "opencode")]
pub use opencode::create_tool_registry;

// Provide a unified create_tool_registry_with_open_window_size function
#[cfg(feature = "old")]
pub use old::create_tool_registry_with_open_window_size;

#[cfg(feature = "opencode")]
pub use opencode::create_tool_registry_with_open_window_size;
