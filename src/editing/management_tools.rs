//! File management tools for file operations
//!
//! This module provides tools for file and directory management operations
//! like delete, move, and copy with simple interfaces.

use crate::core::{Tool, ToolArgs, ToolError, ToolResult};
use crate::state::ToolState;
use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Tool for deleting files or directories
pub struct DeletePathTool {
    name: String,
}

impl DeletePathTool {
    pub fn new() -> Self {
        Self {
            name: "delete_path".to_string(),
        }
    }

    /// Parse parameters from ToolArgs
    fn parse_params(&self, args: &ToolArgs) -> Result<serde_json::Value, ToolError> {
        // Try to parse as JSON first
        if let Some(json_str) = args.get_named_arg("json") {
            return serde_json::from_str(json_str).map_err(|e| ToolError::Json(e));
        }

        // Check if we have structured named arguments
        if !args.named_args.is_empty() {
            return Ok(serde_json::to_value(&args.named_args).map_err(|e| ToolError::Json(e))?);
        }

        // Fall back to positional arguments for backward compatibility
        if args.len() >= 1 {
            let mut params = serde_json::Map::new();
            params.insert(
                "path".to_string(),
                serde_json::Value::String(args.get_arg(0).unwrap().clone()),
            );

            if args.len() >= 2 {
                if let Ok(recursive) = args.get_arg(1).unwrap().parse::<bool>() {
                    params.insert("recursive".to_string(), serde_json::Value::Bool(recursive));
                }
            }

            return Ok(serde_json::Value::Object(params));
        }

        Err(ToolError::InvalidArgs {
            message: "Insufficient parameters".to_string(),
        })
    }
}

impl Tool for DeletePathTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Delete a file or directory"
    }

    fn signature(&self) -> &str {
        "delete_path(path: str, recursive?: bool)"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let params = self.parse_params(args)?;

        let obj = params.as_object().ok_or_else(|| ToolError::InvalidArgs {
            message: "Parameters must be an object".to_string(),
        })?;

        if !obj.contains_key("path") {
            return Err(ToolError::InvalidArgs {
                message: "Missing required parameter: path".to_string(),
            });
        }

        Ok(())
    }

    fn execute(&mut self, args: &ToolArgs, state: &Arc<Mutex<ToolState>>) -> Result<ToolResult> {
        let params = self.parse_params(args)?;
        let obj = params
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Invalid parameters"))?;

        let path_str = obj
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid path parameter"))?;

        let recursive = obj
            .get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let path = PathBuf::from(path_str);

        // Check if path exists
        if !path.exists() {
            return Ok(ToolResult::error(format!(
                "Path not found: {}",
                path.display()
            )));
        }

        let is_dir = path.is_dir();
        let is_file = path.is_file();

        // Safety check for important directories
        let path_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Use centralized filter configuration for protected directories instead of a hardcoded list.
        // Always protect "src" by default, and also protect any directory listed in the search filtering exclude_dirs.
        let filter = crate::search::filtering::ConfigurableFilter::new(None);
        let mut protected = false;

        if path_name == "src" {
            protected = true;
        }

        if !protected {
            if let Some(ex_dirs) = &filter.config.exclude_dirs {
                if ex_dirs.iter().any(|d| d == path_name) {
                    protected = true;
                }
            }
        }

        if is_dir && protected {
            return Ok(ToolResult::error(format!(
                "Safety check: Refusing to delete important directory '{}'. Use recursive=true explicitly if needed.",
                path_name
            )));
        }

        // Perform the deletion
        if is_file {
            fs::remove_file(&path).map_err(|e| anyhow::anyhow!("Failed to delete file: {}", e))?;
        } else if is_dir {
            if recursive {
                fs::remove_dir_all(&path)
                    .map_err(|e| anyhow::anyhow!("Failed to delete directory: {}", e))?;
            } else {
                // Try to remove empty directory
                fs::remove_dir(&path)
                    .map_err(|e| anyhow::anyhow!("Failed to delete directory (not empty?): {}. Use recursive=true for non-empty directories.", e))?;
            }
        }

        // Update state
        {
            let mut state_guard = state
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;
            // If we deleted the currently open file, clear it from state
            if state_guard.current_file.as_ref() == Some(&path) {
                state_guard.current_file = None;
            }
            state_guard.push_history(format!(
                "Deleted {}: {}",
                if is_dir { "directory" } else { "file" },
                path.display()
            ));
        }

        Ok(ToolResult::success_with_data(
            format!(
                "Successfully deleted {}: {}",
                if is_dir { "directory" } else { "file" },
                path.display()
            ),
            serde_json::json!({
                "path": path.to_string_lossy(),
                "type": if is_dir { "directory" } else { "file" },
                "recursive": recursive,
                "deleted": true
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Full path to the file or directory to delete"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "Whether to delete directories and their contents recursively",
                    "default": false
                }
            },
            "required": ["path"]
        })
    }
}

