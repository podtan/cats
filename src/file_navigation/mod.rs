//! File navigation tools
//!
//! Provides windowed file viewing, line navigation, and file creation

use crate::core::{Tool, ToolArgs, ToolError, ToolResult};
use crate::state::ToolState;
use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Default window size used by other windowing utilities
pub const DEFAULT_WINDOW_SIZE: usize = 100;

/// Default window size specifically for the "open" tool (configurable via simpaticoder.toml)
pub const OPEN_TOOL_DEFAULT_WINDOW_SIZE: usize = 1000;

/// Windowed file representation following SWE-agent pattern
#[derive(Debug, Clone)]
pub struct WindowedFile {
    pub path: PathBuf,
    pub content: Vec<String>,
    pub window_start: usize,
    pub window_size: usize,
}

impl WindowedFile {
    /// Create a new windowed file
    pub fn new(path: PathBuf, window_size: Option<usize>) -> Result<Self, ToolError> {
        let content = fs::read_to_string(&path).map_err(|_| ToolError::FileNotFound {
            path: path.to_string_lossy().to_string(),
        })?;

        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        Ok(Self {
            path,
            content: lines,
            window_start: 0,
            window_size: window_size.unwrap_or(DEFAULT_WINDOW_SIZE),
        })
    }

    /// Create from existing content
    pub fn from_content(path: PathBuf, content: Vec<String>, window_size: Option<usize>) -> Self {
        Self {
            path,
            content,
            window_start: 0,
            window_size: window_size.unwrap_or(DEFAULT_WINDOW_SIZE),
        }
    }

    /// Get current window content with line numbers
    pub fn get_window_display(&self) -> String {
        let end = std::cmp::min(self.window_start + self.window_size, self.content.len());
        let window_content = &self.content[self.window_start..end];

        let mut result = String::new();
        result.push_str(&format!(
            "File: {} ({} lines)\n",
            self.path.display(),
            self.content.len()
        ));
        result.push_str(&format!(
            "Lines {}-{} of {}:\n",
            self.window_start + 1,
            end,
            self.content.len()
        ));
        result.push_str(&"-".repeat(80));
        result.push('\n');

        for (i, line) in window_content.iter().enumerate() {
            result.push_str(&format!("{:4} | {}\n", self.window_start + i + 1, line));
        }

        result.push_str(&"-".repeat(80));
        result.push('\n');

        if self.window_start > 0 {
            result.push_str("(Use scroll_up to see previous lines)\n");
        }
        if end < self.content.len() {
            result.push_str("(Use scroll_down to see more lines)\n");
        }

        result
    }
}

/// Tool for opening files
pub struct OpenTool {
    name: String,
    /// Default window size for the open tool (if None, use OPEN_TOOL_DEFAULT_WINDOW_SIZE)
    default_window_size: Option<usize>,
}

impl OpenTool {
    /// Backward-compatible no-arg constructor (defaults to None)
    pub fn new() -> Self {
        Self {
            name: "open".to_string(),
            default_window_size: None,
        }
    }

    /// Create with explicit open tool default window size
    pub fn new_with_open_window_size(open_window_size: Option<usize>) -> Self {
        Self {
            name: "open".to_string(),
            default_window_size: open_window_size,
        }
    }

    /// Constructor that accepts an optional default window size for the open tool
    pub fn new_with_default(default_window_size: Option<usize>) -> Self {
        Self {
            name: "open".to_string(),
            default_window_size,
        }
    }
}

