//! Utility tools for project analysis and task completion

use crate::core::{Tool, ToolArgs, ToolError, ToolResult};
use crate::state::ToolState;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;

use crate::search::ConfigurableFilter;

mod count_tokens;

pub use count_tokens::CountTokensTool;

/// Tool for task classification
pub struct ClassifyTaskTool {
    name: String,
}

impl ClassifyTaskTool {
    pub fn new() -> Self {
        Self {
            name: "classify_task".to_string(),
        }
    }
}

impl Tool for ClassifyTaskTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Classify a task request into one of the supported categories: bug_fix, feature, maintenance, or query"
    }

    fn signature(&self) -> &str {
        "classify_task <task_type>"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        if args.is_empty() {
            return Err(ToolError::InvalidArgs {
                message: "Usage: classify_task <task_type>. Valid types: bug_fix, feature, maintenance, query".to_string(),
            });
        }

        let task_type = args.get_arg(0).unwrap();
        match task_type.as_str() {
            "bug_fix" | "feature" | "maintenance" | "query" => Ok(()),
            _ => Err(ToolError::InvalidArgs {
                message: format!(
                    "Invalid task type: {}. Valid types: bug_fix, feature, maintenance, query",
                    task_type
                ),
            }),
        }
    }

    fn execute(&mut self, args: &ToolArgs, state: &Arc<Mutex<ToolState>>) -> Result<ToolResult> {
        let task_type = args.get_arg(0).unwrap();

        // Update state with classification result
        {
            let mut state_guard = state
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;
            state_guard.push_history(format!("Classified task as: {}", task_type));
        }

        Ok(ToolResult::success_with_data(
            format!("Task classified as: {}", task_type),
            serde_json::json!({
                "task_type": task_type,
                "action": "classify_task"
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task_type": {
                    "type": "string",
                    "description": "The task type classification",
                    "enum": ["bug_fix", "feature", "maintenance", "query"]
                }
            },
            "required": ["task_type"]
        })
    }
}

pub struct FilemapTool {
    name: String,
}

impl FilemapTool {
    pub fn new() -> Self {
        Self {
            name: "filemap".to_string(),
        }
    }

