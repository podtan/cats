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
        if args.get_named_arg("file_path").is_none()
            && args.get_named_arg("filePath").is_none()
            && args.args.is_empty()
        {
            return Err(ToolError::InvalidArgs {
                message: "edit tool requires a 'file_path' argument".to_string(),
            });
        }
        if args.get_named_arg("old_string").is_none()
            && args.get_named_arg("oldString").is_none()
            && args.args.len() < 2
        {
            return Err(ToolError::InvalidArgs {
                message: "edit tool requires an 'old_string' argument".to_string(),
            });
        }
        if args.get_named_arg("new_string").is_none()
            && args.get_named_arg("newString").is_none()
            && args.args.len() < 3
        {
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

        if occurrences == 1 || replace_all {
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
        // occurrences > 1 && !replace_all: fall through to find a unique fuzzy match
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

    // Try indentation-flexible matching: strip minimum common indentation before comparing
    for replaced in indentation_flexible_replacer(content, old_string) {
        if let Some(new_content) = try_replace(content, &replaced, new_string, replace_all)? {
            return Ok(new_content);
        }
    }

    // Try block-anchor fuzzy matching: match by first/last line anchors + Levenshtein middle
    if let Some(replaced) = block_anchor_replacer(content, old_string) {
        if let Some(new_content) = try_replace(content, &replaced, new_string, replace_all)? {
            return Ok(new_content);
        }
    }

    // Try escape-normalized matching: unescape literal \\n, \\t, \\r sequences
    // that LLMs sometimes emit due to JSON double-escaping
    let unescaped = unescape_string(old_string);
    if unescaped != old_string {
        if let Ok(new_content) = replace_in_content(content, &unescaped, new_string, replace_all) {
            return Ok(new_content);
        }
    }

    // If the exact string was present more than once but no fuzzy strategy found a unique match,
    // report the ambiguity (consistent with OpenCode's behaviour).
    if !replace_all && content.matches(old_string).count() > 1 {
        return Err(anyhow::anyhow!(
            "Found multiple matches for oldString. Provide more surrounding lines in oldString to identify the correct match."
        ));
    }

    Err(anyhow::anyhow!(
        "oldString not found in content. Use the read tool to get the exact current file content before editing — do not construct oldString from memory."
    ))
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

/// Indentation-flexible replacer: strips minimum common indentation before comparing.
/// Handles cases where LLMs copy code with different indentation (2-space vs 4-space).
fn indentation_flexible_replacer(content: &str, find: &str) -> Vec<String> {
    let mut results = Vec::new();

    let min_indent = |s: &str| -> usize {
        s.lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0)
    };

    let strip_indent = |s: &str, n: usize| -> String {
        s.lines()
            .map(|l| if l.len() >= n { &l[n..] } else { l.trim_start() })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let find_stripped = strip_indent(find, min_indent(find));
    if find_stripped == find {
        // Already at minimum indentation — nothing new to try
        return results;
    }

    let find_lines: Vec<&str> = find_stripped.split('\n').collect();
    if find_lines.is_empty() {
        return results;
    }

    let content_lines: Vec<&str> = content.split('\n').collect();
    if content_lines.len() < find_lines.len() {
        return results;
    }

    for i in 0..=content_lines.len() - find_lines.len() {
        let block: String = content_lines[i..i + find_lines.len()].join("\n");
        let block_stripped = strip_indent(&block, min_indent(&block));
        if block_stripped.replace("\r\n", "\n").trim() == find_stripped.trim() {
            results.push(block);
        }
    }

    results
}

/// Block-anchor replacer: locates blocks matching first/last trimmed lines, then scores
/// middle lines by Levenshtein similarity. Returns the best unique match above threshold.
/// Mirrors OpenCode's `BlockAnchorReplacer`.
fn block_anchor_replacer(content: &str, find: &str) -> Option<String> {
    let find_lines: Vec<&str> = find.split('\n').collect();
    if find_lines.len() < 2 {
        return None;
    }

    let first_anchor = find_lines.first()?.trim();
    let last_anchor = find_lines.last()?.trim();
    let n = find_lines.len();

    let content_lines: Vec<&str> = content.split('\n').collect();
    if content_lines.len() < n {
        return None;
    }

    // Similarity: 1.0 = identical, 0.0 = completely different
    let similarity = |a: &str, b: &str| -> f64 {
        if a.is_empty() && b.is_empty() {
            return 1.0;
        }
        let dist = edit_distance::edit_distance(a, b);
        let max_len = a.len().max(b.len());
        if max_len == 0 { 1.0 } else { 1.0 - dist as f64 / max_len as f64 }
    };

    let middle_score = |block_lines: &[&str]| -> f64 {
        if block_lines.len() <= 2 {
            return 1.0;
        }
        let middle_block = &block_lines[1..block_lines.len() - 1];
        let middle_find = &find_lines[1..find_lines.len() - 1];
        let pairs = middle_block.iter().zip(middle_find.iter());
        let total: f64 = pairs.map(|(b, f)| similarity(b.trim(), f.trim())).sum();
        total / middle_find.len() as f64
    };

    let mut candidates: Vec<(usize, f64)> = Vec::new();

    for i in 0..=content_lines.len() - n {
        let block = &content_lines[i..i + n];
        if block.first()?.trim() != first_anchor {
            continue;
        }
        if block.last()?.trim() != last_anchor {
            continue;
        }
        let score = middle_score(block);
        candidates.push((i, score));
    }

    if candidates.is_empty() {
        return None;
    }

    // Sort by score descending, pick best
    candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let (best_idx, best_score) = candidates[0];

    // Require ≥30% similarity when multiple candidates; any match is ok for a single candidate
    let threshold = if candidates.len() > 1 { 0.3 } else { 0.0 };
    if best_score < threshold {
        return None;
    }

    Some(content_lines[best_idx..best_idx + n].join("\n"))
}

/// Unescape literal escape sequences that LLMs may emit due to JSON double-escaping./// Converts the 2-char sequences \\n, \\t, \\r, \\" into their actual control characters.
fn unescape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.peek() {
                Some('n') => { chars.next(); out.push('\n'); }
                Some('t') => { chars.next(); out.push('\t'); }
                Some('r') => { chars.next(); out.push('\r'); }
                Some('"') => { chars.next(); out.push('"'); }
                Some('\\') => { chars.next(); out.push('\\'); }
                _ => out.push(c),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Line-trimmed replacer: matches ignoring leading/trailing whitespace on each line
fn line_trimmed_replacer<'a>(content: &'a str, find: &'a str) -> Vec<String> {
    let mut results = Vec::new();
    // Use split('\n') instead of lines() to preserve \r on CRLF files;
    // joining with "\n" then reconstructs the exact original substring.
    let original_lines: Vec<&str> = content.split('\n').collect();
    let search_lines: Vec<&str> = find.split('\n').collect();

    if search_lines.is_empty() {
        return results;
    }

    let search_lines: Vec<&str> = if search_lines.last().map(|l| l.trim().is_empty()) == Some(true) {
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
            // .trim() strips both leading/trailing whitespace AND \r
            if original_lines[i + j].trim() != search_line.trim() {
                matches = false;
                break;
            }
        }

        if matches {
            // original_lines[k] retains \r for CRLF files, so joining with \n
            // produces the exact original substring (e.g. "line1\r\nline2\r")
            let matched: String = original_lines[i..i + search_lines.len()].join("\n");
            results.push(matched);
        }
    }

    results
}

/// Whitespace normalized replacer: matches ignoring all whitespace differences
fn whitespace_normalized_replacer<'a>(content: &'a str, find: &'a str) -> Vec<String> {
    let mut results = Vec::new();

    // split_whitespace already treats \r as whitespace, so normalization works on CRLF too
    let normalize = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    let normalized_find = normalize(find);

    // Single line match — use lines() here since single-line \r stripping is fine
    for line in content.lines() {
        if normalize(line) == normalized_find {
            results.push(line.to_string());
        }
    }

    // Multi-line match — use split('\n') to preserve \r so the joined block matches original
    let find_lines: Vec<&str> = find.split('\n').collect();
    if find_lines.len() > 1 {
        let content_lines: Vec<&str> = content.split('\n').collect();

        if content_lines.len() >= find_lines.len() {
            for i in 0..=content_lines.len() - find_lines.len() {
                let block: String = content_lines[i..i + find_lines.len()].join("\n");
                if normalize(&block) == normalized_find {
                    results.push(block);
                }
            }
        }
    }

    results
}

/// Trimmed boundary replacer: matches ignoring leading/trailing whitespace of the whole string
fn trimmed_boundary_replacer<'a>(content: &'a str, find: &'a str) -> Vec<String> {
    let mut results = Vec::new();
    let trimmed_find = find.trim();

    // Try direct trimmed match — also try CRLF variant for CRLF files
    let trimmed_find_crlf = trimmed_find.replace('\n', "\r\n");
    if content.contains(trimmed_find) {
        results.push(trimmed_find.to_string());
    } else if content.contains(trimmed_find_crlf.as_str()) {
        results.push(trimmed_find_crlf);
    }

    // Try block match — use split('\n') to preserve \r so joined block matches original
    let find_lines: Vec<&str> = find.split('\n').collect();
    if find_lines.is_empty() {
        return results;
    }
    let content_lines: Vec<&str> = content.split('\n').collect();

    if content_lines.len() >= find_lines.len() {
        for i in 0..=content_lines.len() - find_lines.len() {
            let block: String = content_lines[i..i + find_lines.len()].join("\n");
            // Normalize CRLF before trimmed comparison so CRLF blocks match LF find strings
            if block.replace("\r\n", "\n").trim() == trimmed_find {
                results.push(block);
            }
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