impl Tool for OpenTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Opens the file at the given path in the editor. If line_number is provided, the window will be moved to include that line"
    }

    fn signature(&self) -> &str {
        r#"open "<path>" [<line_number>]"#
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        if args.is_empty() {
            return Err(ToolError::InvalidArgs {
                message: "Usage: open \"<file>\" [<line_number>]".to_string(),
            });
        }
        Ok(())
    }

    fn execute(&mut self, args: &ToolArgs, state: &Arc<Mutex<ToolState>>) -> Result<ToolResult> {
        let path = args.get_arg(0).unwrap();
        let path_buf = PathBuf::from(path);

        // Check if path exists
        if !path_buf.exists() {
            return Ok(ToolResult::error(format!("File not found: {}", path)));
        }

        // Get line number if provided
        let line_number: Option<usize> = args.get_arg(1).and_then(|s| s.parse().ok());

        // Check if file is already open and return current state to prevent repetitive opening
        {
            let mut state_guard = state
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;
            if let Some(current_file) = &state_guard.current_file {
                if current_file == &path_buf {
                    // File is already open
                    if let Some(line_num) = line_number {
                        // Move current open file to requested line -- do mutations in an inner scope to avoid holding
                        // a mutable borrow while also calling state_guard.push_history later.
                        let mut moved_window: Option<Vec<String>> = None;
                        let mut total_lines: Option<usize> = None;

                        if let Some(file_state) = state_guard.get_current_file_state_mut() {
                            file_state.goto_line(line_num);
                            total_lines = Some(file_state.total_lines());
                            moved_window = Some(file_state.get_window_with_line_numbers());
                        }

                        // If requested line is beyond EOF, return a warning similar to open-path behavior
                        if let Some(tl) = total_lines {
                            if line_num > tl {
                                return Ok(ToolResult::success_with_data(
                                    format!("File {} is already open (moved to line {} of {}). Warning: Line number {} is greater than total lines.",
                                        path, line_num, tl, line_num),
                                    serde_json::json!({
                                        "path": path,
                                        "already_open": true,
                                        "total_lines": tl,
                                        "current_window": moved_window.unwrap_or_default()
                                    })
                                ));
                            }
                        }

                        // Safe to push history now; mutable borrow from get_current_file_state_mut() has ended
                        state_guard.push_history(format!("Moved to line {} in {}", line_num, path));

                        let current_window = moved_window.unwrap_or_default();
                        let result_message = format!(
                            "File {} is already open (moved to line {}):\n\n{}",
                            path,
                            line_num,
                            current_window.join("\n")
                        );

                        return Ok(ToolResult::success_with_data(
                            result_message,
                            serde_json::json!({
                                "path": path,
                                "already_open": true,
                                "total_lines": total_lines.unwrap_or(0),
                                "window_content": current_window
                            }),
                        ));
                    } else {
                        if let Some(file_state) = state_guard.get_current_file_state() {
                            let current_window = file_state.get_window_with_line_numbers();
                            let result_message = format!(
                                "File {} is already open (showing current window):\n\n{}",
                                path,
                                current_window.join("\n")
                            );

                            return Ok(ToolResult::success_with_data(
                                result_message,
                                serde_json::json!({
                                    "path": path,
                                    "already_open": true,
                                    "total_lines": file_state.total_lines(),
                                    "window_content": current_window
                                }),
                            ));
                        }
                    }
                }
            }
        }

        // Read file content
        let content = fs::read_to_string(&path_buf)
            .map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?;
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        // Get line number if provided
        let line_number: Option<usize> = args.get_arg(1).and_then(|s| s.parse().ok());

        // Determine the open tool's window size (configurable)
        let open_window = self
            .default_window_size
            .unwrap_or(OPEN_TOOL_DEFAULT_WINDOW_SIZE);

        // Create windowed file with the configured window size
        let windowed_file =
            WindowedFile::from_content(path_buf.clone(), lines.clone(), Some(open_window));

        // Update state
        {
            let mut state_guard = state
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;
            state_guard.open_file(path_buf.clone(), lines, open_window)?;

            // Move to specified line if provided
            if let Some(line_num) = line_number {
                if let Some(file_state) = state_guard.get_current_file_state_mut() {
                    file_state.goto_line(line_num);
                    if line_num > file_state.total_lines() {
                        return Ok(ToolResult::success_with_data(
                            format!("Opened {} (moved to line {} of {}). Warning: Line number {} is greater than total lines.", 
                                path, line_num, file_state.total_lines(), line_num),
                            serde_json::json!({
                                "path": path,
                                "total_lines": file_state.total_lines(),
                                "current_window": file_state.get_window_with_line_numbers()
                            })
                        ));
                    }
                }
            }

            state_guard.push_history(format!("Opened file: {}", path));
        }

        // Get the current window content for immediate display
        let state_guard = state
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;
        let current_window = if let Some(file_state) = state_guard.get_current_file_state() {
            file_state.get_window_with_line_numbers()
        } else {
            vec![]
        };
        drop(state_guard);

        // Create result message with file content
        let result_message = format!(
            "Opened file: {} ({} lines)\n\n{}",
            path,
            windowed_file.content.len(),
            current_window.join("\n")
        );

        Ok(ToolResult::success_with_data(
            result_message,
            serde_json::json!({
                "path": path,
                "total_lines": windowed_file.content.len(),
                "window_content": current_window
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the file to open"
                },
                "line_number": {
                    "type": "integer",
                    "description": "Optional line number to move the window to"
                }
            },
            "required": ["path"]
        })
    }
}

/// Tool for navigating to specific lines
pub struct GotoTool {
    name: String,
}

impl GotoTool {
    pub fn new() -> Self {
        Self {
            name: "goto".to_string(),
        }
    }
}

