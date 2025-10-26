//! Enhanced file editing tools with advanced matching and normalization

pub mod management_tools;
pub mod specialized_tools;

// Re-export new specialized tools
pub use management_tools::{CopyPathTool, CreateDirectoryTool, DeletePathTool, MovePathTool};
pub use specialized_tools::{
    CreateFileTool, DeleteFunctionTool, DeleteLineTool, DeleteTextTool, InsertTextTool,
    OverwriteFileTool, ReplaceTextTool,
};

use crate::core::{Tool, ToolArgs, ToolError, ToolResult};
use crate::state::ToolState;
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Edit operation modes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EditMode {
    Replace,
    Insert,
    Delete,
    Create,
    Overwrite,
}

/// Text normalization options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizationOptions {
    pub normalize_eol: bool,
    pub trim_lines: bool,
    pub normalize_whitespace: bool,
    pub ignore_case: bool,
}

impl Default for NormalizationOptions {
    fn default() -> Self {
        Self {
            normalize_eol: true,
            trim_lines: false,
            normalize_whitespace: false,
            ignore_case: false,
        }
    }
}

/// Matching behavior options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingOptions {
    pub regex: bool,
    pub fuzzy: bool,
    pub fuzzy_threshold: f64,
    pub context_lines: usize,
    pub max_matches: usize,
}

impl Default for MatchingOptions {
    fn default() -> Self {
        Self {
            regex: false,
            fuzzy: true,
            fuzzy_threshold: 0.8,
            context_lines: 3,
            max_matches: 10,
        }
    }
}

/// Information about a found match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchInfo {
    pub index: usize,
    pub line_start: usize,
    pub line_end: usize,
    pub char_start: usize,
    pub char_end: usize,
    pub matched_text: String,
    pub context_before: String,
    pub context_after: String,
    pub similarity_score: f64,
    pub preview: String,
}

/// Location where edit was applied
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditLocation {
    pub line_start: usize,
    pub line_end: usize,
    pub char_start: usize,
    pub char_end: usize,
    pub original_text: String,
    pub new_text: String,
}

/// Enhanced edit tool
pub struct EditTool {
    name: String,
}

impl EditTool {
    pub fn new() -> Self {
        Self {
            name: "edit".to_string(),
        }
    }

    /// Normalize text according to options
    fn normalize_text(&self, text: &str, options: &NormalizationOptions) -> String {
        let mut result = text.to_string();

        if options.normalize_eol {
            result = result.replace("\r\n", "\n").replace("\r", "\n");
        }

        if options.trim_lines {
            result = result
                .lines()
                .map(|line| line.trim_end())
                .collect::<Vec<_>>()
                .join("\n");
        }

        if options.normalize_whitespace {
            result = Regex::new(r"\s+")
                .unwrap()
                .replace_all(&result, " ")
                .to_string();
        }

        if options.ignore_case {
            result = result.to_lowercase();
        }

        result
    }

    /// Calculate similarity between two strings
    fn calculate_similarity(&self, text1: &str, text2: &str) -> f64 {
        if text1 == text2 {
            return 1.0;
        }

        let len1 = text1.chars().count();
        let len2 = text2.chars().count();
        let max_len = len1.max(len2);

        if max_len == 0 {
            return 1.0;
        }

        // Simple Levenshtein distance approximation
        let distance = edit_distance::edit_distance(text1, text2);
        1.0 - (distance as f64 / max_len as f64)
    }

