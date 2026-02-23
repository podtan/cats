//! Glob tool implementation compatible with OpenCode
//!
//! Fast file pattern matching tool using glob patterns.

use crate::core::{Tool, ToolArgs, ToolError, ToolResult};
use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;

const LIMIT: usize = 100;

/// Glob tool parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GlobParams {
    /// The glob pattern to match files against
    pub pattern: String,
    /// The directory to search in. If not specified, the current working directory will be used.
    pub path: Option<String>,
}

/// Glob tool for finding files by pattern
pub struct GlobTool {
    name: String,
}

impl GlobTool {
    pub fn new() -> Self {
        Self {
            name: "glob".to_string(),
        }
    }
}

impl Default for GlobTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for GlobTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Fast file pattern matching tool that works with any codebase size"
    }

    fn signature(&self) -> &str {
        "glob --pattern <pattern> [--path <directory>]"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        if args.get_named_arg("pattern").is_none() && args.args.is_empty() {
            return Err(ToolError::InvalidArgs {
                message: "glob tool requires a 'pattern' argument".to_string(),
            });
        }
        Ok(())
    }

    fn execute(
        &mut self,
        args: &ToolArgs,
        state: &Arc<Mutex<crate::state::ToolState>>,
    ) -> Result<ToolResult> {
        let params = parse_glob_args(args)?;

        // Get working directory from ToolState at execution time
        let working_dir = state
            .lock()
            .map(|s| s.working_directory.clone())
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());

        let search_path = params
            .path
            .map(PathBuf::from)
            .unwrap_or_else(|| working_dir.clone());

        let search_path = if search_path.is_absolute() {
            search_path
        } else {
            working_dir.join(&search_path)
        };

        let mut files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
        let mut truncated = false;

        let pattern = glob::Pattern::new(&params.pattern)
            .map_err(|e| anyhow::anyhow!("Invalid glob pattern: {}", e))?;

        for entry in WalkDir::new(&search_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            if files.len() >= LIMIT {
                truncated = true;
                break;
            }

            let path = entry.path().to_path_buf();

            // Check if the relative path matches the pattern
            let relative = path.strip_prefix(&search_path).unwrap_or(&path);
            if pattern.matches_path(relative) {
                let mtime = fs::metadata(&path)
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

                files.push((path, mtime));
            }
        }

        // Sort by modification time (most recent first)
        files.sort_by(|a, b| b.1.cmp(&a.1));

        let file_count = files.len();
        let mut output_lines: Vec<String> = Vec::new();

        if files.is_empty() {
            output_lines.push("No files found".to_string());
        } else {
            for (path, _) in &files {
                output_lines.push(path.display().to_string());
            }

            if truncated {
                output_lines.push("".to_string());
                output_lines.push(
                    "(Results are truncated. Consider using a more specific path or pattern.)"
                        .to_string(),
                );
            }
        }

        Ok(ToolResult::success_with_data(
            output_lines.join("\n"),
            serde_json::json!({
                "count": file_count,
                "truncated": truncated,
                "pattern": params.pattern,
                "path": search_path.display().to_string(),
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        let schema = schemars::schema_for!(GlobParams);
        serde_json::to_value(schema).unwrap_or_default()
    }
}

fn parse_glob_args(args: &ToolArgs) -> Result<GlobParams> {
    let pattern = args
        .get_named_arg("pattern")
        .cloned()
        .or_else(|| args.args.first().cloned())
        .ok_or_else(|| anyhow::anyhow!("pattern is required"))?;

    let path = args.get_named_arg("path").cloned();

    Ok(GlobParams { pattern, path })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_glob_tool_creation() {
        let tool = GlobTool::new();
        assert_eq!(tool.name(), "glob");
    }

    #[test]
    fn test_glob_tool_find_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create some test files
        fs::write(temp_dir.path().join("test1.txt"), "").unwrap();
        fs::write(temp_dir.path().join("test2.txt"), "").unwrap();
        fs::write(temp_dir.path().join("test.rs"), "").unwrap();

        let mut tool = GlobTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let args = ToolArgs::with_named_args(
            vec!["*.txt".to_string()],
            vec![(
                "path".to_string(),
                temp_dir.path().to_str().unwrap().to_string(),
            )]
            .into_iter()
            .collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);
        assert!(result.message.contains("test1.txt"));
        assert!(result.message.contains("test2.txt"));
        assert!(!result.message.contains("test.rs"));
    }

    #[test]
    fn test_glob_tool_no_matches() {
        let temp_dir = TempDir::new().unwrap();

        let mut tool = GlobTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let args = ToolArgs::with_named_args(
            vec!["*.nonexistent".to_string()],
            vec![(
                "path".to_string(),
                temp_dir.path().to_str().unwrap().to_string(),
            )]
            .into_iter()
            .collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);
        assert!(result.message.contains("No files found"));
    }

    #[test]
    fn test_glob_tool_nested_pattern() {
        let temp_dir = TempDir::new().unwrap();

        // Create nested structure
        fs::create_dir_all(temp_dir.path().join("src/nested")).unwrap();
        fs::write(temp_dir.path().join("src/main.rs"), "").unwrap();
        fs::write(temp_dir.path().join("src/nested/mod.rs"), "").unwrap();

        let mut tool = GlobTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let args = ToolArgs::with_named_args(
            vec!["**/*.rs".to_string()],
            vec![(
                "path".to_string(),
                temp_dir.path().to_str().unwrap().to_string(),
            )]
            .into_iter()
            .collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);
        assert!(result.message.contains("main.rs"));
        assert!(result.message.contains("mod.rs"));
    }

    #[test]
    fn test_glob_tool_validation() {
        let tool = GlobTool::new();
        let args = ToolArgs::from_args(&[]);

        let result = tool.validate_args(&args);
        assert!(result.is_err());
    }
}
