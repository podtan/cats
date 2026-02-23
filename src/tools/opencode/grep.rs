//! Grep tool implementation compatible with OpenCode
//!
//! Fast content search tool using regex patterns.

use crate::core::{Tool, ToolArgs, ToolError, ToolResult};
use anyhow::Result;
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;

const MAX_LINE_LENGTH: usize = 2000;
const LIMIT: usize = 100;

/// Grep tool parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GrepParams {
    /// The regex pattern to search for in file contents
    pub pattern: String,
    /// The directory to search in. Defaults to the current working directory.
    pub path: Option<String>,
    /// File pattern to include in the search (e.g. "*.js", "*.{ts,tsx}")
    pub include: Option<String>,
}

/// Grep tool for searching file contents
pub struct GrepTool {
    name: String,
}

impl GrepTool {
    pub fn new() -> Self {
        Self {
            name: "grep".to_string(),
        }
    }
}

impl Default for GrepTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for GrepTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Fast content search tool that searches file contents using regular expressions"
    }

    fn signature(&self) -> &str {
        "grep --pattern <regex> [--path <directory>] [--include <glob>]"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        if args.get_named_arg("pattern").is_none() && args.args.is_empty() {
            return Err(ToolError::InvalidArgs {
                message: "grep tool requires a 'pattern' argument".to_string(),
            });
        }
        Ok(())
    }

    fn execute(
        &mut self,
        args: &ToolArgs,
        state: &Arc<Mutex<crate::state::ToolState>>,
    ) -> Result<ToolResult> {
        let params = parse_grep_args(args)?;

        if params.pattern.is_empty() {
            return Err(ToolError::InvalidArgs {
                message: "pattern is required".to_string(),
            }
            .into());
        }

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

        // Compile the regex pattern
        let regex = Regex::new(&params.pattern)
            .map_err(|e| anyhow::anyhow!("Invalid regex pattern: {}", e))?;

        // Parse include glob pattern if provided
        let include_glob = params
            .include
            .as_ref()
            .map(|g| glob::Pattern::new(g))
            .transpose()
            .map_err(|e| anyhow::anyhow!("Invalid include pattern: {}", e))?;

        let mut matches: Vec<(PathBuf, std::time::SystemTime, usize, String)> = Vec::new();
        let mut truncated = false;

        for entry in WalkDir::new(&search_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            if truncated {
                break;
            }

            let path = entry.path().to_path_buf();

            // Check include pattern
            if let Some(ref glob) = include_glob {
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !glob.matches(filename) {
                    continue;
                }
            }

            // Skip binary files
            if is_likely_binary(&path) {
                continue;
            }

            // Read file and search
            if let Ok(content) = fs::read_to_string(&path) {
                let mtime = fs::metadata(&path)
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

                for (line_num, line) in content.lines().enumerate() {
                    if matches.len() >= LIMIT {
                        truncated = true;
                        break;
                    }

                    if regex.is_match(line) {
                        let truncated_line = if line.len() > MAX_LINE_LENGTH {
                            format!("{}...", &line[..MAX_LINE_LENGTH])
                        } else {
                            line.to_string()
                        };

                        matches.push((path.clone(), mtime, line_num + 1, truncated_line));
                    }
                }
            }
        }

        // Sort by modification time (most recent first)
        matches.sort_by(|a, b| b.1.cmp(&a.1));

        if matches.is_empty() {
            return Ok(ToolResult::success_with_data(
                "No files found".to_string(),
                serde_json::json!({
                    "matches": 0,
                    "truncated": false,
                }),
            ));
        }

        // Format output
        let mut output_lines = vec![format!("Found {} matches", matches.len())];
        let mut current_file: Option<&PathBuf> = None;

        for (path, _mtime, line_num, line_text) in &matches {
            if current_file != Some(path) {
                if current_file.is_some() {
                    output_lines.push("".to_string());
                }
                current_file = Some(path);
                output_lines.push(format!("{}:", path.display()));
            }
            output_lines.push(format!("  Line {}: {}", line_num, line_text));
        }

        if truncated {
            output_lines.push("".to_string());
            output_lines.push(
                "(Results are truncated. Consider using a more specific path or pattern.)"
                    .to_string(),
            );
        }

        Ok(ToolResult::success_with_data(
            output_lines.join("\n"),
            serde_json::json!({
                "matches": matches.len(),
                "truncated": truncated,
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        let schema = schemars::schema_for!(GrepParams);
        serde_json::to_value(schema).unwrap_or_default()
    }
}

fn parse_grep_args(args: &ToolArgs) -> Result<GrepParams> {
    let pattern = args
        .get_named_arg("pattern")
        .cloned()
        .or_else(|| args.args.first().cloned())
        .ok_or_else(|| anyhow::anyhow!("pattern is required"))?;

    let path = args.get_named_arg("path").cloned();

    let include = args.get_named_arg("include").cloned();

    Ok(GrepParams {
        pattern,
        path,
        include,
    })
}

fn is_likely_binary(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    matches!(
        ext.as_str(),
        "zip"
            | "tar"
            | "gz"
            | "exe"
            | "dll"
            | "so"
            | "class"
            | "jar"
            | "war"
            | "7z"
            | "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "pdf"
            | "ico"
            | "webp"
            | "mp3"
            | "mp4"
            | "avi"
            | "mov"
            | "wasm"
            | "pyc"
            | "pyo"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_grep_tool_creation() {
        let tool = GrepTool::new();
        assert_eq!(tool.name(), "grep");
    }

    #[test]
    fn test_grep_tool_find_content() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        fs::write(
            temp_dir.path().join("test1.txt"),
            "Hello, World!\nThis is a test.",
        )
        .unwrap();
        fs::write(temp_dir.path().join("test2.txt"), "No match here.").unwrap();
        fs::write(
            temp_dir.path().join("test3.rs"),
            "fn main() { println!(\"Hello\"); }",
        )
        .unwrap();

        let mut tool = GrepTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let args = ToolArgs::with_named_args(
            vec!["Hello".to_string()],
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
        assert!(result.message.contains("test3.rs"));
        assert!(!result.message.contains("test2.txt"));
    }

    #[test]
    fn test_grep_tool_with_include() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(temp_dir.path().join("test.txt"), "Hello, World!").unwrap();
        fs::write(temp_dir.path().join("test.rs"), "Hello, Rust!").unwrap();

        let mut tool = GrepTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let args = ToolArgs::with_named_args(
            vec!["Hello".to_string()],
            vec![
                (
                    "path".to_string(),
                    temp_dir.path().to_str().unwrap().to_string(),
                ),
                ("include".to_string(), "*.rs".to_string()),
            ]
            .into_iter()
            .collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);
        assert!(result.message.contains("test.rs"));
        assert!(!result.message.contains("test.txt"));
    }

    #[test]
    fn test_grep_tool_no_matches() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(temp_dir.path().join("test.txt"), "Nothing to see here.").unwrap();

        let mut tool = GrepTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let args = ToolArgs::with_named_args(
            vec!["nonexistent".to_string()],
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
    fn test_grep_tool_regex_pattern() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(temp_dir.path().join("test.txt"), "Line 1\nLine 2\nLine 123").unwrap();

        let mut tool = GrepTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let args = ToolArgs::with_named_args(
            vec!["Line \\d+".to_string()],
            vec![(
                "path".to_string(),
                temp_dir.path().to_str().unwrap().to_string(),
            )]
            .into_iter()
            .collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);
        assert!(result.message.contains("Line 1"));
        assert!(result.message.contains("Line 2"));
        assert!(result.message.contains("Line 123"));
    }

    #[test]
    fn test_grep_tool_validation() {
        let tool = GrepTool::new();
        let args = ToolArgs::from_args(&[]);

        let result = tool.validate_args(&args);
        assert!(result.is_err());
    }
}