    /// Find all matches for the pattern in content
    fn find_matches(
        &self,
        content: &str,
        pattern: &str,
        options: &MatchingOptions,
        norm_options: &NormalizationOptions,
    ) -> Result<Vec<MatchInfo>, ToolError> {
        let normalized_content = self.normalize_text(content, norm_options);
        let normalized_pattern = self.normalize_text(pattern, norm_options);

        let mut matches = Vec::new();

        if options.regex {
            // Regex matching
            let regex = if norm_options.ignore_case {
                Regex::new(&format!("(?i){}", normalized_pattern))
            } else {
                Regex::new(&normalized_pattern)
            }
            .map_err(|e| ToolError::Regex(e))?;

            for (i, m) in regex
                .find_iter(&normalized_content)
                .enumerate()
                .take(options.max_matches)
            {
                let match_info = self.create_match_info(
                    content,
                    m.start(),
                    m.end(),
                    i + 1,
                    options.context_lines,
                    pattern,
                    "",
                );
                matches.push(match_info);
            }
        } else {
            // Literal matching
            let mut start = 0;
            let mut index = 1;

            while let Some(pos) = normalized_content[start..].find(&normalized_pattern) {
                let actual_pos = start + pos;
                let end_pos = actual_pos + normalized_pattern.len();

                let match_info = self.create_match_info(
                    content,
                    actual_pos,
                    end_pos,
                    index,
                    options.context_lines,
                    pattern,
                    "",
                );
                matches.push(match_info);

                start = end_pos;
                index += 1;

                if matches.len() >= options.max_matches {
                    break;
                }
            }
        }

        // If no exact matches and fuzzy matching is enabled, try fuzzy matching
        if matches.is_empty() && options.fuzzy {
            matches = self.find_fuzzy_matches(content, pattern, options, norm_options)?;
        }

        Ok(matches)
    }

    /// Find fuzzy matches when exact matching fails
    fn find_fuzzy_matches(
        &self,
        content: &str,
        pattern: &str,
        options: &MatchingOptions,
        norm_options: &NormalizationOptions,
    ) -> Result<Vec<MatchInfo>, ToolError> {
        let lines: Vec<&str> = content.lines().collect();
        let normalized_pattern = self.normalize_text(pattern, norm_options);
        let mut matches = Vec::new();

        for (line_num, line) in lines.iter().enumerate() {
            let normalized_line = self.normalize_text(line, norm_options);
            let similarity = self.calculate_similarity(&normalized_line, &normalized_pattern);

            if similarity >= options.fuzzy_threshold {
                let char_start = lines[..line_num].iter().map(|l| l.len() + 1).sum::<usize>();
                let char_end = char_start + line.len();

                let match_info = MatchInfo {
                    index: matches.len() + 1,
                    line_start: line_num + 1,
                    line_end: line_num + 1,
                    char_start,
                    char_end,
                    matched_text: line.to_string(),
                    context_before: self.get_context_before(
                        lines.as_slice(),
                        line_num,
                        options.context_lines,
                    ),
                    context_after: self.get_context_after(
                        lines.as_slice(),
                        line_num,
                        options.context_lines,
                    ),
                    similarity_score: similarity,
                    preview: format!("{} -> (fuzzy match)", line),
                };
                matches.push(match_info);

                if matches.len() >= options.max_matches {
                    break;
                }
            }
        }

        Ok(matches)
    }

    /// Create match info for a found match
    fn create_match_info(
        &self,
        content: &str,
        start: usize,
        end: usize,
        index: usize,
        context_lines: usize,
        _original_pattern: &str,
        new_text: &str,
    ) -> MatchInfo {
        let matched_text = content[start..end].to_string();
        let lines: Vec<&str> = content.lines().collect();

        // Find line number
        let line_start = content[..start].lines().count();
        let line_end = content[..end].lines().count();

        MatchInfo {
            index,
            line_start: line_start + 1,
            line_end: line_end + 1,
            char_start: start,
            char_end: end,
            matched_text: matched_text.clone(),
            context_before: self.get_context_before(lines.as_slice(), line_start, context_lines),
            context_after: self.get_context_after(lines.as_slice(), line_end, context_lines),
            similarity_score: 1.0,
            preview: if new_text.is_empty() {
                format!("{} -> {}", matched_text, new_text)
            } else {
                format!("{} -> {}", matched_text, new_text)
            },
        }
    }

    /// Get context lines before the match
    fn get_context_before(&self, lines: &[&str], line_num: usize, context_lines: usize) -> String {
        let start = line_num.saturating_sub(context_lines);
        lines[start..line_num].join("\n")
    }

