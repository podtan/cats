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
//! - **LLM Integration**: JSON conversion, tool execution, result handling for LLM providers
//!
//! ## Tool Sets
//!
//! CATS supports multiple tool sets via feature flags:
//!
//! - `old` (default): Original CATS tools
//! - `opencode`: OpenCode-compatible tools (coming soon)
//! - `gemini-cli`: Google Gemini CLI tools (coming soon)
//! - `claude-code`: Claude Code-compatible tools (coming soon)
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
pub mod llm;
pub mod tools;

// Conditional module exports for backward compatibility
#[cfg(feature = "old")]
pub use tools::old as editing;
#[cfg(feature = "old")]
pub use tools::old as execution;
#[cfg(feature = "old")]
pub use tools::old as file_navigation;
#[cfg(feature = "old")]
pub use tools::old as linting;
#[cfg(feature = "old")]
pub use tools::old as search;
#[cfg(feature = "old")]
pub use tools::old as state;
#[cfg(feature = "old")]
pub use tools::old as utils;

// Re-export main types
pub use core::{Tool, ToolArgs, ToolRegistry, ToolResult};

// Re-export tools from the active tool set
#[cfg(feature = "old")]
pub use tools::old::{
    // Editing tools
    CopyPathTool, CreateDirectoryTool, CreateFileTool, DeleteFunctionTool, DeleteLineTool,
    DeletePathTool, DeleteTextTool, InsertTextTool, MovePathTool, OverwriteFileTool,
    ReplaceTextTool,
    // Execution tools
    RunCommandTool,
    // File navigation tools
    CreateTool, GotoTool, OpenTool, ScrollTool, WindowedFile,
    // Search tools
    ConfigurableFilter, FindFileTool, SearchDirTool, SearchFileTool,
    // State tools
    StateTool, ToolState,
    // Utility tools
    ClassifyTaskTool, CountTokensTool, FilemapTool, SubmitTool,
};

// Re-export LLM integration
pub use llm::{
    assistant::{generate_assistant_content, ToolCallInfo},
    converter::json_to_tool_args,
    executor::{
        execute_tool_calls, execute_tool_calls_structured, ExecutionCallback, NoOpCallback,
        ToolCallRequest, ToolExecutionResult,
    },
    result_handler::{handle_large_result, ResultHandlerConfig},
};

// Re-export create_tool_registry functions from the active tool set
#[cfg(feature = "old")]
pub use tools::old::{create_tool_registry, create_tool_registry_with_open_window_size};

#[cfg(feature = "opencode")]
pub use tools::opencode::{create_tool_registry, create_tool_registry_with_open_window_size};

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
