//! OpenCode-compatible tool set
//!
//! This module provides tools compatible with OpenCode's tool interface.
//! These tools follow the same patterns and behaviors as the original OpenCode implementation.

mod bash;
mod edit;
mod glob;
mod grep;
mod list;
mod read;
mod write;

pub use bash::BashTool;
pub use edit::EditTool;
pub use glob::GlobTool;
pub use grep::GrepTool;
pub use list::ListTool;
pub use read::ReadTool;
pub use write::WriteTool;

use crate::core::ToolRegistry;

/// Initialize the tool registry with OpenCode-compatible tools
pub fn create_tool_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    registry.register(Box::new(BashTool::new()));
    registry.register(Box::new(ReadTool::new()));
    registry.register(Box::new(WriteTool::new()));
    registry.register(Box::new(EditTool::new()));
    registry.register(Box::new(GlobTool::new()));
    registry.register(Box::new(GrepTool::new()));
    registry.register(Box::new(ListTool::new()));

    registry
}

/// Initialize the tool registry with OpenCode-compatible tools and custom window size
pub fn create_tool_registry_with_open_window_size(
    _open_window_size: Option<usize>,
) -> ToolRegistry {
    create_tool_registry()
}