    /// Get context lines after the match
    fn get_context_after(&self, lines: &[&str], line_num: usize, context_lines: usize) -> String {
        let end = (line_num + 1 + context_lines).min(lines.len());
        lines[(line_num + 1)..end].join("\n")
    }

    /// Apply edit to content
    fn apply_edit(&self, content: &str, match_info: &MatchInfo, new_text: &str) -> String {
        let mut result = String::new();
        result.push_str(&content[..match_info.char_start]);
        result.push_str(new_text);
        result.push_str(&content[match_info.char_end..]);
        result
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
                "old_text".to_string(),
                serde_json::Value::String(args.get_arg(1).unwrap().clone()),
            );

            if args.len() >= 3 {
                params.insert(
                    "new_text".to_string(),
                    serde_json::Value::String(args.get_arg(2).unwrap().clone()),
                );
            }

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
            message: "Invalid arguments. Use JSON format or provide path and old_text".to_string(),
        })
    }
}

impl Tool for EditTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Enhanced file editing with advanced search and replace, multiple match handling, fuzzy matching, and text normalization options"
    }

    fn signature(&self) -> &str {
        "edit {\"path\": \"file.txt\", \"old_text\": \"search\", \"new_text\": \"replace\", ...}"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let params = self.parse_params(args)?;

        // Validate required fields
        let obj = params.as_object().ok_or_else(|| ToolError::InvalidArgs {
            message: "Parameters must be an object".to_string(),
        })?;

        if !obj.contains_key("path") {
            return Err(ToolError::InvalidArgs {
                message: "Missing required parameter: path".to_string(),
            });
        }

        if !obj.contains_key("old_text")
            && obj.get("mode").and_then(|v| v.as_str()) != Some("create")
            && obj.get("mode").and_then(|v| v.as_str()) != Some("insert")
            && obj.get("mode").and_then(|v| v.as_str()) != Some("overwrite")
        {
            return Err(ToolError::InvalidArgs {
                message: "Missing required parameter: old_text".to_string(),
            });
        }

        Ok(())
    }

    fn execute(&mut self, args: &ToolArgs, state: &Arc<Mutex<ToolState>>) -> Result<ToolResult> {
        let params = self.parse_params(args)?;
        let obj = params.as_object().unwrap();

        // Extract parameters with defaults
        let path =
            obj.get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidArgs {
                    message: "Invalid path".to_string(),
                })?;
        let path_buf = PathBuf::from(path);

        let mode = obj
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("replace");
        let old_text = obj.get("old_text").and_then(|v| v.as_str()).unwrap_or("");
        let new_text = obj.get("new_text").and_then(|v| v.as_str()).unwrap_or("");
        let occurrence = obj
            .get("occurrence")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        let preview = obj
            .get("preview")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Parse normalization options
        let norm_options =
            if let Some(norm_obj) = obj.get("normalization").and_then(|v| v.as_object()) {
                NormalizationOptions {
                    normalize_eol: norm_obj
                        .get("normalize_eol")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true),
                    trim_lines: norm_obj
                        .get("trim_lines")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    normalize_whitespace: norm_obj
                        .get("normalize_whitespace")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    ignore_case: norm_obj
                        .get("ignore_case")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                }
            } else {
                NormalizationOptions::default()
            };

        // Parse matching options
        let match_options = if let Some(match_obj) = obj.get("matching").and_then(|v| v.as_object())
        {
            MatchingOptions {
                regex: match_obj
                    .get("regex")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                fuzzy: match_obj
                    .get("fuzzy")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                fuzzy_threshold: match_obj
                    .get("fuzzy_threshold")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.8),
                context_lines: match_obj
                    .get("context_lines")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize)
                    .unwrap_or(3),
                max_matches: match_obj
                    .get("max_matches")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize)
                    .unwrap_or(10),
            }
        } else {
            MatchingOptions::default()
        };

        // Handle different modes
        match mode {
            "create" => self.handle_create_mode(&path_buf, new_text, preview, state),
            "overwrite" => self.handle_overwrite_mode(&path_buf, new_text, preview, state),
            "insert" => {
                let line_number = obj
                    .get("line_number")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize)
                    .ok_or_else(|| ToolError::InvalidArgs {
                        message: "line_number required for insert mode".to_string(),
                    })?;
                self.handle_insert_mode(&path_buf, line_number, new_text, preview, state)
            }
            "delete" | "replace" => self.handle_replace_mode(
                &path_buf,
                old_text,
                new_text,
                occurrence,
                preview,
                &match_options,
                &norm_options,
                state,
            ),
            _ => Err(ToolError::InvalidArgs {
                message: format!("Unknown mode: {}", mode),
            }
            .into()),
        }
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to edit (absolute or relative to workspace root)"
                },
                "old_text": {
                    "type": "string",
                    "description": "Text pattern to search for. Will be treated as literal text unless regex=true"
                },
                "new_text": {
                    "type": "string",
                    "description": "Replacement text. Can be empty string to delete the matched text"
                },
                "mode": {
                    "type": "string",
                    "enum": ["replace", "insert", "delete", "create", "overwrite"],
                    "default": "replace",
                    "description": "Edit operation mode"
                },
                "occurrence": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Which occurrence of the pattern to replace (1-based). If not specified and multiple matches exist, returns match list"
                },
                "line_number": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "For insert mode: line number to insert after. For other modes: hint to prefer matches near this line"
                },
                "normalization": {
                    "type": "object",
                    "properties": {
                        "normalize_eol": {"type": "boolean", "default": true, "description": "Convert CRLF to LF before matching"},
                        "trim_lines": {"type": "boolean", "default": false, "description": "Remove trailing whitespace from each line before matching"},
                        "normalize_whitespace": {"type": "boolean", "default": false, "description": "Collapse sequences of whitespace to single space"},
                        "ignore_case": {"type": "boolean", "default": false, "description": "Perform case-insensitive matching"}
                    },
                    "description": "Text normalization options"
                },
                "matching": {
                    "type": "object",
                    "properties": {
                        "regex": {"type": "boolean", "default": false, "description": "Treat old_text as a regular expression pattern"},
                        "fuzzy": {"type": "boolean", "default": true, "description": "Enable fuzzy matching if exact match fails"},
                        "fuzzy_threshold": {"type": "number", "minimum": 0.0, "maximum": 1.0, "default": 0.8, "description": "Minimum similarity score for fuzzy matches"},
                        "context_lines": {"type": "integer", "minimum": 0, "default": 3, "description": "Number of context lines to show around matches"},
                        "max_matches": {"type": "integer", "minimum": 1, "default": 10, "description": "Maximum number of matches to return"}
                    },
                    "description": "Matching behavior options"
                },
                "preview": {
                    "type": "boolean",
                    "default": false,
                    "description": "Return a preview of changes without applying them"
                },
                "create_parents": {
                    "type": "boolean",
                    "default": false,
                    "description": "Create parent directories if they don't exist (for create/overwrite modes)"
                }
            },
            "required": ["path"]
        })
    }
}

