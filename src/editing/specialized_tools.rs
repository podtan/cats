//! Specialized editing tools for better AI model compatibility
//!
//! This module provides simple, single-responsibility tools that replace
//! the complex monolithic edit tool for improved compatibility with models
//! like Grok-Code-Fast-1.

use crate::core::{Tool, ToolArgs, ToolError, ToolResult};
use crate::state::ToolState;
use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
/// Tool for creating new files with content
pub struct CreateFileTool {
    name: String,
}

impl CreateFileTool {
    pub fn new() -> Self {
        Self {
            name: "create_file".to_string(),
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
                "path".to_string(),
                serde_json::Value::String(args.get_arg(0).unwrap().clone()),
            );
            params.insert(
                "content".to_string(),
                serde_json::Value::String(args.get_arg(1).unwrap().clone()),
            );
            return Ok(serde_json::Value::Object(params));
        }

        Err(ToolError::InvalidArgs {
            message: "Insufficient parameters".to_string(),
        })
    }
}

impl Tool for CreateFileTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Create a new file with specified content"
    }

    fn signature(&self) -> &str {
        "create_file(path: str, content: str)"
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

        if !obj.contains_key("content") {
            return Err(ToolError::InvalidArgs {
                message: "Missing required parameter: content".to_string(),
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

        let content = obj
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid content parameter"))?;

        let path = PathBuf::from(path_str);

        // Check if file already exists
        if path.exists() {
            return Ok(ToolResult::error(format!(
                "File already exists: {}",
                path.display()
            )));
        }

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| anyhow::anyhow!("Failed to create parent directories: {}", e))?;
        }

        // Create the file
        fs::write(&path, content).map_err(|e| anyhow::anyhow!("Failed to create file: {}", e))?;

        // Update state
        {
            let mut state_guard = state
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;
            state_guard.push_history(format!("Created file: {}", path.display()));
        }

        Ok(ToolResult::success_with_data(
            format!("Successfully created file: {}", path.display()),
            serde_json::json!({
                "path": path.to_string_lossy(),
                "content_length": content.len(),
                "lines_created": content.lines().count(),
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
                    "description": "Full path to the file to create"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }
}

/// Tool for replacing text in files
pub struct ReplaceTextTool {
    name: String,
}

impl ReplaceTextTool {
    pub fn new() -> Self {
        Self {
            name: "replace_text".to_string(),
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
        if args.len() >= 3 {
            let mut params = serde_json::Map::new();
            params.insert(
                "path".to_string(),
                serde_json::Value::String(args.get_arg(0).unwrap().clone()),
            );
            params.insert(
                "old_text".to_string(),
                serde_json::Value::String(args.get_arg(1).unwrap().clone()),
            );
            params.insert(
                "new_text".to_string(),
                serde_json::Value::String(args.get_arg(2).unwrap().clone()),
            );

            if args.len() >= 4 {
                if let Ok(occurrence) = args.get_arg(3).unwrap().parse::<usize>() {
                    params.insert(
                        "occurrence".to_string(),
                        serde_json::Value::Number(occurrence.into()),
                    );
                }
            }

            return Ok(serde_json::Value::Object(params));
        }

        Err(ToolError::InvalidArgs {
            message: "Insufficient parameters".to_string(),
        })
    }
}

impl Tool for ReplaceTextTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Replace specific text in a file"
    }

    fn signature(&self) -> &str {
        "replace_text(path: str, old_text: str, new_text: str, occurrence?: int)"
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

        if !obj.contains_key("old_text") {
            return Err(ToolError::InvalidArgs {
                message: "Missing required parameter: old_text".to_string(),
            });
        }

        if !obj.contains_key("new_text") {
            return Err(ToolError::InvalidArgs {
                message: "Missing required parameter: new_text".to_string(),
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

        let old_text = obj
            .get("old_text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid old_text parameter"))?;

        let new_text = obj
            .get("new_text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid new_text parameter"))?;

        let occurrence = obj
            .get("occurrence")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        let path = PathBuf::from(path_str);

        // Check if file exists
        if !path.exists() {
            return Ok(ToolResult::error(format!(
                "File not found: {}",
                path.display()
            )));
        }

        // Read the file
        let content =
            fs::read_to_string(&path).map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?;

        // Find all matches
        let mut matches = Vec::new();
        let mut start = 0;

        while let Some(pos) = content[start..].find(old_text) {
            matches.push(start + pos);
            start += pos + old_text.len();
        }

        if matches.is_empty() {
            return Ok(ToolResult::error(format!("Text not found: '{}'", old_text)));
        }

        // Handle multiple matches
        if matches.len() > 1 && occurrence.is_none() {
            // Build a short preview with line numbers for each match to help disambiguate
            let mut previews: Vec<String> = Vec::new();
            // Build mapping from byte offsets to line numbers
            let mut line_start_offsets: Vec<usize> = Vec::new();
            let mut acc = 0usize;
            for line in content.lines() {
                line_start_offsets.push(acc);
                acc += line.len() + 1; // account for the '\n' that was removed by lines()
            }
            for &pos in matches.iter() {
                // find the largest line_start_offset <= pos
                let mut line_no = 0usize;
                for (ln_idx, &off) in line_start_offsets.iter().enumerate() {
                    if off <= pos {
                        line_no = ln_idx + 1; // 1-based
                    } else {
                        break;
                    }
                }
                // get the line content (safe)
                let line_content = content.lines().nth(line_no.saturating_sub(1)).unwrap_or("");
                let preview = format!("{}: {}", line_no, line_content.trim());
                previews.push(preview);
                if previews.len() >= 5 {
                    break;
                } // limit previews
            }
            let preview_text = if previews.is_empty() {
                "".to_string()
            } else {
                format!("\nMatches (line: snippet):\n{}", previews.join("\n"))
            };

            return Ok(ToolResult::error(format!(
                "Found {} occurrences of '{}'. Use 'occurrence' parameter to specify which one to replace (1-{}).{}",
                matches.len(), old_text, matches.len(), preview_text
            )));
        }

        // Select occurrence
        let selected_pos = if let Some(occ) = occurrence {
            if occ == 0 || occ > matches.len() {
                return Ok(ToolResult::error(format!(
                    "Invalid occurrence {}. Found {} matches",
                    occ,
                    matches.len()
                )));
            }
            matches[occ - 1]
        } else {
            matches[0] // Single match or first match
        };

        // Replace the text
        let mut new_content = content;
        let start_pos = selected_pos;
        let end_pos = start_pos + old_text.len();
        new_content.replace_range(start_pos..end_pos, new_text);

        // Write the file
        fs::write(&path, &new_content)
            .map_err(|e| anyhow::anyhow!("Failed to write file: {}", e))?;

        // Update state
        {
            let mut state_guard = state
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;
            if state_guard.current_file.as_ref() == Some(&path) {
                let lines: Vec<String> = new_content.lines().map(|s| s.to_string()).collect();
                state_guard.open_file(path.clone(), lines, 100)?;
            }
            state_guard.push_history(format!("Replaced text in: {}", path.display()));
        }

        let occurrence_text = occurrence.unwrap_or(1);
        let chars_changed = new_text.len() as i64 - old_text.len() as i64;

        Ok(ToolResult::success_with_data(
            format!(
                "Successfully replaced occurrence {} in {}",
                occurrence_text,
                path.display()
            ),
            serde_json::json!({
                "path": path.to_string_lossy(),
                "occurrence": occurrence_text,
                "old_text": old_text,
                "new_text": new_text,
                "characters_changed": chars_changed,
                "total_matches": matches.len()
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Full path to the file"
                },
                "old_text": {
                    "type": "string",
                    "description": "Exact text to replace"
                },
                "new_text": {
                    "type": "string",
                    "description": "Replacement text"
                },
                "occurrence": {
                    "type": "integer",
                    "description": "Which occurrence to replace (1-based, default: 1 if only one match)",
                    "minimum": 1
                }
            },
            "required": ["path", "old_text", "new_text"]
        })
    }
}

/// Tool for inserting text at a specific line
pub struct InsertTextTool {
    name: String,
}

impl InsertTextTool {
    pub fn new() -> Self {
        Self {
            name: "insert_text".to_string(),
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
        if args.len() >= 3 {
            let mut params = serde_json::Map::new();
            params.insert(
                "path".to_string(),
                serde_json::Value::String(args.get_arg(0).unwrap().clone()),
            );

            // Parse line number as integer
            if let Ok(line_number) = args.get_arg(1).unwrap().parse::<u64>() {
                params.insert(
                    "line_number".to_string(),
                    serde_json::Value::Number(line_number.into()),
                );
            } else {
                return Err(ToolError::InvalidArgs {
                    message: "Invalid line_number - must be a number".to_string(),
                });
            }

            params.insert(
                "text".to_string(),
                serde_json::Value::String(args.get_arg(2).unwrap().clone()),
            );

            if args.len() >= 4 {
                params.insert(
                    "position".to_string(),
                    serde_json::Value::String(args.get_arg(3).unwrap().clone()),
                );
            }

            return Ok(serde_json::Value::Object(params));
        }

        Err(ToolError::InvalidArgs {
            message: "Insufficient parameters".to_string(),
        })
    }
}

impl Tool for InsertTextTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Insert text at a specific line in a file"
    }

    fn signature(&self) -> &str {
        "insert_text(path: str, line_number: int, text: str, position?: str)"
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

        if !obj.contains_key("line_number") {
            return Err(ToolError::InvalidArgs {
                message: "Missing required parameter: line_number".to_string(),
            });
        }

        if !obj.contains_key("text") {
            return Err(ToolError::InvalidArgs {
                message: "Missing required parameter: text".to_string(),
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

        let line_number = {
            let v = obj
                .get("line_number")
                .ok_or_else(|| anyhow::anyhow!("Invalid line_number parameter"))?;
            if let Some(n) = v.as_u64() {
                n as usize
            } else if let Some(s) = v.as_str() {
                s.parse::<u64>()
                    .map_err(|_| anyhow::anyhow!("Invalid line_number parameter"))?
                    as usize
            } else {
                return Err(anyhow::anyhow!("Invalid line_number parameter"));
            }
        };

        let text = obj
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid text parameter"))?;

        let position = obj
            .get("position")
            .and_then(|v| v.as_str())
            .unwrap_or("after_line"); // Default position

        let path = PathBuf::from(path_str);

        // Check if file exists
        if !path.exists() {
            return Ok(ToolResult::error(format!(
                "File not found: {}",
                path.display()
            )));
        }

        // Read the file
        let content =
            fs::read_to_string(&path).map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?;

        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        // Validate line number
        if line_number == 0 || line_number > lines.len() + 1 {
            return Ok(ToolResult::error(format!(
                "Invalid line number {}. File has {} lines",
                line_number,
                lines.len()
            )));
        }

        // Insert text based on position
        // Note: insertion index must be in 0..=lines.len().
        // The API accepts a 1-based `line_number` and allows `line_number == lines.len() + 1`
        // to mean "at the end". For `after_line` we must clamp the index to `lines.len()`
        // so that inserting after the last line becomes an append instead of producing
        // an out-of-range index (which would panic when calling Vec::insert).
        let insert_index = match position {
            "before_line" => line_number - 1,
            // Clamp to lines.len() so that `after_line` with line_number == lines.len() + 1
            // becomes an append (index == lines.len()). This also handles the empty-file case
            // where lines.len() == 0 and line_number == 1 -> insert_index == 0.
            "after_line" => std::cmp::min(line_number, lines.len()),
            "at_end" => lines.len(),
            _ => {
                return Ok(ToolResult::error(format!(
                    "Invalid position '{}'. Use 'before_line', 'after_line', or 'at_end'",
                    position
                )))
            }
        };

        // Insert the text
        lines.insert(insert_index, text.to_string());

        // Write the file
        let new_content = lines.join("\n");
        if !new_content.is_empty() && !new_content.ends_with('\n') {
            fs::write(&path, format!("{}\n", new_content))
                .map_err(|e| anyhow::anyhow!("Failed to write file: {}", e))?;
        } else {
            fs::write(&path, new_content)
                .map_err(|e| anyhow::anyhow!("Failed to write file: {}", e))?;
        }

        // Update state
        {
            let mut state_guard = state
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;
            if state_guard.current_file.as_ref() == Some(&path) {
                state_guard.open_file(path.clone(), lines, 100)?;
            }
            state_guard.push_history(format!(
                "Inserted text at line {} in: {}",
                insert_index + 1,
                path.display()
            ));
        }

        Ok(ToolResult::success_with_data(
            format!(
                "Successfully inserted text at line {} in {}",
                insert_index + 1,
                path.display()
            ),
            serde_json::json!({
                "path": path.to_string_lossy(),
                "line_number": insert_index + 1,
                "position": position,
                "text": text,
                "lines_added": 1
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Full path to the file"
                },
                "line_number": {
                    "type": "integer",
                    "description": "Line number where to insert text (1-based)",
                    "minimum": 1
                },
                "text": {
                    "type": "string",
                    "description": "Text to insert"
                },
                "position": {
                    "type": "string",
                    "description": "Where to insert relative to line number",
                    "enum": ["before_line", "after_line", "at_end"],
                    "default": "after_line"
                }
            },
            "required": ["path", "line_number", "text"]
        })
    }
}

/// Tool for deleting text from files
/// Tool for deleting lines by range
pub struct DeleteLineTool {
    name: String,
}

impl DeleteLineTool {
    pub fn new() -> Self {
        Self {
            name: "delete_line".to_string(),
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
            let mut params = serde_json::Map::new();
            for (key, value) in &args.named_args {
                if key == "start_line" || key == "end_line" {
                    // Parse numeric values
                    let numeric_value =
                        value.parse::<u64>().map_err(|_| ToolError::InvalidArgs {
                            message: format!("Invalid {} - must be a number", key),
                        })?;
                    params.insert(key.clone(), serde_json::Value::Number(numeric_value.into()));
                } else {
                    params.insert(key.clone(), serde_json::Value::String(value.clone()));
                }
            }
            return Ok(serde_json::Value::Object(params));
        }

        // Fall back to positional arguments for backward compatibility
        if args.len() >= 3 {
            let mut params = serde_json::Map::new();
            params.insert(
                "path".to_string(),
                serde_json::Value::String(args.get_arg(0).unwrap().clone()),
            );
            // Parse start_line and end_line
            let start_line =
                args.get_arg(1)
                    .unwrap()
                    .parse::<u64>()
                    .map_err(|_| ToolError::InvalidArgs {
                        message: "Invalid start_line - must be a number".to_string(),
                    })?;
            let end_line =
                args.get_arg(2)
                    .unwrap()
                    .parse::<u64>()
                    .map_err(|_| ToolError::InvalidArgs {
                        message: "Invalid end_line - must be a number".to_string(),
                    })?;
            params.insert(
                "start_line".to_string(),
                serde_json::Value::Number(start_line.into()),
            );
            params.insert(
                "end_line".to_string(),
                serde_json::Value::Number(end_line.into()),
            );
            return Ok(serde_json::Value::Object(params));
        }

        Err(ToolError::InvalidArgs {
            message: "Insufficient parameters".to_string(),
        })
    }
}

impl Tool for DeleteLineTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Delete a range of lines from a file (inclusive)"
    }

    fn signature(&self) -> &str {
        "delete_line(path: str, start_line: int, end_line: int)"
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
        if !obj.contains_key("start_line") {
            return Err(ToolError::InvalidArgs {
                message: "Missing required parameter: start_line".to_string(),
            });
        }
        if !obj.contains_key("end_line") {
            return Err(ToolError::InvalidArgs {
                message: "Missing required parameter: end_line".to_string(),
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
        let start_line_u64 = obj
            .get("start_line")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("Invalid start_line parameter"))?;
        let end_line_u64 = obj
            .get("end_line")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("Invalid end_line parameter"))?;

        let start_line = start_line_u64 as usize;
        let mut end_line = end_line_u64 as usize;

        if start_line == 0 {
            return Ok(ToolResult::error(
                "Invalid start_line 0. Line numbers are 1-based".to_string(),
            ));
        }
        if end_line < start_line {
            return Ok(ToolResult::error(format!(
                "Invalid line range: {} > {}",
                start_line, end_line
            )));
        }

        let path = PathBuf::from(path_str);
        if !path.exists() {
            return Ok(ToolResult::error(format!(
                "File not found: {}",
                path.display()
            )));
        }

        let content =
            fs::read_to_string(&path).map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let total_lines = lines.len();

        if start_line > total_lines {
            return Ok(ToolResult::error(format!(
                "Invalid start_line {}. File has {} lines",
                start_line, total_lines
            )));
        }
        if end_line > total_lines {
            end_line = total_lines; // clamp to end for convenience
        }

        // Convert to 0-based inclusive range
        let start_idx = start_line - 1;
        let end_idx = end_line - 1;
        let lines_to_delete = end_idx - start_idx + 1;

        // Remove the range
        lines.drain(start_idx..=end_idx);

        // Write back
        let new_content = lines.join("\n");
        if !new_content.is_empty() && !new_content.ends_with('\n') {
            fs::write(&path, format!("{}\n", new_content))
                .map_err(|e| anyhow::anyhow!("Failed to write file: {}", e))?;
        } else {
            fs::write(&path, new_content)
                .map_err(|e| anyhow::anyhow!("Failed to write file: {}", e))?;
        }

        // Update state
        {
            let mut state_guard = state
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;
            if state_guard.current_file.as_ref() == Some(&path) {
                state_guard.open_file(path.clone(), lines.clone(), 100)?;
            }
            state_guard.push_history(format!(
                "Deleted lines {}-{} in: {}",
                start_line,
                end_line,
                path.display()
            ));
        }

        Ok(ToolResult::success_with_data(
            format!(
                "Successfully deleted lines {}-{} in {}",
                start_line,
                end_line,
                path.display()
            ),
            serde_json::json!({
                "path": path.to_string_lossy(),
                "start_line": start_line,
                "end_line": end_line,
                "lines_deleted": lines_to_delete,
                "total_lines": lines.len()
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Full path to the file" },
                "start_line": { "type": "integer", "minimum": 1, "description": "Start line (1-based, inclusive)" },
                "end_line": { "type": "integer", "minimum": 1, "description": "End line (1-based, inclusive)" }
            },
            "required": ["path", "start_line", "end_line"]
        })
    }
}
pub struct DeleteTextTool {
    name: String,
}

impl DeleteTextTool {
    pub fn new() -> Self {
        Self {
            name: "delete_text".to_string(),
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
                "path".to_string(),
                serde_json::Value::String(args.get_arg(0).unwrap().clone()),
            );
            params.insert(
                "text_to_delete".to_string(),
                serde_json::Value::String(args.get_arg(1).unwrap().clone()),
            );

            if args.len() >= 3 {
                if let Ok(occurrence) = args.get_arg(2).unwrap().parse::<usize>() {
                    params.insert(
                        "occurrence".to_string(),
                        serde_json::Value::Number(occurrence.into()),
                    );
                }
            }

            return Ok(serde_json::Value::Object(params));
        }

        Err(ToolError::InvalidArgs {
            message: "Insufficient parameters".to_string(),
        })
    }
}

impl Tool for DeleteTextTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Delete specific text from a file"
    }

    fn signature(&self) -> &str {
        "delete_text(path: str, text_to_delete: str, occurrence?: int)"
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

        if !obj.contains_key("text_to_delete") {
            return Err(ToolError::InvalidArgs {
                message: "Missing required parameter: text_to_delete".to_string(),
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

        let text_to_delete = obj
            .get("text_to_delete")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid text_to_delete parameter"))?;

        let occurrence = obj
            .get("occurrence")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        let path = PathBuf::from(path_str);

        // Check if file exists
        if !path.exists() {
            return Ok(ToolResult::error(format!(
                "File not found: {}",
                path.display()
            )));
        }

        // Read the file
        let content =
            fs::read_to_string(&path).map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?;

        // Find all matches
        let mut matches = Vec::new();
        let mut start = 0;

        while let Some(pos) = content[start..].find(text_to_delete) {
            matches.push(start + pos);
            start += pos + text_to_delete.len();
        }

        if matches.is_empty() {
            return Ok(ToolResult::error(format!(
                "Text not found: '{}'",
                text_to_delete
            )));
        }

        // Handle multiple matches
        if matches.len() > 1 && occurrence.is_none() {
            return Ok(ToolResult::error(format!(
                "Found {} occurrences of '{}'. Use 'occurrence' parameter to specify which one to delete (1-{})",
                matches.len(), text_to_delete, matches.len()
            )));
        }

        // Select occurrence
        let selected_pos = if let Some(occ) = occurrence {
            if occ == 0 || occ > matches.len() {
                return Ok(ToolResult::error(format!(
                    "Invalid occurrence {}. Found {} matches",
                    occ,
                    matches.len()
                )));
            }
            matches[occ - 1]
        } else {
            matches[0] // Single match or first match
        };

        // Delete the text (replace with empty string)
        let mut new_content = content;
        let start_pos = selected_pos;
        let end_pos = start_pos + text_to_delete.len();
        new_content.replace_range(start_pos..end_pos, "");

        // Write the file
        fs::write(&path, &new_content)
            .map_err(|e| anyhow::anyhow!("Failed to write file: {}", e))?;

        // Update state
        {
            let mut state_guard = state
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;
            if state_guard.current_file.as_ref() == Some(&path) {
                let lines: Vec<String> = new_content.lines().map(|s| s.to_string()).collect();
                state_guard.open_file(path.clone(), lines, 100)?;
            }
            state_guard.push_history(format!("Deleted text from: {}", path.display()));
        }

        let occurrence_text = occurrence.unwrap_or(1);

        Ok(ToolResult::success_with_data(
            format!(
                "Successfully deleted occurrence {} from {}",
                occurrence_text,
                path.display()
            ),
            serde_json::json!({
                "path": path.to_string_lossy(),
                "occurrence": occurrence_text,
                "deleted_text": text_to_delete,
                "characters_removed": text_to_delete.len(),
                "total_matches": matches.len()
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Full path to the file"
                },
                "text_to_delete": {
                    "type": "string",
                    "description": "Exact text to delete"
                },
                "occurrence": {
                    "type": "integer",
                    "description": "Which occurrence to delete (1-based, default: 1 if only one match)",
                    "minimum": 1
                }
            },
            "required": ["path", "text_to_delete"]
        })
    }
}

/// Tool for deleting a function definition by name (language-aware)
pub struct DeleteFunctionTool {
    name: String,
}

impl DeleteFunctionTool {
    pub fn new() -> Self {
        Self {
            name: "delete_function".to_string(),
        }
    }

    /// Parse parameters from ToolArgs
    fn parse_params(&self, args: &ToolArgs) -> Result<serde_json::Value, ToolError> {
        // Try to parse as JSON first
        if let Some(json_str) = args.get_named_arg("json") {
            return serde_json::from_str(json_str).map_err(|e| ToolError::Json(e));
        }

        // Structured named args
        if !args.named_args.is_empty() {
            return Ok(serde_json::to_value(&args.named_args).map_err(|e| ToolError::Json(e))?);
        }

        // Positional: file_name, function_name
        if args.len() >= 2 {
            let mut params = serde_json::Map::new();
            params.insert(
                "file_name".to_string(),
                serde_json::Value::String(args.get_arg(0).unwrap().clone()),
            );
            params.insert(
                "function_name".to_string(),
                serde_json::Value::String(args.get_arg(1).unwrap().clone()),
            );
            return Ok(serde_json::Value::Object(params));
        }

        Err(ToolError::InvalidArgs {
            message: "Insufficient parameters".to_string(),
        })
    }

    fn guess_language(&self, path: &std::path::Path) -> Option<&'static str> {
        match path.extension().and_then(|s| s.to_str()) {
            Some("rs") => Some("rust"),
            _ => None,
        }
    }

    fn delete_rust_function(
        content: &str,
        func_name: &str,
    ) -> Result<Option<(String, usize, usize)>> {
        use regex::Regex;
        let pattern = format!(
            r"(?m)^[ \t]*(?:pub[ \t]+)?(?:async[ \t]+)?(?:const[ \t]+)?(?:unsafe[ \t]+)?fn[ \t]+{}\b",
            regex::escape(func_name)
        );
        let re = Regex::new(&pattern).map_err(|e| anyhow::anyhow!("Invalid regex: {}", e))?;
        if let Some(m) = re.find(content) {
            // From the start of match, find the opening brace of the function body
            let start_idx = m.start();
            let mut i = m.end();
            let bytes = content.as_bytes();
            let len = bytes.len();

            // Scan to first '{' that's not inside parens/angles/strings/comments
            let mut paren_depth = 0i32; // ()
            let mut angle_depth = 0i32; // <>
            let mut in_string = false;
            let mut in_char = false;
            let mut in_line_comment = false;
            let mut in_block_comment = false;
            let mut prev = 0u8; // track previous byte for '->' detection
            let mut body_start = None;

            while i < len {
                let c = bytes[i];
                let next = if i + 1 < len { bytes[i + 1] } else { 0 };

                if in_line_comment {
                    if c == b'\n' {
                        in_line_comment = false;
                    }
                    i += 1;
                    continue;
                }
                if in_block_comment {
                    if c == b'*' && next == b'/' {
                        in_block_comment = false;
                        i += 2;
                        continue;
                    }
                    i += 1;
                    continue;
                }
                if in_string {
                    if c == b'\\' {
                        i += 2;
                        continue;
                    }
                    if c == b'"' {
                        in_string = false;
                    }
                    i += 1;
                    continue;
                }
                if in_char {
                    if c == b'\\' {
                        i += 2;
                        continue;
                    }
                    if c == b'\'' {
                        in_char = false;
                    }
                    i += 1;
                    continue;
                }

                // Enter comments
                if c == b'/' && next == b'/' {
                    in_line_comment = true;
                    i += 2;
                    continue;
                }
                if c == b'/' && next == b'*' {
                    in_block_comment = true;
                    i += 2;
                    continue;
                }

                // Enter strings/chars
                if c == b'"' {
                    in_string = true;
                    i += 1;
                    continue;
                }
                if c == b'\'' {
                    in_char = true;
                    i += 1;
                    continue;
                }

                // Track parentheses and angle brackets in signature
                if c == b'(' {
                    paren_depth += 1;
                    i += 1;
                    continue;
                }
                if c == b')' {
                    paren_depth -= 1;
                    i += 1;
                    continue;
                }
                if c == b'<' {
                    angle_depth += 1;
                    i += 1;
                    continue;
                }
                if c == b'>' {
                    // Don't treat '->' (return type arrow) as a generic angle bracket
                    if prev == b'-' {
                        i += 1;
                        continue;
                    }
                    if angle_depth > 0 {
                        angle_depth -= 1;
                    }
                    i += 1;
                    continue;
                }

                // Semicolon before body means trait method declaration - skip
                if c == b';' && paren_depth == 0 && angle_depth == 0 {
                    return Ok(None);
                }

                if c == b'{' && paren_depth == 0 && angle_depth == 0 {
                    body_start = Some(i);
                    break;
                }
                i += 1;
                prev = c;
            }

            let body_start = if let Some(pos) = body_start {
                pos
            } else {
                return Ok(None);
            };

            // Now match braces to find end of function body
            let mut depth = 0i32;
            let mut j = body_start;
            let mut in_string = false;
            let mut in_char = false;
            let mut in_line_comment = false;
            let mut in_block_comment = false;
            while j < len {
                let c = bytes[j];
                let next = if j + 1 < len { bytes[j + 1] } else { 0 };

                if in_line_comment {
                    if c == b'\n' {
                        in_line_comment = false;
                    }
                    j += 1;
                    continue;
                }
                if in_block_comment {
                    if c == b'*' && next == b'/' {
                        in_block_comment = false;
                        j += 2;
                        continue;
                    }
                    j += 1;
                    continue;
                }
                if in_string {
                    if c == b'\\' {
                        j += 2;
                        continue;
                    }
                    if c == b'"' {
                        in_string = false;
                    }
                    j += 1;
                    continue;
                }
                if in_char {
                    if c == b'\\' {
                        j += 2;
                        continue;
                    }
                    if c == b'\'' {
                        in_char = false;
                    }
                    j += 1;
                    continue;
                }

                if c == b'/' && next == b'/' {
                    in_line_comment = true;
                    j += 2;
                    continue;
                }
                if c == b'/' && next == b'*' {
                    in_block_comment = true;
                    j += 2;
                    continue;
                }
                if c == b'"' {
                    in_string = true;
                    j += 1;
                    continue;
                }
                if c == b'\'' {
                    in_char = true;
                    j += 1;
                    continue;
                }

                if c == b'{' {
                    depth += 1;
                }
                if c == b'}' {
                    depth -= 1;
                    if depth == 0 {
                        // end of the function body
                        let end_idx = j + 1; // include the closing brace
                                             // Compute line boundaries to delete whole lines
                        let before = &content[..start_idx];
                        let func_block = &content[start_idx..end_idx];
                        let start_line = before.lines().count() + 1;
                        let end_line = start_line + func_block.lines().count() - 1;

                        // Optionally include contiguous attributes/doc comments above
                        let lines: Vec<&str> = content.lines().collect();
                        let mut adj_start_line = start_line;
                        while adj_start_line > 1 {
                            let prev_line = lines[adj_start_line - 2].trim_start();
                            if prev_line.starts_with("#[")
                                || prev_line.starts_with("///")
                                || prev_line.starts_with("//!")
                            {
                                adj_start_line -= 1;
                                continue;
                            }
                            break;
                        }

                        // Remove those lines and rebuild content
                        let mut new_lines: Vec<&str> = lines.clone();
                        new_lines.drain(adj_start_line - 1..end_line);
                        let new_content = new_lines.join("\n");
                        return Ok(Some((new_content, adj_start_line, end_line)));
                    }
                }
                j += 1;
            }
            // If we get here, brace matching failed
            Ok(None)
        } else {
            Ok(None)
        }
    }
}

impl Tool for DeleteFunctionTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Delete a function definition by name. Currently supports Rust (.rs). For unsupported languages, suggests using delete_line or delete_text."
    }

    fn signature(&self) -> &str {
        "delete_function(file_name: str, function_name: str)"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let params = self.parse_params(args)?;
        let obj = params.as_object().ok_or_else(|| ToolError::InvalidArgs {
            message: "Parameters must be an object".to_string(),
        })?;
        if !obj.contains_key("file_name") && !obj.contains_key("path") {
            return Err(ToolError::InvalidArgs {
                message: "Missing required parameter: file_name".to_string(),
            });
        }
        if !obj.contains_key("function_name") {
            return Err(ToolError::InvalidArgs {
                message: "Missing required parameter: function_name".to_string(),
            });
        }
        Ok(())
    }

    fn execute(&mut self, args: &ToolArgs, state: &Arc<Mutex<ToolState>>) -> Result<ToolResult> {
        let params = self.parse_params(args)?;
        let obj = params
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Invalid parameters"))?;

        let file_name = obj
            .get("file_name")
            .and_then(|v| v.as_str())
            .or_else(|| obj.get("path").and_then(|v| v.as_str()))
            .ok_or_else(|| anyhow::anyhow!("Invalid file_name parameter"))?;
        let function_name = obj
            .get("function_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid function_name parameter"))?;

        let path = PathBuf::from(file_name);
        if !path.exists() {
            return Ok(ToolResult::error(format!(
                "File not found: {}",
                path.display()
            )));
        }

        let lang = self.guess_language(&path);
        if lang != Some("rust") {
            return Ok(ToolResult::error_with_data(
                format!("delete_function is not implemented yet for this file type. Please use delete_line or delete_text."),
                serde_json::json!({
                    "file_name": path.to_string_lossy(),
                    "function_name": function_name,
                    "language": path.extension().and_then(|s| s.to_str()).unwrap_or("unknown"),
                    "supported_languages": ["rs"],
                    "suggestions": [
                        "Use delete_line with the function's line range",
                        "Use delete_text to remove the function body manually",
                        "Consider adding support for this language in delete_function"
                    ]
                })
            ));
        }

        let content =
            fs::read_to_string(&path).map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?;
        match Self::delete_rust_function(&content, function_name)? {
            Some((new_content, start_line, end_line)) => {
                // Write back to file
                fs::write(
                    &path,
                    if !new_content.is_empty() && !new_content.ends_with('\n') {
                        format!("{}\n", new_content)
                    } else {
                        new_content.clone()
                    },
                )
                .map_err(|e| anyhow::anyhow!("Failed to write file: {}", e))?;
                // Update state
                {
                    let mut state_guard = state
                        .lock()
                        .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;
                    let lines: Vec<String> = new_content.lines().map(|s| s.to_string()).collect();
                    if state_guard.current_file.as_ref() == Some(&path) {
                        state_guard.open_file(path.clone(), lines.clone(), 100)?;
                    }
                    state_guard.push_history(format!(
                        "Deleted function '{}' (lines {}-{}) in: {}",
                        function_name,
                        start_line,
                        end_line,
                        path.display()
                    ));
                }

                Ok(ToolResult::success_with_data(
                    format!(
                        "Successfully deleted function '{}' from {}",
                        function_name,
                        path.display()
                    ),
                    serde_json::json!({
                        "path": path.to_string_lossy(),
                        "function_name": function_name,
                        "start_line": start_line,
                        "end_line": end_line,
                        "lines_deleted": (end_line - start_line + 1)
                    }),
                ))
            }
            None => Ok(ToolResult::error_with_data(
                format!(
                    "Function '{}' not found in {}",
                    function_name,
                    path.display()
                ),
                serde_json::json!({
                    "path": path.to_string_lossy(),
                    "function_name": function_name,
                    "suggestions": [
                        "Check the exact function name (case-sensitive)",
                        "Ensure it's a function with a body (trait methods without bodies are ignored)",
                        "Use search_dir/search_file to locate the function",
                        "As a fallback, use delete_line with the function's line range"
                    ]
                }),
            )),
        }
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_name": {"type": "string", "description": "Path to the source file (currently only .rs supported)"},
                "function_name": {"type": "string", "description": "Name of the function to delete"}
            },
            "required": ["file_name", "function_name"]
        })
    }
}

