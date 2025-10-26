//! State management for tools
//!
//! Maintains context of currently open files, cursor positions, and session history

use crate::core::{Tool, ToolArgs, ToolError, ToolResult};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Represents the state of the tool system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolState {
    /// Currently open files with their windowed views
    pub open_files: HashMap<PathBuf, FileState>,
    /// The currently active file
    pub current_file: Option<PathBuf>,
    /// Session history for undo/redo
    pub history: Vec<StateSnapshot>,
    /// Current working directory
    pub working_directory: PathBuf,
}

/// State of an individual file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileState {
    /// File content as lines
    pub content: Vec<String>,
    /// Current window start position (0-indexed line number)
    pub window_start: usize,
    /// Window size (number of lines to show)
    pub window_size: usize,
    /// File modification tracking
    pub modified: bool,
    /// Last modification timestamp
    pub last_modified: Option<std::time::SystemTime>,
}

impl FileState {
    /// Create new file state
    pub fn new(content: Vec<String>, window_size: usize) -> Self {
        Self {
            content,
            window_start: 0,
            window_size,
            modified: false,
            last_modified: None,
        }
    }

    /// Get the current window content
    pub fn get_window(&self) -> Vec<&String> {
        let end = std::cmp::min(self.window_start + self.window_size, self.content.len());
        self.content[self.window_start..end].iter().collect()
    }

    /// Get window with line numbers
    pub fn get_window_with_line_numbers(&self) -> Vec<String> {
        let end = std::cmp::min(self.window_start + self.window_size, self.content.len());
        self.content[self.window_start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{:4} | {}", self.window_start + i + 1, line))
            .collect()
    }

    /// Move window to show specific line
    pub fn goto_line(&mut self, line_number: usize) {
        if line_number == 0 {
            return;
        }
        let target_line = line_number.saturating_sub(1);
        if target_line < self.content.len() {
            // Center the target line in the window if possible
            let half_window = self.window_size / 2;
            self.window_start = target_line.saturating_sub(half_window);

            // Ensure we don't go past the end
            let max_start = self.content.len().saturating_sub(self.window_size);
            if self.window_start > max_start {
                self.window_start = max_start;
            }
        }
    }

    /// Scroll window up
    pub fn scroll_up(&mut self) {
        self.window_start = self.window_start.saturating_sub(self.window_size);
    }

    /// Scroll window down  
    pub fn scroll_down(&mut self) {
        let max_start = self.content.len().saturating_sub(self.window_size);
        self.window_start = std::cmp::min(self.window_start + self.window_size, max_start);
    }

    /// Get total number of lines
    pub fn total_lines(&self) -> usize {
        self.content.len()
    }

    /// Check if window is at the beginning
    pub fn is_at_start(&self) -> bool {
        self.window_start == 0
    }

    /// Check if window is at the end
    pub fn is_at_end(&self) -> bool {
        self.window_start + self.window_size >= self.content.len()
    }
}

/// Snapshot of the tool state for history tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub timestamp: std::time::SystemTime,
    pub current_file: Option<PathBuf>,
    pub operation: String,
    pub file_states: HashMap<PathBuf, FileState>,
}

impl ToolState {
    /// Create a new tool state
    pub fn new() -> Self {
        Self {
            open_files: HashMap::new(),
            current_file: None,
            history: Vec::new(),
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Open a file and add it to the state
    pub fn open_file(
        &mut self,
        path: PathBuf,
        content: Vec<String>,
        window_size: usize,
    ) -> Result<()> {
        let file_state = FileState::new(content, window_size);
        self.open_files.insert(path.clone(), file_state);
        self.current_file = Some(path);
        Ok(())
    }

    /// Get the current file state
    pub fn get_current_file_state(&self) -> Option<&FileState> {
        self.current_file
            .as_ref()
            .and_then(|path| self.open_files.get(path))
    }

    /// Get mutable current file state
    pub fn get_current_file_state_mut(&mut self) -> Option<&mut FileState> {
        let current_file = self.current_file.clone()?;
        self.open_files.get_mut(&current_file)
    }

    /// Switch to a different open file
    pub fn switch_to_file(&mut self, path: &PathBuf) -> Result<(), ToolError> {
        if self.open_files.contains_key(path) {
            self.current_file = Some(path.clone());
            Ok(())
        } else {
            Err(ToolError::FileNotFound {
                path: path.to_string_lossy().to_string(),
            })
        }
    }

    /// Close a file
    pub fn close_file(&mut self, path: &PathBuf) {
        self.open_files.remove(path);
        if self.current_file.as_ref() == Some(path) {
            self.current_file = self.open_files.keys().next().cloned();
        }
    }

    /// Create a snapshot for history
    pub fn create_snapshot(&self, operation: String) -> StateSnapshot {
        StateSnapshot {
            timestamp: std::time::SystemTime::now(),
            current_file: self.current_file.clone(),
            operation,
            file_states: self.open_files.clone(),
        }
    }

    /// Add to history
    pub fn push_history(&mut self, operation: String) {
        let snapshot = self.create_snapshot(operation);
        self.history.push(snapshot);

        // Keep history size manageable
        const MAX_HISTORY: usize = 100;
        if self.history.len() > MAX_HISTORY {
            self.history.remove(0);
        }
    }

    /// Get state summary for display
    pub fn get_summary(&self) -> String {
        let mut summary = String::new();

        summary.push_str(&format!(
            "Working Directory: {}\n",
            self.working_directory.display()
        ));

        if let Some(current) = &self.current_file {
            summary.push_str(&format!("Current File: {}\n", current.display()));

            if let Some(file_state) = self.open_files.get(current) {
                summary.push_str(&format!(
                    "Lines: {} | Window: {}-{} (size: {})\n",
                    file_state.total_lines(),
                    file_state.window_start + 1,
                    std::cmp::min(
                        file_state.window_start + file_state.window_size,
                        file_state.total_lines()
                    ),
                    file_state.window_size
                ));
            }
        } else {
            summary.push_str("No file currently open\n");
        }

        if !self.open_files.is_empty() {
            summary.push_str("\nOpen Files:\n");
            for (path, file_state) in &self.open_files {
                let modified = if file_state.modified {
                    " (modified)"
                } else {
                    ""
                };
                summary.push_str(&format!(
                    "  {} - {} lines{}\n",
                    path.display(),
                    file_state.total_lines(),
                    modified
                ));
            }
        }

        summary
    }
}

impl Default for ToolState {
    fn default() -> Self {
        Self::new()
    }
}

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
}
