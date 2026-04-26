//! MultiEdit tool implementation compatible with OpenCode
//!
//! Performs multiple string replacements in a single file in one operation.

use crate::core::{Tool, ToolArgs, ToolError, ToolResult};
use crate::tools::opencode::edit::replace_in_content;
use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Single edit operation
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EditOperation {
    /// The text to replace
    #[serde(alias = "oldString")]
    pub old_string: String,
    /// The text to replace it with (must be different from oldString)
    #[serde(alias = "newString")]
    pub new_string: String,
    /// Replace all occurrences of oldString (default false)
    #[serde(alias = "replaceAll")]
    pub replace_all: Option<bool>,
}

/// MultiEdit tool parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MultiEditParams {
    /// The absolute path to the file to modify
    pub file_path: String,
    /// Array of edit operations to perform sequentially on the file
    pub edits: Vec<EditOperation>,
}

/// MultiEdit tool for performing multiple string replacements in one operation
pub struct MultiEditTool {
    name: String,
}

impl MultiEditTool {
    pub fn new() -> Self {
        Self {
            name: "multiedit".to_string(),
        }
    }
}

impl Default for MultiEditTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for MultiEditTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Performs multiple string replacements in a single file in one operation"
    }

    fn signature(&self) -> &str {
        "multiedit --file-path <path> --edits <json>"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        if args.get_named_arg("file_path").is_none()
            && args.get_named_arg("filePath").is_none()
            && args.args.is_empty()
        {
            return Err(ToolError::InvalidArgs {
                message: "multiedit tool requires a 'file_path' argument".to_string(),
            });
        }
        if args.get_named_arg("edits").is_none() && args.args.len() < 2 {
            return Err(ToolError::InvalidArgs {
                message: "multiedit tool requires an 'edits' argument".to_string(),
            });
        }
        Ok(())
    }

    fn execute(
        &mut self,
        args: &ToolArgs,
        state: &Arc<Mutex<crate::state::ToolState>>,
    ) -> Result<ToolResult> {
        let params = parse_multiedit_args(args)?;

        if params.edits.is_empty() {
            return Err(anyhow::anyhow!("edits array cannot be empty").into());
        }

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

        let is_new_file = !filepath.exists();

        // Read current content (empty if new file)
        let mut content = if is_new_file {
            if let Some(parent) = filepath.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            String::new()
        } else {
            if filepath.is_dir() {
                return Err(anyhow::anyhow!(
                    "Path is a directory, not a file: {}",
                    filepath.display()
                ));
            }
            fs::read_to_string(&filepath)?
        };

        // Track all edits for the diff
        let _original_content = content.clone();

        // Apply all edits sequentially
        for (i, edit) in params.edits.iter().enumerate() {
            if edit.old_string == edit.new_string {
                return Err(anyhow::anyhow!(
                    "Edit {}: oldString and newString must be different",
                    i + 1
                )
                .into());
            }

            content = if edit.old_string.is_empty() {
                // Creating new content (empty oldString)
                edit.new_string.clone()
            } else {
                replace_in_content(
                    &content,
                    &edit.old_string,
                    &edit.new_string,
                    edit.replace_all.unwrap_or(false),
                )
                .map_err(|e| anyhow::anyhow!("Edit {}: {}", i + 1, e))?
            };
        }

        // Write the final content
        fs::write(&filepath, &content)?;

        let message = format!(
            "Applied {} edits to: {}",
            params.edits.len(),
            filepath.display()
        );

        Ok(ToolResult::success_with_data(
            message,
            serde_json::json!({
                "file_path": filepath.display().to_string(),
                "edits_count": params.edits.len(),
                "is_new_file": is_new_file,
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        let schema = schemars::schema_for!(MultiEditParams);
        serde_json::to_value(schema).unwrap_or_default()
    }
}

fn parse_multiedit_args(args: &ToolArgs) -> Result<MultiEditParams> {
    let file_path = args
        .get_named_arg("file_path")
        .cloned()
        .or_else(|| args.get_named_arg("filePath").cloned())
        .or_else(|| args.args.first().cloned())
        .ok_or_else(|| anyhow::anyhow!("file_path is required"))?;

    let edits_json = args
        .get_named_arg("edits")
        .cloned()
        .or_else(|| args.args.get(1).cloned())
        .ok_or_else(|| anyhow::anyhow!("edits is required"))?;

    let edits: Vec<EditOperation> = serde_json::from_str(&edits_json)
        .map_err(|e| anyhow::anyhow!("Failed to parse edits JSON: {}", e))?;

    Ok(MultiEditParams { file_path, edits })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_multiedit_tool_creation() {
        let tool = MultiEditTool::new();
        assert_eq!(tool.name(), "multiedit");
    }

    #[test]
    fn test_multiedit_tool_multiple_edits() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Hello, World!").unwrap();
        writeln!(temp_file, "Foo Bar").unwrap();

        let mut tool = MultiEditTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let edits = serde_json::json!([
            {"old_string": "Hello", "new_string": "Hi"},
            {"old_string": "Foo", "new_string": "Baz"}
        ])
        .to_string();

        let args = ToolArgs::with_named_args(
            vec![temp_file.path().to_str().unwrap().to_string()],
            vec![("edits".to_string(), edits)].into_iter().collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);

        let content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(content.contains("Hi"));
        assert!(content.contains("Baz"));
        assert!(!content.contains("Hello"));
        assert!(!content.contains("Foo"));
    }

    #[test]
    fn test_multiedit_tool_create_new_file() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_path = temp_dir.path().join("new_file.txt");

        let mut tool = MultiEditTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let edits = serde_json::json!([
            {"old_string": "", "new_string": "New file content"}
        ])
        .to_string();

        let args = ToolArgs::with_named_args(
            vec![file_path.to_str().unwrap().to_string()],
            vec![("edits".to_string(), edits)].into_iter().collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "New file content");
    }

    #[test]
    fn test_multiedit_tool_validation() {
        let tool = MultiEditTool::new();
        let args = ToolArgs::from_args(&[]);

        let result = tool.validate_args(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_multiedit_tool_same_string_error() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Hello, World!").unwrap();

        let mut tool = MultiEditTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let edits = serde_json::json!([
            {"old_string": "Hello", "new_string": "Hello"}
        ])
        .to_string();

        let args = ToolArgs::with_named_args(
            vec![temp_file.path().to_str().unwrap().to_string()],
            vec![("edits".to_string(), edits)].into_iter().collect(),
        );

        let result = tool.execute(&args, &state);
        assert!(result.is_err());
    }
}
