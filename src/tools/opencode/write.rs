//! Write tool implementation compatible with OpenCode
//!
//! Writes content to files in the local filesystem.

use crate::core::{Tool, ToolArgs, ToolError, ToolResult};
use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Write tool parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WriteParams {
    /// The content to write to the file
    pub content: String,
    /// The absolute path to the file to write
    pub file_path: String,
}

/// Write tool for writing files
pub struct WriteTool {
    name: String,
}

impl WriteTool {
    pub fn new() -> Self {
        Self {
            name: "write".to_string(),
        }
    }
}

impl Default for WriteTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for WriteTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Writes a file to the local filesystem"
    }

    fn signature(&self) -> &str {
        "write --file-path <path> --content <content>"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        if args.get_named_arg("file_path").is_none() && args.args.is_empty() {
            return Err(ToolError::InvalidArgs {
                message: "write tool requires a 'file_path' argument".to_string(),
            });
        }
        Ok(())
    }

    fn execute(
        &mut self,
        args: &ToolArgs,
        state: &Arc<Mutex<crate::state::ToolState>>,
    ) -> Result<ToolResult> {
        let params = parse_write_args(args)?;

        // Get working directory from ToolState at execution time
        let working_dir = state
            .lock()
            .map(|s| s.working_directory.clone())
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());

        let filepath = if Path::new(&params.file_path).is_absolute() {
            PathBuf::from(&params.file_path)
        } else {
            working_dir.join(&params.file_path)
        };

        let exists = filepath.exists();

        // Check if file was read first (for existing files)
        // Note: In a real implementation, this would check across the session
        // For now, we'll allow writes as this is a library

        // Create parent directories if needed
        if let Some(parent) = filepath.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        // Write the file
        fs::write(&filepath, &params.content)?;

        let relative_path = filepath.display().to_string();
        let message = if exists {
            format!("Overwrote file: {}", relative_path)
        } else {
            format!("Created file: {}", relative_path)
        };

        Ok(ToolResult::success_with_data(
            message,
            serde_json::json!({
                "file_path": relative_path,
                "exists": exists,
                "content_length": params.content.len(),
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        let schema = schemars::schema_for!(WriteParams);
        serde_json::to_value(schema).unwrap_or_default()
    }
}

fn parse_write_args(args: &ToolArgs) -> Result<WriteParams> {
    let file_path = args
        .get_named_arg("file_path")
        .cloned()
        .or_else(|| args.get_named_arg("filePath").cloned())
        .or_else(|| args.args.first().cloned())
        .ok_or_else(|| anyhow::anyhow!("file_path is required"))?;

    let content = args
        .get_named_arg("content")
        .cloned()
        .or_else(|| args.args.get(1).cloned())
        .unwrap_or_default();

    Ok(WriteParams { content, file_path })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_tool_creation() {
        let tool = WriteTool::new();
        assert_eq!(tool.name(), "write");
    }

    #[test]
    fn test_write_tool_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let mut tool = WriteTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let args = ToolArgs::with_named_args(
            vec![file_path.to_str().unwrap().to_string()],
            vec![("content".to_string(), "Hello, World!".to_string())]
                .into_iter()
                .collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);
        assert!(result.message.contains("Created file"));

        // Verify file was written
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[test]
    fn test_write_tool_overwrite() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create initial file
        fs::write(&file_path, "Original content").unwrap();

        let mut tool = WriteTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let args = ToolArgs::with_named_args(
            vec![file_path.to_str().unwrap().to_string()],
            vec![("content".to_string(), "New content".to_string())]
                .into_iter()
                .collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);
        assert!(result.message.contains("Overwrote file"));

        // Verify file was overwritten
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "New content");
    }

    #[test]
    fn test_write_tool_creates_directories() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("subdir/nested/test.txt");

        let mut tool = WriteTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let args = ToolArgs::with_named_args(
            vec![file_path.to_str().unwrap().to_string()],
            vec![("content".to_string(), "Nested content".to_string())]
                .into_iter()
                .collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);

        // Verify file was written
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Nested content");
    }

    #[test]
    fn test_write_tool_validation() {
        let tool = WriteTool::new();
        let args = ToolArgs::from_args(&[]);

        let result = tool.validate_args(&args);
        assert!(result.is_err());
    }
}