    /// Check if a file should be included in the filemap
    #[allow(dead_code)]
    fn should_include_file(path: &Path) -> bool {
        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy();

            // Skip hidden files and directories
            if name_str.starts_with('.') {
                return false;
            }

            // Skip common build/cache directories
            if matches!(
                name_str.as_ref(),
                "target"
                    | "node_modules"
                    | "__pycache__"
                    | "dist"
                    | "build"
                    | ".git"
                    | ".svn"
                    | ".hg"
                    | "venv"
                    | "env"
                    | ".venv"
            ) {
                return false;
            }

            // Skip binary and cache files
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if matches!(
                    ext_str.as_str(),
                    "exe"
                        | "dll"
                        | "so"
                        | "dylib"
                        | "a"
                        | "o"
                        | "pyc"
                        | "png"
                        | "jpg"
                        | "jpeg"
                        | "gif"
                        | "bmp"
                        | "ico"
                        | "mp3"
                        | "mp4"
                        | "avi"
                        | "mov"
                        | "wav"
                        | "pdf"
                        | "zip"
                        | "tar"
                        | "gz"
                        | "rar"
                        | "7z"
                ) {
                    return false;
                }
            }
        }

        true
    }

    /// Generate a tree-like directory structure
    fn generate_tree(path: &Path, max_depth: usize) -> Result<String> {
        let mut result = String::new();

        if path.is_file() {
            // If it's a file, just show its content summary
            return Self::show_file_content(path);
        }

        result.push_str(&format!("📁 {}\n", path.display()));

        // Create a single ConfigurableFilter instance to avoid re-reading config per entry
        let filter = ConfigurableFilter::new(None);

        // Use `filter_entry` to prevent walking into ignored directories (e.g. `.git`, `target`).
        // We still `filter_map` the iterator to ignore IO errors and then skip the root path itself.
        let mut entries: Vec<_> = WalkDir::new(path)
            .max_depth(max_depth)
            .into_iter()
            .filter_entry(|e| filter.should_include_path(e.path()))
            .filter_map(|e| e.ok())
            .filter(|e| e.path() != path)
            .collect();

        // Sort entries by full path so items that belong to the same directory
        // are listed next to each other. This ensures files appear directly
        // under their parent directory instead of printing all directories
        // first which separates files from their directory context.
        entries.sort_by(|a, b| a.path().cmp(b.path()));

        let mut file_count = 0;
        let mut dir_count = 0;

        for entry in entries.iter().take(100) {
            // Limit to first 100 entries
            let depth = entry.depth();
            let indent = "  ".repeat(depth);
            let name = entry.file_name().to_string_lossy();

            if entry.file_type().is_dir() {
                result.push_str(&format!("{}📁 {}/\n", indent, name));
                dir_count += 1;
            } else {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                result.push_str(&format!("{}📄 {} ({} bytes)\n", indent, name, size));
                file_count += 1;
            }
        }

        if entries.len() > 100 {
            result.push_str(&format!("... and {} more items\n", entries.len() - 100));
        }

        result.push_str(&format!(
            "\nSummary: {} directories, {} files\n",
            dir_count, file_count
        ));

        Ok(result)
    }

    /// Show abbreviated file content (similar to SWE-agent's filemap for Python)
    fn show_file_content(path: &Path) -> Result<String> {
        let content =
            fs::read_to_string(path).map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?;

        let lines: Vec<&str> = content.lines().collect();
        let mut result = String::new();

        result.push_str(&format!("📄 {} ({} lines)\n", path.display(), lines.len()));

        // Check if it's a Python file for special handling
        if let Some(ext) = path.extension() {
            if ext == "py" {
                return Self::show_python_file_content(path, &lines);
            }
        }

        // For non-Python files, show with elision of large blocks
        let mut current_line = 0;
        while current_line < lines.len() {
            let line = lines[current_line];

            // Check if this is the start of a large block (function, class, etc.)
            if Self::is_block_start(line) {
                // Find the end of this block
                let block_end = Self::find_block_end(&lines, current_line);

                // If block is more than 5 lines, elide it
                if block_end - current_line > 5 {
                    result.push_str(&format!("{:4} | {}\n", current_line + 1, line));
                    result.push_str(&format!(
                        "     | ... eliding lines {}-{} ...\n",
                        current_line + 2,
                        block_end
                    ));
                    current_line = block_end;
                } else {
                    // Show the small block normally
                    for i in current_line..=block_end {
                        if i < lines.len() {
                            result.push_str(&format!("{:4} | {}\n", i + 1, lines[i]));
                        }
                    }
                    current_line = block_end + 1;
                }
            } else {
                result.push_str(&format!("{:4} | {}\n", current_line + 1, line));
                current_line += 1;
            }

            // Limit total output
            if result.len() > 50000 {
                result.push_str("... (output truncated) ...\n");
                break;
            }
        }

        Ok(result)
    }

    /// Show Python file content with function/class elision
    fn show_python_file_content(path: &Path, lines: &[&str]) -> Result<String> {
        let mut result = String::new();
        result.push_str(&format!("📄 {} ({} lines)\n", path.display(), lines.len()));

        let mut current_line = 0;
        while current_line < lines.len() {
            let line = lines[current_line];
            let trimmed = line.trim_start();

            // Check for Python function or class definitions
            if trimmed.starts_with("def ") || trimmed.starts_with("class ") {
                let indent_level = line.len() - line.trim_start().len();

                // Find the end of this function/class
                let mut end_line = current_line + 1;
                let mut found_body = false;

                while end_line < lines.len() {
                    let next_line = lines[end_line];
                    let next_trimmed = next_line.trim();

                    // Skip empty lines and comments
                    if next_trimmed.is_empty() || next_trimmed.starts_with('#') {
                        end_line += 1;
                        continue;
                    }

                    let next_indent = next_line.len() - next_line.trim_start().len();

                    // If we find content at the same or lower indentation level, we've reached the end
                    if next_indent <= indent_level && found_body {
                        break;
                    }

                    if next_indent > indent_level {
                        found_body = true;
                    }

                    end_line += 1;
                }

                // Show the definition line
                result.push_str(&format!("{:4} | {}\n", current_line + 1, line));

                // If the function/class body is more than 5 lines, elide it
                if end_line - current_line > 5 {
                    result.push_str(&format!(
                        "     | ... eliding lines {}-{} ...\n",
                        current_line + 2,
                        end_line
                    ));
                } else {
                    // Show the small function/class normally
                    for i in (current_line + 1)..end_line {
                        if i < lines.len() {
                            result.push_str(&format!("{:4} | {}\n", i + 1, lines[i]));
                        }
                    }
                }

                current_line = end_line;
            } else {
                result.push_str(&format!("{:4} | {}\n", current_line + 1, line));
                current_line += 1;
            }

            // Limit total output
            if result.len() > 50000 {
                result.push_str("... (output truncated) ...\n");
                break;
            }
        }

        Ok(result)
    }

    /// Check if a line starts a block that should be elided
    fn is_block_start(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("def ") || 
        trimmed.starts_with("class ") ||
        trimmed.starts_with("fn ") ||  // Rust functions
        trimmed.starts_with("function ") ||  // JavaScript functions
        trimmed.starts_with("impl ") ||  // Rust impl blocks
        (trimmed.starts_with("if ") && line.trim_end().ends_with(":")) ||  // Python if blocks
        (trimmed.starts_with("for ") && line.trim_end().ends_with(":")) ||  // Python loops
        (trimmed.starts_with("while ") && line.trim_end().ends_with(":")) // Python while loops
    }

    /// Find the end of a code block
    fn find_block_end(lines: &[&str], start: usize) -> usize {
        if start >= lines.len() {
            return start;
        }

        let start_line = lines[start];
        let indent_level = start_line.len() - start_line.trim_start().len();

        let mut end_line = start + 1;
        let mut found_body = false;

        while end_line < lines.len() {
            let line = lines[end_line];
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
                end_line += 1;
                continue;
            }

            let line_indent = line.len() - line.trim_start().len();

            // If we find content at the same or lower indentation level after finding body, we've reached the end
            if line_indent <= indent_level && found_body {
                break;
            }

            if line_indent > indent_level {
                found_body = true;
            }

            end_line += 1;
        }

        end_line.saturating_sub(1)
    }
}

