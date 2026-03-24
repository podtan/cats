//! Edit tool implementation compatible with OpenCode
//!
//! Performs exact string replacements in files with multiple fallback strategies.

use crate::core::{Tool, ToolArgs, ToolError, ToolResult};
use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Edit tool parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EditParams {
    /// The absolute path to the file to modify
    pub file_path: String,
    /// The text to replace
    pub old_string: String,
    /// The text to replace it with (must be different from oldString)
    pub new_string: String,
    /// Replace all occurrences of oldString (default false)
    pub replace_all: Option<bool>,
}

/// Edit tool for performing string replacements in files
pub struct EditTool {
    name: String,
}

impl EditTool {
    pub fn new() -> Self {
        Self {
            name: "edit".to_string(),
        }
    }
}

impl Default for EditTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for EditTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Performs exact string replacements in files"
    }

    fn signature(&self) -> &str {
        "edit --file-path <path> --old-string <old> --new-string <new> [--replace-all]"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        if args.get_named_arg("file_path").is_none() && args.args.is_empty() {
            return Err(ToolError::InvalidArgs {
                message: "edit tool requires a 'file_path' argument".to_string(),
            });
        }
        if args.get_named_arg("old_string").is_none() && args.args.len() < 2 {
            return Err(ToolError::InvalidArgs {
                message: "edit tool requires an 'old_string' argument".to_string(),
            });
        }
        if args.get_named_arg("new_string").is_none() && args.args.len() < 3 {
            return Err(ToolError::InvalidArgs {
                message: "edit tool requires a 'new_string' argument".to_string(),
            });
        }
        Ok(())
    }

    fn execute(
        &mut self,
        args: &ToolArgs,
        state: &Arc<Mutex<crate::state::ToolState>>,
    ) -> Result<ToolResult> {
        let params = parse_edit_args(args)?;

        if params.old_string == params.new_string {
            return Err(anyhow::anyhow!("oldString and newString must be different").into());
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

        if !filepath.exists() {
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

        // Read file content
        let content = fs::read_to_string(&filepath)?;
        let replace_all = params.replace_all.unwrap_or(false);

        // Perform replacement
        let new_content = replace_in_content(
            &content,
            &params.old_string,
            &params.new_string,
            replace_all,
        )?;

        // Write back
        fs::write(&filepath, &new_content)?;

        // Generate diff summary
        let additions = new_content.lines().count() as i32 - content.lines().count() as i32;
        let changes = if replace_all {
            "multiple locations".to_string()
        } else {
            "1 location".to_string()
        };

        let message = format!(
            "Edited file: {} (changes in {})",
            filepath.display(),
            changes
        );

        Ok(ToolResult::success_with_data(
            message,
            serde_json::json!({
                "file_path": filepath.display().to_string(),
                "old_string": params.old_string,
                "new_string": params.new_string,
                "replace_all": replace_all,
                "additions": additions,
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        let schema = schemars::schema_for!(EditParams);
        serde_json::to_value(schema).unwrap_or_default()
    }
}

fn parse_edit_args(args: &ToolArgs) -> Result<EditParams> {
    let file_path = args
        .get_named_arg("file_path")
        .cloned()
        .or_else(|| args.get_named_arg("filePath").cloned())
        .or_else(|| args.args.first().cloned())
        .ok_or_else(|| anyhow::anyhow!("file_path is required"))?;

    let old_string = args
        .get_named_arg("old_string")
        .cloned()
        .or_else(|| args.get_named_arg("oldString").cloned())
        .or_else(|| args.args.get(1).cloned())
        .ok_or_else(|| anyhow::anyhow!("old_string is required"))?;

    let new_string = args
        .get_named_arg("new_string")
        .cloned()
        .or_else(|| args.get_named_arg("newString").cloned())
        .or_else(|| args.args.get(2).cloned())
        .ok_or_else(|| anyhow::anyhow!("new_string is required"))?;

    let replace_all = args
        .get_named_arg("replace_all")
        .or_else(|| args.get_named_arg("replaceAll"))
        .map(|s| s == "true");

    Ok(EditParams {
        file_path,
        old_string,
        new_string,
        replace_all,
    })
}

/// Replace string in content with multiple fallback strategies
pub fn replace_in_content(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Result<String> {
    // Try simple replacement first
    if content.contains(old_string) {
        let occurrences = content.matches(old_string).count();

        if occurrences > 1 && !replace_all {
            return Err(anyhow::anyhow!(
                "Found multiple matches for oldString. Provide more surrounding lines in oldString to identify the correct match."
            ));
        }

        if replace_all {
            return Ok(content.replace(old_string, new_string));
        }

        // Replace first occurrence
        if let Some(pos) = content.find(old_string) {
            let mut result =
                String::with_capacity(content.len() - old_string.len() + new_string.len());
            result.push_str(&content[..pos]);
            result.push_str(new_string);
            result.push_str(&content[pos + old_string.len()..]);
            return Ok(result);
        }
    }

    // Try line-trimmed matching
    for replaced in line_trimmed_replacer(content, old_string) {
        if let Some(new_content) = try_replace(content, &replaced, new_string, replace_all)? {
            return Ok(new_content);
        }
    }

    // Try whitespace normalized matching
    for replaced in whitespace_normalized_replacer(content, old_string) {
        if let Some(new_content) = try_replace(content, &replaced, new_string, replace_all)? {
            return Ok(new_content);
        }
    }

    // Try trimmed boundary matching
    for replaced in trimmed_boundary_replacer(content, old_string) {
        if let Some(new_content) = try_replace(content, &replaced, new_string, replace_all)? {
            return Ok(new_content);
        }
    }

    Err(anyhow::anyhow!("oldString not found in content"))
}

fn try_replace(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Result<Option<String>> {
    let occurrences = content.matches(old_string).count();

    if occurrences == 0 {
        return Ok(None);
    }

    if occurrences > 1 && !replace_all {
        return Ok(None);
    }

    if replace_all {
        return Ok(Some(content.replace(old_string, new_string)));
    }

    // Replace first occurrence only
    if let Some(pos) = content.find(old_string) {
        let mut result = String::with_capacity(content.len() - old_string.len() + new_string.len());
        result.push_str(&content[..pos]);
        result.push_str(new_string);
        result.push_str(&content[pos + old_string.len()..]);
        return Ok(Some(result));
    }

    Ok(None)
}

/// Line-trimmed replacer: matches ignoring leading/trailing whitespace on each line
fn line_trimmed_replacer<'a>(content: &'a str, find: &'a str) -> Vec<String> {
    let mut results = Vec::new();
    let original_lines: Vec<&str> = content.lines().collect();
    let search_lines: Vec<&str> = find.lines().collect();

    if search_lines.is_empty() {
        return results;
    }

    let search_lines: Vec<&str> = if search_lines.last().map(|l| l.is_empty()) == Some(true) {
        search_lines[..search_lines.len() - 1].to_vec()
    } else {
        search_lines
    };

    if search_lines.is_empty() {
        return results;
    }

    for i in 0..=original_lines.len().saturating_sub(search_lines.len()) {
        let mut matches = true;

        for (j, search_line) in search_lines.iter().enumerate() {
            if original_lines[i + j].trim() != search_line.trim() {
                matches = false;
                break;
            }
        }

        if matches {
            // Extract the original matched content
            let matched: String = original_lines[i..i + search_lines.len()].join("\n");
            results.push(matched);
        }
    }

    results
}

/// Whitespace normalized replacer: matches ignoring all whitespace differences
fn whitespace_normalized_replacer<'a>(content: &'a str, find: &'a str) -> Vec<String> {
    let mut results = Vec::new();

    let normalize = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    let normalized_find = normalize(find);

    // Single line match
    for line in content.lines() {
        if normalize(line) == normalized_find {
            results.push(line.to_string());
        }
    }

    // Multi-line match
    let find_lines: Vec<&str> = find.lines().collect();
    if find_lines.len() > 1 {
        let content_lines: Vec<&str> = content.lines().collect();

        for i in 0..=content_lines.len().saturating_sub(find_lines.len()) {
            let block: String = content_lines[i..i + find_lines.len()].join("\n");
            if normalize(&block) == normalized_find {
                results.push(block);
            }
        }
    }

    results
}

/// Trimmed boundary replacer: matches ignoring leading/trailing whitespace of the whole string
fn trimmed_boundary_replacer<'a>(content: &'a str, find: &'a str) -> Vec<String> {
    let mut results = Vec::new();
    let trimmed_find = find.trim();

    // Try direct trimmed match
    if content.contains(trimmed_find) {
        results.push(trimmed_find.to_string());
    }

    // Try block match where the block's trimmed content matches
    let find_lines: Vec<&str> = find.lines().collect();
    let content_lines: Vec<&str> = content.lines().collect();

    for i in 0..=content_lines.len().saturating_sub(find_lines.len()) {
        let block: String = content_lines[i..i + find_lines.len()].join("\n");
        if block.trim() == trimmed_find {
            results.push(block);
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_edit_tool_creation() {
        let tool = EditTool::new();
        assert_eq!(tool.name(), "edit");
    }

    #[test]
    fn test_edit_tool_simple_replace() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Hello, World!").unwrap();

        let mut tool = EditTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let args = ToolArgs::with_named_args(
            vec![temp_file.path().to_str().unwrap().to_string()],
            vec![
                ("old_string".to_string(), "World".to_string()),
                ("new_string".to_string(), "Rust".to_string()),
            ]
            .into_iter()
            .collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);

        let content = fs::read_to_string(temp_file.path()).unwrap();
        assert_eq!(content.trim(), "Hello, Rust!");
    }

    #[test]
    fn test_edit_tool_replace_all() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "foo bar foo baz foo").unwrap();

        let mut tool = EditTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let args = ToolArgs::with_named_args(
            vec![temp_file.path().to_str().unwrap().to_string()],
            vec![
                ("old_string".to_string(), "foo".to_string()),
                ("new_string".to_string(), "qux".to_string()),
                ("replace_all".to_string(), "true".to_string()),
            ]
            .into_iter()
            .collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);

        let content = fs::read_to_string(temp_file.path()).unwrap();
        assert_eq!(content.trim(), "qux bar qux baz qux");
    }

    #[test]
    fn test_edit_tool_multiple_matches_error() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "foo bar foo").unwrap();

        let mut tool = EditTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let args = ToolArgs::with_named_args(
            vec![temp_file.path().to_str().unwrap().to_string()],
            vec![
                ("old_string".to_string(), "foo".to_string()),
                ("new_string".to_string(), "bar".to_string()),
            ]
            .into_iter()
            .collect(),
        );

        let result = tool.execute(&args, &state);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("multiple matches"));
    }

    #[test]
    fn test_edit_tool_not_found() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Hello, World!").unwrap();

        let mut tool = EditTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let args = ToolArgs::with_named_args(
            vec![temp_file.path().to_str().unwrap().to_string()],
            vec![
                ("old_string".to_string(), "NotPresent".to_string()),
                ("new_string".to_string(), "New".to_string()),
            ]
            .into_iter()
            .collect(),
        );

        let result = tool.execute(&args, &state);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_edit_tool_same_string_error() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Hello, World!").unwrap();

        let mut tool = EditTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let args = ToolArgs::with_named_args(
            vec![temp_file.path().to_str().unwrap().to_string()],
            vec![
                ("old_string".to_string(), "World".to_string()),
                ("new_string".to_string(), "World".to_string()),
            ]
            .into_iter()
            .collect(),
        );

        let result = tool.execute(&args, &state);
        assert!(result.is_err());
    }

    #[test]
    fn test_edit_tool_validation() {
        let tool = EditTool::new();
        let args = ToolArgs::from_args(&[]);

        let result = tool.validate_args(&args);
        assert!(result.is_err());
    }
}