/// Tool for moving/renaming files or directories
pub struct MovePathTool {
    name: String,
}

impl MovePathTool {
    pub fn new() -> Self {
        Self {
            name: "move_path".to_string(),
        }
    }

    /// Parse parameters from ToolArgs
    fn parse_params(&self, args: &ToolArgs) -> Result<serde_json::Value, ToolError> {
        // Try to parse as JSON first
        if let Some(json_str) = args.get_named_arg("json") {
            return serde_json::from_str(json_str).map_err(|e| ToolError::Json(e));
        }

        // Check if we have structured named arguments
        if !args.named_args.is_empty() {
            return Ok(serde_json::to_value(&args.named_args).map_err(|e| ToolError::Json(e))?);
        }

        // Fall back to positional arguments for backward compatibility
        if args.len() >= 2 {
            let mut params = serde_json::Map::new();
            params.insert(
                "source".to_string(),
                serde_json::Value::String(args.get_arg(0).unwrap().clone()),
            );
            params.insert(
                "destination".to_string(),
                serde_json::Value::String(args.get_arg(1).unwrap().clone()),
            );
            return Ok(serde_json::Value::Object(params));
        }

        Err(ToolError::InvalidArgs {
            message: "Insufficient parameters".to_string(),
        })
    }
}

impl Tool for MovePathTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Move or rename a file or directory"
    }

    fn signature(&self) -> &str {
        "move_path(source: str, destination: str)"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let params = self.parse_params(args)?;

        let obj = params.as_object().ok_or_else(|| ToolError::InvalidArgs {
            message: "Parameters must be an object".to_string(),
        })?;

        if !obj.contains_key("source") {
            return Err(ToolError::InvalidArgs {
                message: "Missing required parameter: source".to_string(),
            });
        }

        if !obj.contains_key("destination") {
            return Err(ToolError::InvalidArgs {
                message: "Missing required parameter: destination".to_string(),
            });
        }

        Ok(())
    }

    fn execute(&mut self, args: &ToolArgs, state: &Arc<Mutex<ToolState>>) -> Result<ToolResult> {
        let params = self.parse_params(args)?;
        let obj = params
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Invalid parameters"))?;

        let source_str = obj
            .get("source")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid source parameter"))?;

        let dest_str = obj
            .get("destination")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid destination parameter"))?;

        let source = PathBuf::from(source_str);
        let destination = PathBuf::from(dest_str);

        // Check if source exists
        if !source.exists() {
            return Ok(ToolResult::error(format!(
                "Source path not found: {}",
                source.display()
            )));
        }

        // Check if destination already exists
        if destination.exists() {
            return Ok(ToolResult::error(format!(
                "Destination already exists: {}",
                destination.display()
            )));
        }

        // Create parent directories for destination if needed
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                anyhow::anyhow!("Failed to create destination parent directories: {}", e)
            })?;
        }

        let is_dir = source.is_dir();

        // Perform the move
        fs::rename(&source, &destination)
            .map_err(|e| anyhow::anyhow!("Failed to move path: {}", e))?;

        // Update state
        {
            let mut state_guard = state
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;
            // If we moved the currently open file, update the path in state
            if state_guard.current_file.as_ref() == Some(&source) {
                state_guard.current_file = Some(destination.clone());
            }
            state_guard.push_history(format!(
                "Moved {} from {} to {}",
                if is_dir { "directory" } else { "file" },
                source.display(),
                destination.display()
            ));
        }

        Ok(ToolResult::success_with_data(
            format!(
                "Successfully moved {} from {} to {}",
                if is_dir { "directory" } else { "file" },
                source.display(),
                destination.display()
            ),
            serde_json::json!({
                "source": source.to_string_lossy(),
                "destination": destination.to_string_lossy(),
                "type": if is_dir { "directory" } else { "file" },
                "moved": true
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": "Full path to the source file or directory"
                },
                "destination": {
                    "type": "string",
                    "description": "Full path to the destination"
                }
            },
            "required": ["source", "destination"]
        })
    }
}

