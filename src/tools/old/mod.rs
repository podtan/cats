//! Original CATS tools (deprecated, for migration)
//!
//! This module contains the original tool implementations from CATS.
//! These are kept for backward compatibility during the migration to OpenCode-style tools.

pub mod editing;
pub mod execution;
pub mod file_navigation;
pub mod linting;
pub mod search;
pub mod state;
pub mod utils;

// Re-export main types for backward compatibility
pub use editing::{
    CopyPathTool, CreateDirectoryTool, CreateFileTool, DeleteFunctionTool, DeleteLineTool,
    DeletePathTool, DeleteTextTool, InsertTextTool, MovePathTool, OverwriteFileTool,
    ReplaceTextTool,
};
pub use execution::RunCommandTool;
pub use file_navigation::{CreateTool, GotoTool, OpenTool, ScrollTool, WindowedFile};
pub use search::{ConfigurableFilter, FindFileTool, SearchDirTool, SearchFileTool};
pub use state::{StateTool, ToolState};
pub use utils::{ClassifyTaskTool, CountTokensTool, FilemapTool, SubmitTool};

use crate::core::ToolRegistry;

/// Initialize the tool registry with all available tools (backward-compatible)
pub fn create_tool_registry() -> ToolRegistry {
    create_tool_registry_with_open_window_size(None)
}

/// Initialize the tool registry with a configurable default window size for the "open" tool
pub fn create_tool_registry_with_open_window_size(open_window_size: Option<usize>) -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    // Command execution tool (NEW - replaces direct bash)
    registry.register(Box::new(execution::RunCommandTool::new()));

    // File navigation tools
    registry.register(Box::new(file_navigation::OpenTool::new_with_open_window_size(
        open_window_size,
    )));
    registry.register(Box::new(file_navigation::GotoTool::new()));
    registry.register(Box::new(file_navigation::ScrollTool::new("scroll_up", true)));
    registry.register(Box::new(file_navigation::ScrollTool::new("scroll_down", false)));
    registry.register(Box::new(file_navigation::CreateTool::new()));

    // Search tools
    registry.register(Box::new(search::FindFileTool::new()));
    registry.register(Box::new(search::SearchFileTool::new()));
    registry.register(Box::new(search::SearchDirTool::new()));

    // Editing tools - New specialized tools
    registry.register(Box::new(editing::CreateFileTool::new()));
    registry.register(Box::new(editing::ReplaceTextTool::new()));
    registry.register(Box::new(editing::InsertTextTool::new()));
    registry.register(Box::new(editing::DeleteTextTool::new()));
    registry.register(Box::new(editing::DeleteLineTool::new()));
    registry.register(Box::new(editing::OverwriteFileTool::new()));
    registry.register(Box::new(
        editing::specialized_tools::DeleteFunctionTool::new(),
    ));

    // File management tools
    registry.register(Box::new(editing::DeletePathTool::new()));
    registry.register(Box::new(editing::MovePathTool::new()));
    registry.register(Box::new(editing::CopyPathTool::new()));
    registry.register(Box::new(editing::CreateDirectoryTool::new()));

    // State management
    registry.register(Box::new(state::StateTool::new()));

    // Utility tools
    registry.register(Box::new(utils::CountTokensTool::new()));
    registry.register(Box::new(utils::FilemapTool::new()));
    registry.register(Box::new(utils::SubmitTool::new()));
    registry.register(Box::new(utils::ClassifyTaskTool::new()));

    registry
}