impl Tool for GotoTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Moves the window to show <line_number>"
    }

    fn signature(&self) -> &str {
        "goto <line_number>"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        if args.is_empty() {
            return Err(ToolError::InvalidArgs {
                message: "Usage: goto <line_number>".to_string(),
            });
        }

        if args.get_arg(0).unwrap().parse::<usize>().is_err() {
            return Err(ToolError::InvalidArgs {
                message: "Line number must be a positive integer".to_string(),
            });
        }

        Ok(())
    }

    fn execute(&mut self, args: &ToolArgs, state: &Arc<Mutex<ToolState>>) -> Result<ToolResult> {
        let line_number: usize = args
            .get_arg(0)
            .unwrap()
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid line number"))?;

        let mut state_guard = state
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;

        // Check if there's a current file
        let current_file = state_guard
            .current_file
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No file is currently open. Use 'open' first."))?;

        // Move to the specified line
        if let Some(file_state) = state_guard.get_current_file_state_mut() {
            let total_lines = file_state.total_lines();

            if line_number == 0 {
                return Ok(ToolResult::error("Line numbers start from 1".to_string()));
            }

            if line_number > total_lines {
                return Ok(ToolResult::error(format!(
                    "Line {} is beyond the end of the file (total lines: {})",
                    line_number, total_lines
                )));
            }

            file_state.goto_line(line_number);
            let display = file_state.get_window_with_line_numbers();

            state_guard.push_history(format!(
                "Moved to line {} in {}",
                line_number,
                current_file.display()
            ));

            let result_message = format!(
                "Moved to line {} in {}\n\n{}",
                line_number,
                current_file.display(),
                display.join("\n")
            );

            Ok(ToolResult::success_with_data(
                result_message,
                serde_json::json!({
                    "line_number": line_number,
                    "file": current_file,
                    "window": display
                }),
            ))
        } else {
            Err(anyhow::anyhow!("Failed to get file state"))
        }
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "line_number": {
                    "type": "integer",
                    "description": "The line number to move the window to",
                    "minimum": 1
                }
            },
            "required": ["line_number"]
        })
    }
}

/// Tool for scrolling within files
pub struct ScrollTool {
    name: String,
    up: bool, // true for scroll_up, false for scroll_down
}

impl ScrollTool {
    pub fn new(name: &str, up: bool) -> Self {
        Self {
            name: name.to_string(),
            up,
        }
    }
}

impl Tool for ScrollTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        if self.up {
            "Moves the window up by the window size"
        } else {
            "Moves the window down by the window size"
        }
    }

    fn signature(&self) -> &str {
        &self.name
    }

    fn validate_args(&self, _args: &ToolArgs) -> Result<(), ToolError> {
        Ok(()) // Scroll tools take no arguments
    }

    fn execute(&mut self, _args: &ToolArgs, state: &Arc<Mutex<ToolState>>) -> Result<ToolResult> {
        let mut state_guard = state
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;

        // Check if there's a current file
        let current_file = state_guard
            .current_file
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No file is currently open. Use 'open' first."))?;

        if let Some(file_state) = state_guard.get_current_file_state_mut() {
            let old_start = file_state.window_start;

            if self.up {
                if file_state.is_at_start() {
                    return Ok(ToolResult::success("Already at the beginning of the file"));
                }
                file_state.scroll_up();
            } else {
                if file_state.is_at_end() {
                    return Ok(ToolResult::success("Already at the end of the file"));
                }
                file_state.scroll_down();
            }

            let new_start = file_state.window_start;
            let display = file_state.get_window_with_line_numbers();

            let direction = if self.up { "up" } else { "down" };
            state_guard.push_history(format!(
                "Scrolled {} in {}",
                direction,
                current_file.display()
            ));

            let result_message = format!(
                "Scrolled {} from line {} to line {} in {}\n\n{}",
                direction,
                old_start + 1,
                new_start + 1,
                current_file.display(),
                display.join("\n")
            );

            Ok(ToolResult::success_with_data(
                result_message,
                serde_json::json!({
                    "direction": direction,
                    "old_start": old_start + 1,
                    "new_start": new_start + 1,
                    "file": current_file,
                    "window": display
                }),
            ))
        } else {
            Err(anyhow::anyhow!("Failed to get file state"))
        }
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }
}

/// Tool for creating new files
pub struct CreateTool {
    name: String,
}

impl CreateTool {
    pub fn new() -> Self {
        Self {
            name: "create".to_string(),
        }
    }
}

