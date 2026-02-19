//! OpenCode-compatible tool set (coming soon)
//!
//! This module will provide tools compatible with OpenCode's tool interface.

// TODO: Implement OpenCode tools
// For now, this is a placeholder

use crate::core::ToolRegistry;

/// Initialize the tool registry with OpenCode-compatible tools
pub fn create_tool_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    // TODO: Add OpenCode tools
    registry
}

/// Initialize the tool registry with OpenCode-compatible tools and custom window size
pub fn create_tool_registry_with_open_window_size(
    _open_window_size: Option<usize>,
) -> ToolRegistry {
    create_tool_registry()
}
