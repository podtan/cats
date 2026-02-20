//! State management for tools
//!
//! Maintains context of currently open files, cursor positions, and session history

// Re-export state types from crate-level state module
pub use crate::state::{FileState, StateSnapshot, ToolState};

use crate::core::{Tool, ToolArgs, ToolError, ToolResult};
use anyhow::Result;
use std::sync::{Arc, Mutex};

/// Tool for displaying and managing state
pub struct StateTool {
    name: String,
}

impl StateTool {
    pub fn new() -> Self {
        Self {
            name: "_state".to_string(),
        }
    }
}

impl Tool for StateTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Display current state of open files and tool context"
    }

    fn signature(&self) -> &str {
        "_state"
    }

    fn validate_args(&self, _args: &ToolArgs) -> Result<(), ToolError> {
        Ok(()) // State tool takes no arguments
    }

    fn execute(&mut self, _args: &ToolArgs, state: &Arc<Mutex<ToolState>>) -> Result<ToolResult> {
        let state = state
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;
        let summary = state.get_summary();

        // Include current window content if there's an open file
        let mut full_response = format!("Current tool state:\n\n{}", summary);

        if let Some(file_state) = state.get_current_file_state() {
            let window_content = file_state.get_window_with_line_numbers();
            if !window_content.is_empty() {
                full_response.push_str("\n\nCurrent window:\n");
                full_response.push_str(&window_content.join("\n"));
            }
        }

        Ok(ToolResult::success_with_data(
            full_response,
            serde_json::json!({
                "summary": summary,
                "working_directory": state.working_directory,
                "current_file": state.current_file,
                "open_files": state.open_files.keys().collect::<Vec<_>>(),
                "history_count": state.history.len()
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_state_creation() {
        let content = vec![
            "line1".to_string(),
            "line2".to_string(),
            "line3".to_string(),
        ];
        let file_state = FileState::new(content, 2);

        assert_eq!(file_state.window_size, 2);
        assert_eq!(file_state.window_start, 0);
        assert_eq!(file_state.total_lines(), 3);
        assert!(!file_state.modified);
    }

    #[test]
    fn test_file_state_windowing() {
        let content: Vec<String> = (1..=10).map(|i| format!("line {}", i)).collect();
        let mut file_state = FileState::new(content, 3);

        // Test initial window
        let window = file_state.get_window();
        assert_eq!(window.len(), 3);
        assert_eq!(window[0], "line 1");
        assert_eq!(window[2], "line 3");

        // Test scroll down
        file_state.scroll_down();
        let window = file_state.get_window();
        assert_eq!(window[0], "line 4");

        // Test goto line
        file_state.goto_line(8);
        assert!(file_state.window_start >= 5); // Should center around line 8
    }

    #[test]
    fn test_tool_state_file_management() {
        let mut state = ToolState::new();
        let path = PathBuf::from("test.txt");
        let content = vec!["line1".to_string(), "line2".to_string()];

        // Test opening file
        state.open_file(path.clone(), content, 10).unwrap();
        assert_eq!(state.current_file, Some(path.clone()));
        assert!(state.open_files.contains_key(&path));

        // Test getting current file state
        let file_state = state.get_current_file_state().unwrap();
        assert_eq!(file_state.total_lines(), 2);

        // Test closing file
        state.close_file(&path);
        assert!(!state.open_files.contains_key(&path));
        assert_eq!(state.current_file, None);
    }

    #[test]
    fn test_state_tool_execution() {
        let mut tool = StateTool::new();
        let state = Arc::new(Mutex::new(ToolState::new()));
        let args = ToolArgs::from_args(&[]);

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);
        assert!(result.data.is_some());
    }

    use std::path::PathBuf;
}