/// Tool for copying files or directories
pub struct CopyPathTool {
    name: String,
}

impl CopyPathTool {
    pub fn new() -> Self {
        Self {
            name: "copy_path".to_string(),
        }
    }

    /// Parse parameters from ToolArgs
    fn parse_params(&self, args: &ToolArgs) -> Result<serde_json::Value, ToolError> {
        // Try to parse as JSON first
        if let Some(json_str) = args.get_named_arg("json") {
            return serde_json::from_str(json_str).map_err(|e| ToolError::Json(e));
        }

        // Check if we have structured named arguments
        if !args.named_args.is_empty() {
            return Ok(serde_json::to_value(&args.named_args).map_err(|e| ToolError::Json(e))?);
        }

        // Fall back to positional arguments for backward compatibility
        if args.len() >= 2 {
            let mut params = serde_json::Map::new();
            params.insert(
                "source".to_string(),
                serde_json::Value::String(args.get_arg(0).unwrap().clone()),
            );
            params.insert(
                "destination".to_string(),
                serde_json::Value::String(args.get_arg(1).unwrap().clone()),
            );

            if args.len() >= 3 {
                if let Ok(recursive) = args.get_arg(2).unwrap().parse::<bool>() {
                    params.insert("recursive".to_string(), serde_json::Value::Bool(recursive));
                }
            }

            return Ok(serde_json::Value::Object(params));
        }

        Err(ToolError::InvalidArgs {
            message: "Insufficient parameters".to_string(),
        })
    }
}

