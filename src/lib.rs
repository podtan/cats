//! # CATS - Coding Agent ToolS
//!
//! A comprehensive toolkit for building AI-powered coding agents.
//! This crate provides structured, LLM-friendly tools for software engineering tasks.
//!
//! ## Features
//!
//! - **File Navigation**: Windowed file viewing, line navigation, scrolling
//! - **Search Tools**: File discovery, content search across files and directories  
//! - **File Editing**: Search/replace editing with integrated linting
//! - **State Management**: Persistent tool state and session history
//! - **Utility Tools**: Project structure visualization, task submission
//!
//! ## Usage
//!
//! ```rust,no_run
//! use cats::{create_tool_registry, ToolArgs};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut registry = create_tool_registry();
//! let result = registry.execute_tool("_state", &ToolArgs::from_args(&[]))?;
//! println!("{}", result.message);
//! # Ok(())
//! # }
//! ```

pub mod core;
pub mod editing;
pub mod execution;
pub mod file_navigation;
pub mod linting;
pub mod search;
pub mod state;
pub mod utils;

// Re-export main types
pub use core::{Tool, ToolArgs, ToolRegistry, ToolResult};
pub use editing::{
    CopyPathTool, CreateDirectoryTool, CreateFileTool, DeleteFunctionTool, DeleteLineTool,
    DeletePathTool, DeleteTextTool, InsertTextTool, MovePathTool, OverwriteFileTool,
    ReplaceTextTool,
};
pub use execution::RunCommandTool;
pub use file_navigation::{CreateTool, GotoTool, OpenTool, ScrollTool, WindowedFile};
pub use search::{FindFileTool, SearchDirTool, SearchFileTool};
pub use state::{StateTool, ToolState};
pub use utils::{ClassifyTaskTool, CountTokensTool, FilemapTool, SubmitTool};

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
    registry.register(Box::new(OpenTool::new_with_open_window_size(
        open_window_size,
    )));
    registry.register(Box::new(GotoTool::new()));
    registry.register(Box::new(ScrollTool::new("scroll_up", true)));
    registry.register(Box::new(ScrollTool::new("scroll_down", false)));
    registry.register(Box::new(CreateTool::new()));

    // Search tools
    registry.register(Box::new(FindFileTool::new()));
    registry.register(Box::new(SearchFileTool::new()));
    registry.register(Box::new(SearchDirTool::new()));

    // Editing tools - New specialized tools
    registry.register(Box::new(CreateFileTool::new()));
    registry.register(Box::new(ReplaceTextTool::new()));
    registry.register(Box::new(InsertTextTool::new()));
    registry.register(Box::new(DeleteTextTool::new()));
    registry.register(Box::new(DeleteLineTool::new()));
    registry.register(Box::new(OverwriteFileTool::new()));
    registry.register(Box::new(
        editing::specialized_tools::DeleteFunctionTool::new(),
    ));

    // File management tools
    registry.register(Box::new(DeletePathTool::new()));
    registry.register(Box::new(MovePathTool::new()));
    registry.register(Box::new(CopyPathTool::new()));
    registry.register(Box::new(CreateDirectoryTool::new()));

    // State management
    registry.register(Box::new(StateTool::new()));

    // Utility tools
    registry.register(Box::new(CountTokensTool::new()));
    registry.register(Box::new(FilemapTool::new()));
    registry.register(Box::new(SubmitTool::new()));
    registry.register(Box::new(ClassifyTaskTool::new()));

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = create_tool_registry();

        // Test that all expected tools are registered
        let tool_names = registry.list_tools();

        // File navigation tools
        assert!(tool_names.contains(&"open".to_string()));
        assert!(tool_names.contains(&"goto".to_string()));
        assert!(tool_names.contains(&"scroll_up".to_string()));
        assert!(tool_names.contains(&"scroll_down".to_string()));
        assert!(tool_names.contains(&"create".to_string()));

        // Command execution tool
        assert!(tool_names.contains(&"run_command".to_string()));

        // Search tools
        assert!(tool_names.contains(&"find_file".to_string()));
        assert!(tool_names.contains(&"search_file".to_string()));
        assert!(tool_names.contains(&"search_dir".to_string()));

        // Editing tools - New specialized tools
        assert!(tool_names.contains(&"create_file".to_string()));
        assert!(tool_names.contains(&"replace_text".to_string()));
        assert!(tool_names.contains(&"insert_text".to_string()));
        assert!(tool_names.contains(&"delete_text".to_string()));
        assert!(tool_names.contains(&"delete_line".to_string()));
        assert!(tool_names.contains(&"overwrite_file".to_string()));

        // File management tools
        assert!(tool_names.contains(&"delete_path".to_string()));
        assert!(tool_names.contains(&"move_path".to_string()));
        assert!(tool_names.contains(&"copy_path".to_string()));
        assert!(tool_names.contains(&"create_directory".to_string()));

        // State and utility tools
        assert!(tool_names.contains(&"_state".to_string()));
        assert!(tool_names.contains(&"count_tokens".to_string()));
        assert!(tool_names.contains(&"filemap".to_string()));
        assert!(tool_names.contains(&"submit".to_string()));
        assert!(tool_names.contains(&"classify_task".to_string()));
    }
}