impl EditTool {
    /// Handle create mode - create new file
    fn handle_create_mode(
        &self,
        path: &PathBuf,
        content: &str,
        preview: bool,
        state: &Arc<Mutex<ToolState>>,
    ) -> Result<ToolResult> {
        if path.exists() {
            return Ok(ToolResult::error(format!(
                "File already exists: {}",
                path.display()
            )));
        }

        if preview {
            return Ok(ToolResult::success_with_data(
                format!(
                    "Preview: Would create file {} with {} characters",
                    path.display(),
                    content.len()
                ),
                serde_json::json!({
                    "path": path.to_string_lossy(),
                    "mode": "create",
                    "preview": true,
                    "content_length": content.len()
                }),
            ));
        }

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| anyhow::anyhow!("Failed to create parent directories: {}", e))?;
        }

        fs::write(path, content).map_err(|e| anyhow::anyhow!("Failed to create file: {}", e))?;

        // Update state
        {
            let mut state_guard = state
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;
            state_guard.push_history(format!("Created file: {}", path.display()));
        }

        Ok(ToolResult::success_with_data(
            format!("Successfully created file {}", path.display()),
            serde_json::json!({
                "path": path.to_string_lossy(),
                "mode": "create",
                "content_length": content.len(),
                "lines_created": content.lines().count()
            }),
        ))
    }

    /// Handle replace/delete mode - search and replace text
    fn handle_replace_mode(
        &self,
        path: &PathBuf,
        old_text: &str,
        new_text: &str,
        occurrence: Option<usize>,
        preview: bool,
        match_options: &MatchingOptions,
        norm_options: &NormalizationOptions,
        state: &Arc<Mutex<ToolState>>,
    ) -> Result<ToolResult> {
        if !path.exists() {
            return Ok(ToolResult::error(format!(
                "File not found: {}",
                path.display()
            )));
        }

        let content =
            fs::read_to_string(path).map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?;

        let matches = self.find_matches(&content, old_text, match_options, norm_options)?;

        if matches.is_empty() {
            return Ok(ToolResult::error_with_data(
                format!("No matches found for '{}' in {}", old_text, path.display()),
                serde_json::json!({
                    "error_type": "no_matches",
                    "path": path.to_string_lossy(),
                    "searched_pattern": old_text,
                    "suggestions": self.generate_no_match_suggestions(&content, old_text, norm_options)
                }),
            ));
        }

        // If multiple matches and no occurrence specified, return match list
        if matches.len() > 1 && occurrence.is_none() {
            return Ok(ToolResult::error_with_data(
                format!(
                    "Found {} occurrences of '{}' in {}",
                    matches.len(),
                    old_text,
                    path.display()
                ),
                serde_json::json!({
                    "error_type": "multiple_matches",
                    "total_matches": matches.len(),
                    "showing_matches": matches.len(),
                    "matches": matches,
                    "suggestions": vec![
                        format!("Use 'occurrence: N' parameter to select specific match (1-{})", matches.len()),
                        "Provide more context in old_text to make the match unique".to_string(),
                        "Use file navigation tools to view matches: 'open' and 'goto <line_number>'".to_string()
                    ]
                }),
            ));
        }

        // Select the specific occurrence
        let selected_match = if let Some(occ) = occurrence {
            if occ == 0 || occ > matches.len() {
                return Ok(ToolResult::error(format!(
                    "Invalid occurrence {}. Found {} matches",
                    occ,
                    matches.len()
                )));
            }
            &matches[occ - 1]
        } else {
            &matches[0] // Single match
        };

        if preview {
            return Ok(ToolResult::success_with_data(
                format!(
                    "Preview: Would replace occurrence {} at line {}",
                    selected_match.index, selected_match.line_start
                ),
                serde_json::json!({
                    "path": path.to_string_lossy(),
                    "mode": "replace",
                    "preview": true,
                    "match_info": selected_match,
                    "change_preview": selected_match.preview
                }),
            ));
        }

        // Apply the edit
        let new_content = self.apply_edit(&content, selected_match, new_text);

        fs::write(path, &new_content)
            .map_err(|e| anyhow::anyhow!("Failed to write file: {}", e))?;

        // Update state
        {
            let mut state_guard = state
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;
            if state_guard.current_file.as_ref() == Some(path) {
                let lines: Vec<String> = new_content.lines().map(|s| s.to_string()).collect();
                state_guard.open_file(path.clone(), lines, 100)?;
            }
            state_guard.push_history(format!(
                "Edited file: {} (replaced occurrence {})",
                path.display(),
                selected_match.index
            ));
        }

        let chars_changed = new_text.len() as i64 - selected_match.matched_text.len() as i64;

        Ok(ToolResult::success_with_data(
            format!(
                "Successfully replaced occurrence {} in {}",
                selected_match.index,
                path.display()
            ),
            serde_json::json!({
                "path": path.to_string_lossy(),
                "mode": "replace",
                "lines_changed": (selected_match.line_end - selected_match.line_start + 1),
                "characters_changed": chars_changed,
                "applied_at": [EditLocation {
                    line_start: selected_match.line_start,
                    line_end: selected_match.line_end,
                    char_start: selected_match.char_start,
                    char_end: selected_match.char_end,
                    original_text: selected_match.matched_text.clone(),
                    new_text: new_text.to_string(),
                }]
            }),
        ))
    }

    /// Generate suggestions when no matches are found
    fn generate_no_match_suggestions(
        &self,
        content: &str,
        pattern: &str,
        norm_options: &NormalizationOptions,
    ) -> Vec<serde_json::Value> {
        let mut suggestions = Vec::new();

        // Try case-insensitive search
        if !norm_options.ignore_case && content.to_lowercase().contains(&pattern.to_lowercase()) {
            suggestions.push(serde_json::json!({
                "type": "case_difference",
                "description": "Pattern found with different case",
                "suggestion": "Enable case-insensitive matching with 'ignore_case: true'"
            }));
        }

        // Try EOL normalization
        let normalized_content = content.replace("\r\n", "\n").replace("\r", "\n");
        let normalized_pattern = pattern.replace("\r\n", "\n").replace("\r", "\n");
        if normalized_content.contains(&normalized_pattern) {
            suggestions.push(serde_json::json!({
                "type": "eol_difference",
                "description": "Pattern found with different line endings",
                "suggestion": "Line ending differences detected. Try 'normalize_eol: true'"
            }));
        }

        // Try whitespace normalization
        let ws_content = Regex::new(r"\s+").unwrap().replace_all(content, " ");
        let ws_pattern = Regex::new(r"\s+").unwrap().replace_all(pattern, " ");
        if ws_content.to_string().contains(&ws_pattern.to_string()) {
            suggestions.push(serde_json::json!({
                "type": "whitespace_difference",
                "description": "Pattern found with different whitespace",
                "suggestion": "Whitespace differences detected. Try 'normalize_whitespace: true'"
            }));
        }

        suggestions
    }

    /// Handle overwrite mode - overwrite entire file
    fn handle_overwrite_mode(
        &self,
        path: &PathBuf,
        content: &str,
        preview: bool,
        state: &Arc<Mutex<ToolState>>,
    ) -> Result<ToolResult> {
        if preview {
            return Ok(ToolResult::success_with_data(
                format!(
                    "Preview: Would overwrite file {} with {} characters",
                    path.display(),
                    content.len()
                ),
                serde_json::json!({
                    "path": path.to_string_lossy(),
                    "mode": "overwrite",
                    "preview": true,
                    "content_length": content.len()
                }),
            ));
        }

        fs::write(path, content).map_err(|e| anyhow::anyhow!("Failed to overwrite file: {}", e))?;

        // Update state
        {
            let mut state_guard = state
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;
            if state_guard.current_file.as_ref() == Some(path) {
                let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                state_guard.open_file(path.clone(), lines, 100)?;
            }
            state_guard.push_history(format!("Overwritten file: {}", path.display()));
        }

        Ok(ToolResult::success_with_data(
            format!("Successfully overwritten file {}", path.display()),
            serde_json::json!({
                "path": path.to_string_lossy(),
                "mode": "overwrite",
                "content_length": content.len(),
                "lines_written": content.lines().count()
            }),
        ))
    }

    /// Handle insert mode - insert text at specific line
    fn handle_insert_mode(
        &self,
        path: &PathBuf,
        line_number: usize,
        text: &str,
        preview: bool,
        state: &Arc<Mutex<ToolState>>,
    ) -> Result<ToolResult> {
        if !path.exists() {
            return Ok(ToolResult::error(format!(
                "File not found: {}",
                path.display()
            )));
        }

        let content =
            fs::read_to_string(path).map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?;

        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        if line_number == 0 || line_number > lines.len() + 1 {
            return Ok(ToolResult::error(format!(
                "Invalid line number {}. File has {} lines",
                line_number,
                lines.len()
            )));
        }

        if preview {
            return Ok(ToolResult::success_with_data(
                format!(
                    "Preview: Would insert text at line {} in {}",
                    line_number,
                    path.display()
                ),
                serde_json::json!({
                    "path": path.to_string_lossy(),
                    "mode": "insert",
                    "preview": true,
                    "line_number": line_number,
                    "text": text
                }),
            ));
        }

        // Insert at the specified line (after line_number-1)
        lines.insert(line_number, text.to_string());

        let new_content = lines.join("\n");
        fs::write(path, &new_content)
            .map_err(|e| anyhow::anyhow!("Failed to write file: {}", e))?;

        // Update state
        {
            let mut state_guard = state
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;
            if state_guard.current_file.as_ref() == Some(path) {
                state_guard.open_file(path.clone(), lines.clone(), 100)?;
            }
            state_guard.push_history(format!(
                "Inserted text in {} at line {}",
                path.display(),
                line_number
            ));
        }

        Ok(ToolResult::success_with_data(
            format!(
                "Successfully inserted text at line {} in {}",
                line_number,
                path.display()
            ),
            serde_json::json!({
                "path": path.to_string_lossy(),
                "mode": "insert",
                "line_number": line_number,
                "lines_changed": 1,
                "total_lines": lines.len()
            }),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Tool;
    use crate::state::ToolState;
    use std::collections::HashMap;

    #[test]
    fn test_edit_tool_openai_schema() {
        let tool = EditTool::new();
        let schema = tool.get_parameters_schema();

        // Verify no oneOf at top level (this was the OpenAI error)
        assert!(
            schema.get("oneOf").is_none(),
            "Schema must not contain oneOf at top level"
        );

        // Verify it has type: object
        assert_eq!(schema.get("type").and_then(|v| v.as_str()), Some("object"));

        // Verify required parameters
        let required = schema.get("required").and_then(|v| v.as_array()).unwrap();
        assert!(required.contains(&serde_json::Value::String("path".to_string())));

        // Verify properties exist
        let properties = schema
            .get("properties")
            .and_then(|v| v.as_object())
            .unwrap();
        assert!(properties.contains_key("path"));
        assert!(properties.contains_key("old_text"));
        assert!(properties.contains_key("new_text"));
        assert!(properties.contains_key("mode"));

        println!("✅ Edit tool schema is valid for OpenAI function calling!");
    }

    #[test]
    fn test_edit_tool_mode_validation() {
        let tool = EditTool::new();

        // Test create mode validation (should require new_text)
        let mut args = HashMap::new();
        args.insert("path".to_string(), "/tmp/test.txt".to_string());
        args.insert("mode".to_string(), "create".to_string());
        let tool_args = ToolArgs::with_named_args(vec![], args.clone());

        // This should fail validation because new_text is missing for create mode
        // But that's handled in execute, not validate_args
        assert!(tool.validate_args(&tool_args).is_ok());

        // Test insert mode validation (should require new_text and line_number)
        args.insert("mode".to_string(), "insert".to_string());
        let tool_args = ToolArgs::with_named_args(vec![], args);
        assert!(tool.validate_args(&tool_args).is_ok());
    }

    #[test]
    fn test_edit_tool_create_mode() {
        use std::sync::{Arc, Mutex};
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("new_test_file.txt");
        let mut tool = EditTool::new();
        let state = Arc::new(Mutex::new(ToolState::new()));

        // Test create mode with proper parameters
        let mut args = HashMap::new();
        args.insert("path".to_string(), test_file.to_string_lossy().to_string());
        args.insert("mode".to_string(), "create".to_string());
        args.insert(
            "new_text".to_string(),
            "Hello, World!\nThis is a test file.".to_string(),
        );
        let tool_args = ToolArgs::with_named_args(vec![], args);

        let result = tool.execute(&tool_args, &state).unwrap();
        assert!(
            result.success,
            "Create mode should succeed: {}",
            result.message
        );

        // Verify file was created
        assert!(test_file.exists(), "File should be created");
        let content = std::fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "Hello, World!\nThis is a test file.");

        println!("✅ Edit tool create mode works correctly!");
    }
}