impl Tool for CopyPathTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Copy a file or directory"
    }

    fn signature(&self) -> &str {
        "copy_path(source: str, destination: str, recursive?: bool)"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let params = self.parse_params(args)?;

        let obj = params.as_object().ok_or_else(|| ToolError::InvalidArgs {
            message: "Parameters must be an object".to_string(),
        })?;

        if !obj.contains_key("source") {
            return Err(ToolError::InvalidArgs {
                message: "Missing required parameter: source".to_string(),
            });
        }

        if !obj.contains_key("destination") {
            return Err(ToolError::InvalidArgs {
                message: "Missing required parameter: destination".to_string(),
            });
        }

        Ok(())
    }

    fn execute(&mut self, args: &ToolArgs, state: &Arc<Mutex<ToolState>>) -> Result<ToolResult> {
        let params = self.parse_params(args)?;
        let obj = params
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Invalid parameters"))?;

        let source_str = obj
            .get("source")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid source parameter"))?;

        let dest_str = obj
            .get("destination")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid destination parameter"))?;

        let recursive = obj
            .get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(true); // Default to true for directories

        let source = PathBuf::from(source_str);
        let destination = PathBuf::from(dest_str);

        // Check if source exists
        if !source.exists() {
            return Ok(ToolResult::error(format!(
                "Source path not found: {}",
                source.display()
            )));
        }

        // Check if destination already exists
        if destination.exists() {
            return Ok(ToolResult::error(format!(
                "Destination already exists: {}",
                destination.display()
            )));
        }

        let is_dir = source.is_dir();
        let is_file = source.is_file();

        // Create parent directories for destination if needed
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                anyhow::anyhow!("Failed to create destination parent directories: {}", e)
            })?;
        }

        if is_file {
            // Copy file
            fs::copy(&source, &destination)
                .map_err(|e| anyhow::anyhow!("Failed to copy file: {}", e))?;
        } else if is_dir {
            if !recursive {
                return Ok(ToolResult::error(
                    "Cannot copy directory without recursive=true".to_string(),
                ));
            }

            // Copy directory recursively
            self.copy_dir_recursive(&source, &destination)
                .map_err(|e| anyhow::anyhow!("Failed to copy directory: {}", e))?;
        }

        // Update state
        {
            let mut state_guard = state
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;
            state_guard.push_history(format!(
                "Copied {} from {} to {}",
                if is_dir { "directory" } else { "file" },
                source.display(),
                destination.display()
            ));
        }

        Ok(ToolResult::success_with_data(
            format!(
                "Successfully copied {} from {} to {}",
                if is_dir { "directory" } else { "file" },
                source.display(),
                destination.display()
            ),
            serde_json::json!({
                "source": source.to_string_lossy(),
                "destination": destination.to_string_lossy(),
                "type": if is_dir { "directory" } else { "file" },
                "recursive": recursive,
                "copied": true
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": "Full path to the source file or directory"
                },
                "destination": {
                    "type": "string",
                    "description": "Full path to the destination"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "Whether to copy directories recursively",
                    "default": true
                }
            },
            "required": ["source", "destination"]
        })
    }
}

impl CopyPathTool {
    /// Recursively copy directory contents
    fn copy_dir_recursive(&self, source: &PathBuf, destination: &PathBuf) -> Result<()> {
        // Create the destination directory
        fs::create_dir_all(destination)?;

        // Copy all entries
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let entry_path = entry.path();
            let entry_name = entry.file_name();
            let dest_path = destination.join(entry_name);

            if entry_path.is_dir() {
                // Recursively copy subdirectory
                self.copy_dir_recursive(&entry_path, &dest_path)?;
            } else {
                // Copy file
                fs::copy(&entry_path, &dest_path)?;
            }
        }

        Ok(())
    }
}

/// Tool for creating directories
pub struct CreateDirectoryTool {
    name: String,
}

impl CreateDirectoryTool {
    pub fn new() -> Self {
        Self {
            name: "create_directory".to_string(),
        }
    }

    /// Parse parameters from ToolArgs
    fn parse_params(&self, args: &ToolArgs) -> Result<serde_json::Value, ToolError> {
        // Try to parse as JSON first
        if let Some(json_str) = args.get_named_arg("json") {
            return serde_json::from_str(json_str).map_err(|e| ToolError::Json(e));
        }

        // Check if we have structured named arguments
        if !args.named_args.is_empty() {
            return Ok(serde_json::to_value(&args.named_args).map_err(|e| ToolError::Json(e))?);
        }

        // Fall back to positional arguments for backward compatibility
        if args.len() >= 1 {
            let mut params = serde_json::Map::new();
            params.insert(
                "path".to_string(),
                serde_json::Value::String(args.get_arg(0).unwrap().clone()),
            );
            return Ok(serde_json::Value::Object(params));
        }

        Err(ToolError::InvalidArgs {
            message: "Insufficient parameters".to_string(),
        })
    }
}

