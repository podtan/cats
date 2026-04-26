//! Read tool implementation compatible with OpenCode
//!
//! Reads files from the local filesystem with line numbers and windowing.

use crate::core::{Tool, ToolArgs, ToolError, ToolResult};
use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const DEFAULT_READ_LIMIT: usize = 2000;
const MAX_LINE_LENGTH: usize = 2000;

/// Read tool parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadParams {
    /// The path to the file to read
    pub file_path: String,
    /// The line number to start reading from (1-based; default 1)
    pub offset: Option<usize>,
    /// The number of lines to read (defaults to 2000)
    pub limit: Option<usize>,
}

/// Read tool for reading files
pub struct ReadTool {
    name: String,
}

impl ReadTool {
    pub fn new() -> Self {
        Self {
            name: "read".to_string(),
        }
    }
}

impl Default for ReadTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for ReadTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Reads a file from the local filesystem"
    }

    fn signature(&self) -> &str {
        "read --file-path <path> [--offset <n>] [--limit <n>]"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        if args.get_named_arg("file_path").is_none()
            && args.get_named_arg("filePath").is_none()
            && args.args.is_empty()
        {
            return Err(ToolError::InvalidArgs {
                message: "read tool requires a 'file_path' argument".to_string(),
            });
        }
        Ok(())
    }

    fn execute(
        &mut self,
        args: &ToolArgs,
        state: &Arc<Mutex<crate::state::ToolState>>,
    ) -> Result<ToolResult> {
        let params = parse_read_args(args)?;

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

        // Check if file exists
        if !filepath.exists() {
            // Try to find similar files for suggestions
            let dir = filepath.parent().unwrap_or(Path::new("."));
            let base = filepath.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if dir.exists() {
                let entries: Vec<String> = fs::read_dir(dir)?
                    .filter_map(|e| e.ok())
                    .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
                    .filter(|s| {
                        s.to_lowercase().contains(&base.to_lowercase())
                            || base.to_lowercase().contains(&s.to_lowercase())
                    })
                    .take(3)
                    .collect();

                if !entries.is_empty() {
                    let suggestions: Vec<String> = entries
                        .iter()
                        .map(|e| dir.join(e).display().to_string())
                        .collect();
                    return Err(anyhow::anyhow!(
                        "File not found: {}\n\nDid you mean one of these?\n{}",
                        filepath.display(),
                        suggestions.join("\n")
                    ));
                }
            }

            return Err(ToolError::FileNotFound {
                path: filepath.display().to_string(),
            }
            .into());
        }

        if filepath.is_dir() {
            return Err(anyhow::anyhow!(
                "Path is a directory, not a file: {}",
                filepath.display()
            ));
        }

        // Check for binary file
        if is_binary_file(&filepath)? {
            return Err(anyhow::anyhow!(
                "Cannot read binary file: {}",
                filepath.display()
            ));
        }

        // Read file content
        let content = fs::read_to_string(&filepath)?;
        let lines: Vec<&str> = content.lines().collect();

        let offset = params.offset.unwrap_or(1);
        let limit = params.limit.unwrap_or(DEFAULT_READ_LIMIT);

        // Convert 1-based offset to 0-based skip count (matches OpenCode behaviour)
        let skip = offset.saturating_sub(1);

        // Get the requested window
        let _end = std::cmp::min(skip + limit, lines.len());
        let window: Vec<&str> = lines.iter().skip(skip).take(limit).copied().collect();

        // Format with line numbers
        let formatted: Vec<String> = window
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let line_num = skip + i + 1;
                let truncated = if line.len() > MAX_LINE_LENGTH {
                    format!("{}...", &line[..MAX_LINE_LENGTH])
                } else {
                    line.to_string()
                };
                format!("{:05}| {}", line_num, truncated)
            })
            .collect();

        let total_lines = lines.len();
        let last_read_line = skip + window.len();
        let has_more_lines = total_lines > last_read_line;

        let mut output = String::from("<file>\n");
        output.push_str(&formatted.join("\n"));

        if has_more_lines {
            output.push_str(&format!(
                "\n\n(File has more lines. Use 'offset' parameter to read beyond line {})",
                last_read_line
            ));
        } else {
            output.push_str(&format!("\n\n(End of file - total {} lines)", total_lines));
        }
        output.push_str("\n</file>");

        // Create preview (first 20 lines)
        let preview: String = window
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<&str>>()
            .join("\n");

        Ok(ToolResult::success_with_data(
            output,
            serde_json::json!({
                "preview": preview,
                "file_path": filepath.display().to_string(),
                "total_lines": total_lines,
                "offset": offset,
                "limit": limit,
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        let schema = schemars::schema_for!(ReadParams);
        serde_json::to_value(schema).unwrap_or_default()
    }
}

fn parse_read_args(args: &ToolArgs) -> Result<ReadParams> {
    let file_path = args
        .get_named_arg("file_path")
        .cloned()
        .or_else(|| args.get_named_arg("filePath").cloned())
        .or_else(|| args.args.first().cloned())
        .ok_or_else(|| anyhow::anyhow!("file_path is required"))?;

    let offset = args
        .get_named_arg("offset")
        .and_then(|s| s.parse::<usize>().ok());

    let limit = args
        .get_named_arg("limit")
        .and_then(|s| s.parse::<usize>().ok());

    Ok(ReadParams {
        file_path,
        offset,
        limit,
    })
}

fn is_binary_file(path: &Path) -> Result<bool> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // Known binary extensions
    match ext.as_str() {
        "zip" | "tar" | "gz" | "exe" | "dll" | "so" | "class" | "jar" | "war" | "7z" | "doc"
        | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp" | "bin" | "dat"
        | "obj" | "o" | "a" | "lib" | "wasm" | "pyc" | "pyo" | "png" | "jpg" | "jpeg" | "gif"
        | "pdf" | "ico" | "webp" | "mp3" | "mp4" | "avi" | "mov" => return Ok(true),
        _ => {}
    }

    // Check file content for binary data
    let metadata = fs::metadata(path)?;
    if metadata.len() == 0 {
        return Ok(false);
    }

    let mut file = fs::File::open(path)?;
    let mut buffer = [0u8; 4096];
    let bytes_read = std::io::Read::read(&mut file, &mut buffer)?;

    if bytes_read == 0 {
        return Ok(false);
    }

    let bytes = &buffer[..bytes_read];

    // Check for null bytes (common in binary files)
    if bytes.contains(&0) {
        return Ok(true);
    }

    // Count non-printable characters
    let non_printable = bytes
        .iter()
        .filter(|&&b| b < 9 || (b > 13 && b < 32))
        .count();

    // If >30% non-printable, consider binary
    Ok(non_printable as f32 / bytes_read as f32 > 0.3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_read_tool_creation() {
        let tool = ReadTool::new();
        assert_eq!(tool.name(), "read");
    }

    #[test]
    fn test_read_tool_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "line 1").unwrap();
        writeln!(temp_file, "line 2").unwrap();
        writeln!(temp_file, "line 3").unwrap();

        let mut tool = ReadTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let args = ToolArgs::from_args(&[temp_file.path().to_str().unwrap()]);

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);
        assert!(result.message.contains("line 1"));
        assert!(result.message.contains("line 2"));
        assert!(result.message.contains("line 3"));
    }

    #[test]
    fn test_read_tool_with_offset() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "line 1").unwrap();
        writeln!(temp_file, "line 2").unwrap();
        writeln!(temp_file, "line 3").unwrap();

        let mut tool = ReadTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let args = ToolArgs::with_named_args(
            vec![temp_file.path().to_str().unwrap().to_string()],
            vec![("offset".to_string(), "1".to_string())]
                .into_iter()
                .collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);
        assert!(!result.message.contains("00001| line 1"));
        assert!(result.message.contains("00002| line 2"));
    }

    #[test]
    fn test_read_tool_file_not_found() {
        let mut tool = ReadTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let args = ToolArgs::from_args(&["/nonexistent/file.txt"]);

        let result = tool.execute(&args, &state);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_tool_validation() {
        let tool = ReadTool::new();
        let args = ToolArgs::from_args(&[]);

        let result = tool.validate_args(&args);
        assert!(result.is_err());
    }
}