impl Tool for CreateTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Creates and opens a new file with the given name"
    }

    fn signature(&self) -> &str {
        "create <filename>"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        if args.is_empty() {
            return Err(ToolError::InvalidArgs {
                message: "Usage: create <filename>".to_string(),
            });
        }
        Ok(())
    }

    fn execute(&mut self, args: &ToolArgs, state: &Arc<Mutex<ToolState>>) -> Result<ToolResult> {
        let filename = args.get_arg(0).unwrap();
        let path_buf = PathBuf::from(filename);

        // Check if file already exists
        if path_buf.exists() {
            return Ok(ToolResult::error(format!(
                "File already exists: {}",
                filename
            )));
        }

        // Create the file
        fs::write(&path_buf, "").map_err(|e| anyhow::anyhow!("Failed to create file: {}", e))?;

        // Open the new file
        let content = vec![]; // Empty file

        // Update state
        {
            let mut state_guard = state
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;
            state_guard.open_file(path_buf.clone(), content.clone(), DEFAULT_WINDOW_SIZE)?;
            state_guard.push_history(format!("Created file: {}", filename));
        }

        // Create windowed file for display
        let windowed_file = WindowedFile::from_content(path_buf, content, None);
        let display = windowed_file.get_window_display();

        Ok(ToolResult::success_with_data(
            format!("Created and opened file: {}", filename),
            serde_json::json!({
                "path": filename,
                "display": display,
                "created": true
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "filename": {
                    "type": "string",
                    "description": "The name of the file to create"
                }
            },
            "required": ["filename"]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let file_path = dir.path().join(name);
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "{}", content).unwrap();
        file_path
    }

    #[test]
    fn test_windowed_file_creation() {
        let temp_dir = TempDir::new().unwrap();
        let test_content = "line1\nline2\nline3\nline4\nline5";
        let file_path = create_test_file(&temp_dir, "test.txt", test_content);

        let windowed_file = WindowedFile::new(file_path, Some(3)).unwrap();
        assert_eq!(windowed_file.content.len(), 5);
        assert_eq!(windowed_file.window_size, 3);
        assert_eq!(windowed_file.window_start, 0);
    }

    #[test]
    fn test_open_tool() {
        let temp_dir = TempDir::new().unwrap();
        let test_content = "line1\nline2\nline3";
        let file_path = create_test_file(&temp_dir, "test.txt", test_content);

        let mut tool = OpenTool::new();
        let state = Arc::new(Mutex::new(ToolState::new()));
        let args = ToolArgs::from_args(&[file_path.to_str().unwrap()]);

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);

        // Verify state was updated
        let state_guard = state.lock().unwrap();
        assert!(state_guard.current_file.is_some());
        assert_eq!(state_guard.open_files.len(), 1);
    }

    #[test]
    fn test_goto_tool() {
        let mut tool = GotoTool::new();
        let state = Arc::new(Mutex::new(ToolState::new()));

        // Test with no file open
        let args = ToolArgs::from_args(&["5"]);
        let result = tool.execute(&args, &state);
        assert!(result.is_err());

        // Open a file first
        let content: Vec<String> = (1..=10).map(|i| format!("line {}", i)).collect();
        {
            let mut state_guard = state.lock().unwrap();
            state_guard
                .open_file(PathBuf::from("test.txt"), content, DEFAULT_WINDOW_SIZE)
                .unwrap();
        }

        // Now test goto
        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_open_tool_with_configured_window() {
        // Create a temporary file to test configured window size (hermetic test)
        let temp_dir = TempDir::new().unwrap();
        let test_content = "line1\nline2\nline3\nline4";
        let file_path = create_test_file(&temp_dir, "test_agent.rs", test_content);

        let mut tool = OpenTool::new_with_open_window_size(Some(1000));
        let state = Arc::new(Mutex::new(ToolState::new()));

        let args = ToolArgs::from_args(&[file_path.to_str().unwrap()]);
        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);

        let state_guard = state.lock().unwrap();
        let file_state = state_guard
            .get_current_file_state()
            .expect("file should be open");
        assert_eq!(
            file_state.window_size, 1000,
            "Open tool should set window size to configured value"
        );
    }

    #[test]
    fn test_scroll_tools() {
        let mut up_tool = ScrollTool::new("scroll_up", true);
        let mut down_tool = ScrollTool::new("scroll_down", false);
        let state = Arc::new(Mutex::new(ToolState::new()));

        // Open a file with enough content to scroll
        let content: Vec<String> = (1..=200).map(|i| format!("line {}", i)).collect();
        {
            let mut state_guard = state.lock().unwrap();
            state_guard
                .open_file(PathBuf::from("test.txt"), content, 50)
                .unwrap();
        }

        let args = ToolArgs::from_args(&[]);

        // Test scroll down
        let result = down_tool.execute(&args, &state).unwrap();
        assert!(result.success);

        // Test scroll up
        let result = up_tool.execute(&args, &state).unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_create_tool() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let mut tool = CreateTool::new();
        let state = Arc::new(Mutex::new(ToolState::new()));
        let args = ToolArgs::from_args(&["new_file.txt"]);

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);

        // Verify file was created
        assert!(PathBuf::from("new_file.txt").exists());

        // Verify state was updated
        let state_guard = state.lock().unwrap();
        assert!(state_guard.current_file.is_some());
    }
}