impl Tool for FilemapTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Generate a file structure map or show file contents with abbreviated view"
    }

    fn signature(&self) -> &str {
        "filemap <file_path>"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        if args.is_empty() {
            return Err(ToolError::InvalidArgs {
                message: "Usage: filemap <file_path>".to_string(),
            });
        }
        Ok(())
    }

    fn execute(&mut self, args: &ToolArgs, state: &Arc<Mutex<ToolState>>) -> Result<ToolResult> {
        let file_path = args.get_arg(0).unwrap();
        let path_buf = PathBuf::from(file_path);

        // Check if path exists
        if !path_buf.exists() {
            return Ok(ToolResult::error(format!("Path not found: {}", file_path)));
        }

        // Generate the filemap
        let content = Self::generate_tree(&path_buf, 3)?; // Max depth of 3

        // Update state
        {
            let mut state_guard = state
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;
            state_guard.push_history(format!("Generated filemap for: {}", file_path));
        }

        Ok(ToolResult::success_with_data(
            content,
            serde_json::json!({
                "path": file_path,
                "type": if path_buf.is_file() { "file" } else { "directory" },
                "action": "filemap"
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "The path to the file or directory to map"
                }
            },
            "required": ["file_path"]
        })
    }
}

pub struct SubmitTool {
    name: String,
}

impl SubmitTool {
    pub fn new() -> Self {
        Self {
            name: "submit".to_string(),
        }
    }
}

impl Tool for SubmitTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Submit your completed task or solution"
    }

    fn signature(&self) -> &str {
        "submit"
    }

    fn validate_args(&self, _args: &ToolArgs) -> Result<(), ToolError> {
        Ok(()) // Submit takes no arguments
    }

    fn execute(&mut self, _args: &ToolArgs, state: &Arc<Mutex<ToolState>>) -> Result<ToolResult> {
        // Update state
        {
            let mut state_guard = state
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;
            state_guard.push_history("Task submitted".to_string());
        }

        Ok(ToolResult::success_with_data(
            "Task has been submitted successfully".to_string(),
            serde_json::json!({
                "action": "submit",
                "status": "completed"
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