impl Tool for CreateDirectoryTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Create a new directory (and parent directories if needed)"
    }

    fn signature(&self) -> &str {
        "create_directory(path: str)"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let params = self.parse_params(args)?;

        let obj = params.as_object().ok_or_else(|| ToolError::InvalidArgs {
            message: "Parameters must be an object".to_string(),
        })?;

        if !obj.contains_key("path") {
            return Err(ToolError::InvalidArgs {
                message: "Missing required parameter: path".to_string(),
            });
        }

        Ok(())
    }

    fn execute(&mut self, args: &ToolArgs, state: &Arc<Mutex<ToolState>>) -> Result<ToolResult> {
        let params = self.parse_params(args)?;
        let obj = params
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Invalid parameters"))?;

        let path_str = obj
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid path parameter"))?;

        let path = PathBuf::from(path_str);

        // Check if directory already exists
        if path.exists() {
            return Ok(ToolResult::error(format!(
                "Directory already exists: {}",
                path.display()
            )));
        }

        // Create the directory (and parents)
        fs::create_dir_all(&path)
            .map_err(|e| anyhow::anyhow!("Failed to create directory: {}", e))?;

        // Update state
        {
            let mut state_guard = state
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;
            state_guard.push_history(format!("Created directory: {}", path.display()));
        }

        Ok(ToolResult::success_with_data(
            format!("Successfully created directory: {}", path.display()),
            serde_json::json!({
                "path": path.to_string_lossy(),
                "type": "directory",
                "created": true
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Full path to the directory to create"
                }
            },
            "required": ["path"]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_delete_path_tool_file() {
        let temp_dir = TempDir::new().unwrap();

        // Create test file with absolute path
        let test_file = temp_dir.path().join("test_delete.txt");
        fs::write(&test_file, "test content").unwrap();

        let mut tool = DeletePathTool::new();
        let state = Arc::new(Mutex::new(ToolState::new()));

        let args = ToolArgs::with_named_args(
            vec![],
            vec![("path".to_string(), test_file.to_string_lossy().to_string())]
                .into_iter()
                .collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);

        // Verify file was deleted
        assert!(!test_file.exists());
    }

    #[test]
    fn test_move_path_tool() {
        let temp_dir = TempDir::new().unwrap();

        // Create test file with absolute path
        let source = temp_dir.path().join("source.txt");
        fs::write(&source, "test content").unwrap();
        let destination = temp_dir.path().join("destination.txt");

        let mut tool = MovePathTool::new();
        let state = Arc::new(Mutex::new(ToolState::new()));

        let args = ToolArgs::with_named_args(
            vec![],
            vec![
                ("source".to_string(), source.to_string_lossy().to_string()),
                (
                    "destination".to_string(),
                    destination.to_string_lossy().to_string(),
                ),
            ]
            .into_iter()
            .collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);

        // Verify file was moved
        assert!(!source.exists());
        assert!(destination.exists());

        let content = fs::read_to_string(&destination).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_copy_path_tool() {
        let temp_dir = TempDir::new().unwrap();

        // Create test file with absolute path
        let source = temp_dir.path().join("source.txt");
        fs::write(&source, "test content").unwrap();
        let destination = temp_dir.path().join("copy.txt");

        let mut tool = CopyPathTool::new();
        let state = Arc::new(Mutex::new(ToolState::new()));

        let args = ToolArgs::with_named_args(
            vec![],
            vec![
                ("source".to_string(), source.to_string_lossy().to_string()),
                (
                    "destination".to_string(),
                    destination.to_string_lossy().to_string(),
                ),
            ]
            .into_iter()
            .collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);

        // Verify file was copied
        assert!(source.exists());
        assert!(destination.exists());

        let content = fs::read_to_string(&destination).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_create_directory_tool() {
        let temp_dir = TempDir::new().unwrap();

        let mut tool = CreateDirectoryTool::new();
        let state = Arc::new(Mutex::new(ToolState::new()));

        // Use absolute path for the directory to be created
        let dir_path = temp_dir.path().join("new_dir/sub_dir");

        let args = ToolArgs::with_named_args(
            vec![],
            vec![("path".to_string(), dir_path.to_string_lossy().to_string())]
                .into_iter()
                .collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);

        // Verify directory was created
        assert!(dir_path.exists());
        assert!(dir_path.is_dir());
    }
}