/// Tool for overwriting entire files
pub struct OverwriteFileTool {
    name: String,
}

impl OverwriteFileTool {
    pub fn new() -> Self {
        Self {
            name: "overwrite_file".to_string(),
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
                "path".to_string(),
                serde_json::Value::String(args.get_arg(0).unwrap().clone()),
            );
            params.insert(
                "content".to_string(),
                serde_json::Value::String(args.get_arg(1).unwrap().clone()),
            );
            return Ok(serde_json::Value::Object(params));
        }

        Err(ToolError::InvalidArgs {
            message: "Insufficient parameters".to_string(),
        })
    }
}

impl Tool for OverwriteFileTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Overwrite entire file with new content"
    }

    fn signature(&self) -> &str {
        "overwrite_file(path: str, content: str)"
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

        if !obj.contains_key("content") {
            return Err(ToolError::InvalidArgs {
                message: "Missing required parameter: content".to_string(),
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

        let content = obj
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid content parameter"))?;

        let path = PathBuf::from(path_str);

        // Check if file exists
        if !path.exists() {
            return Ok(ToolResult::error(format!(
                "File not found: {}",
                path.display()
            )));
        }

        // Get original content length for reporting
        let original_content =
            fs::read_to_string(&path).map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?;
        let original_lines = original_content.lines().count();

        // Write new content
        fs::write(&path, content).map_err(|e| anyhow::anyhow!("Failed to write file: {}", e))?;

        // Update state
        {
            let mut state_guard = state
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;
            if state_guard.current_file.as_ref() == Some(&path) {
                let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                state_guard.open_file(path.clone(), lines, 100)?;
            }
            state_guard.push_history(format!("Overwritten file: {}", path.display()));
        }

        let new_lines = content.lines().count();

        Ok(ToolResult::success_with_data(
            format!("Successfully overwritten file: {}", path.display()),
            serde_json::json!({
                "path": path.to_string_lossy(),
                "content_length": content.len(),
                "lines_original": original_lines,
                "lines_new": new_lines,
                "lines_changed": new_lines as i64 - original_lines as i64,
                "overwritten": true
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Full path to the file to overwrite"
                },
                "content": {
                    "type": "string",
                    "description": "New content for the file"
                }
            },
            "required": ["path", "content"]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_file_tool() {
        let temp_dir = TempDir::new().unwrap();

        let mut tool = CreateFileTool::new();
        let state = Arc::new(Mutex::new(ToolState::new()));

        // Use absolute path
        let test_file = temp_dir.path().join("test_file.txt");

        let args = ToolArgs::with_named_args(
            vec![],
            vec![
                ("path".to_string(), test_file.to_string_lossy().to_string()),
                ("content".to_string(), "Hello, World!".to_string()),
            ]
            .into_iter()
            .collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);

        // Verify file was created
        assert!(test_file.exists());

        let content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[test]
    fn test_replace_text_tool() {
        let temp_dir = TempDir::new().unwrap();

        // Create test file with absolute path
        let test_file = temp_dir.path().join("test_replace.txt");
        fs::write(&test_file, "Hello, World!\nThis is a test.").unwrap();

        let mut tool = ReplaceTextTool::new();
        let state = Arc::new(Mutex::new(ToolState::new()));

        let args = ToolArgs::with_named_args(
            vec![],
            vec![
                ("path".to_string(), test_file.to_string_lossy().to_string()),
                ("old_text".to_string(), "World".to_string()),
                ("new_text".to_string(), "Rust".to_string()),
            ]
            .into_iter()
            .collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);

        // Verify content was replaced
        let content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "Hello, Rust!\nThis is a test.");
    }

    #[test]
    fn test_insert_text_tool() {
        let temp_dir = TempDir::new().unwrap();

        // Create test file with absolute path
        let test_file = temp_dir.path().join("test_insert.txt");
        fs::write(&test_file, "Line 1\nLine 3").unwrap();

        let mut tool = InsertTextTool::new();
        let state = Arc::new(Mutex::new(ToolState::new()));

        let args = ToolArgs::with_named_args(
            vec![],
            vec![
                ("path".to_string(), test_file.to_string_lossy().to_string()),
                ("line_number".to_string(), "2".to_string()),
                ("text".to_string(), "Line 2".to_string()),
                ("position".to_string(), "before_line".to_string()),
            ]
            .into_iter()
            .collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);

        // Verify content was inserted
        let content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "Line 1\nLine 2\nLine 3\n");
    }

    #[test]
    fn test_delete_text_tool() {
        let temp_dir = TempDir::new().unwrap();

        // Create test file with absolute path
        let test_file = temp_dir.path().join("test_delete.txt");
        fs::write(&test_file, "Hello, World!\nThis is a test.").unwrap();

        let mut tool = DeleteTextTool::new();
        let state = Arc::new(Mutex::new(ToolState::new()));

        let args = ToolArgs::with_named_args(
            vec![],
            vec![
                ("path".to_string(), test_file.to_string_lossy().to_string()),
                ("text_to_delete".to_string(), ", World".to_string()),
            ]
            .into_iter()
            .collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);

        // Verify content was deleted
        let content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "Hello!\nThis is a test.");
    }

    #[test]
    fn test_overwrite_file_tool() {
        let temp_dir = TempDir::new().unwrap();

        // Create test file with absolute path
        let test_file = temp_dir.path().join("test_overwrite.txt");
        fs::write(&test_file, "Original content").unwrap();

        let mut tool = OverwriteFileTool::new();
        let state = Arc::new(Mutex::new(ToolState::new()));

        let args = ToolArgs::with_named_args(
            vec![],
            vec![
                ("path".to_string(), test_file.to_string_lossy().to_string()),
                (
                    "content".to_string(),
                    "New content\nWith multiple lines".to_string(),
                ),
            ]
            .into_iter()
            .collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);

        // Verify content was overwritten
        let content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "New content\nWith multiple lines");
    }

    #[test]
    fn test_delete_line_tool_basic_range() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test_delete_line.txt");
        fs::write(&test_file, "a\nb\nc\nd\ne\n").unwrap();

        let mut tool = DeleteLineTool::new();
        let state = Arc::new(Mutex::new(ToolState::new()));

        let args = ToolArgs::with_named_args(
            vec![],
            vec![
                ("path".to_string(), test_file.to_string_lossy().to_string()),
                ("start_line".to_string(), "2".to_string()),
                ("end_line".to_string(), "4".to_string()),
            ]
            .into_iter()
            .collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success, "{}", result.message);

        let content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "a\ne\n");
    }

    #[test]
    fn test_delete_function_tool_rust() {
        let temp_dir = TempDir::new().unwrap();

        let test_file = temp_dir.path().join("test_mod.rs");
        let content = r#"// Simple Rust module
pub fn keep() {}

pub fn target(a: i32) -> i32 {
    let x = a + 1;
    x
}

pub fn keep2() {
    println!("ok");
}
"#;
        fs::write(&test_file, content).unwrap();

        let mut tool = DeleteFunctionTool::new();
        let state = Arc::new(Mutex::new(ToolState::new()));

        let args = ToolArgs::with_named_args(
            vec![],
            vec![
                (
                    "file_name".to_string(),
                    test_file.to_string_lossy().to_string(),
                ),
                ("function_name".to_string(), "target".to_string()),
            ]
            .into_iter()
            .collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success, "{}", result.message);

        let updated = fs::read_to_string(&test_file).unwrap();
        assert!(!updated.contains("fn target("));
        assert!(updated.contains("fn keep()"));
        assert!(updated.contains("fn keep2()"));
    }
}
