//! JSON to ToolArgs conversion for LLM function calls
//!
//! Converts JSON arguments from LLM providers (OpenAI, Anthropic, etc.)
//! to the CATS ToolArgs format.

use crate::ToolArgs;
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use tracing::debug;

/// Convert OpenAI/Anthropic function call JSON arguments to ToolArgs format
///
/// # Arguments
/// * `tool_name` - The name of the tool being called
/// * `args` - JSON value containing the function arguments
///
/// # Returns
/// * `Result<ToolArgs>` - Converted tool arguments in CATS format
pub fn json_to_tool_args(tool_name: &str, args: Value) -> Result<ToolArgs> {
    let mut positional_args = Vec::new();
    let mut named_args = HashMap::new();

    if let Some(obj) = args.as_object() {
        // Convert based on tool type - map OpenAI function parameters to positional args
        match tool_name {
            "open" => {
                if let Some(path) = obj.get("path").and_then(|v| v.as_str()) {
                    positional_args.push(path.to_string());
                }
                if let Some(line_num) = obj.get("line_number").and_then(|v| v.as_u64()) {
                    positional_args.push(line_num.to_string());
                }
            }
            "goto" => {
                if let Some(line_num) = obj.get("line_number").and_then(|v| v.as_u64()) {
                    positional_args.push(line_num.to_string());
                }
            }
            "create" => {
                if let Some(filename) = obj.get("filename").and_then(|v| v.as_str()) {
                    positional_args.push(filename.to_string());
                }
            }
            "find_file" => {
                if let Some(file_name) = obj.get("file_name").and_then(|v| v.as_str()) {
                    positional_args.push(file_name.to_string());
                }
                if let Some(dir) = obj.get("dir").and_then(|v| v.as_str()) {
                    positional_args.push(dir.to_string());
                }
            }
            "search_file" => {
                if let Some(search_term) = obj.get("search_term").and_then(|v| v.as_str()) {
                    positional_args.push(search_term.to_string());
                }
                if let Some(file) = obj.get("file").and_then(|v| v.as_str()) {
                    positional_args.push(file.to_string());
                }
            }
            "search_dir" => {
                if let Some(search_term) = obj.get("search_term").and_then(|v| v.as_str()) {
                    positional_args.push(search_term.to_string());
                }
                if let Some(dir) = obj.get("dir").and_then(|v| v.as_str()) {
                    positional_args.push(dir.to_string());
                }
            }
            "edit" => {
                // For edit tool, preserve all arguments as named arguments to support complex modes
                for (key, value) in obj {
                    if let Some(str_val) = value.as_str() {
                        named_args.insert(key.clone(), str_val.to_string());
                    } else if let Some(bool_val) = value.as_bool() {
                        named_args.insert(key.clone(), bool_val.to_string());
                    } else if let Some(num_val) = value.as_u64() {
                        named_args.insert(key.clone(), num_val.to_string());
                    } else {
                        named_args.insert(key.clone(), value.to_string());
                    }
                }
                // Also maintain backward compatibility with positional args
                if let Some(path) = obj.get("path").and_then(|v| v.as_str()) {
                    positional_args.push(path.to_string());
                }
                if let Some(old_text) = obj.get("old_text").and_then(|v| v.as_str()) {
                    positional_args.push(old_text.to_string());
                }
                if let Some(new_text) = obj.get("new_text").and_then(|v| v.as_str()) {
                    positional_args.push(new_text.to_string());
                }
            }
            "insert" => {
                if let Some(path) = obj.get("path").and_then(|v| v.as_str()) {
                    positional_args.push(path.to_string());
                }
                if let Some(line_num) = obj.get("line_number").and_then(|v| v.as_u64()) {
                    positional_args.push(line_num.to_string());
                }
                if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                    positional_args.push(text.to_string());
                }
            }
            "run_command" => {
                if let Some(command) = obj.get("command").and_then(|v| v.as_str()) {
                    positional_args.push(command.to_string());
                }
            }
            "filemap" => {
                if let Some(file_path) = obj.get("file_path").and_then(|v| v.as_str()) {
                    positional_args.push(file_path.to_string());
                }
            }
            // File Creation Tools
            "create_file" => {
                if let Some(path) = obj.get("path").and_then(|v| v.as_str()) {
                    positional_args.push(path.to_string());
                }
                if let Some(content) = obj.get("content").and_then(|v| v.as_str()) {
                    positional_args.push(content.to_string());
                }
            }
            // File Editing Tools
            "replace_text" => {
                if let Some(path) = obj.get("path").and_then(|v| v.as_str()) {
                    positional_args.push(path.to_string());
                }
                if let Some(old_text) = obj.get("old_text").and_then(|v| v.as_str()) {
                    positional_args.push(old_text.to_string());
                }
                if let Some(new_text) = obj.get("new_text").and_then(|v| v.as_str()) {
                    positional_args.push(new_text.to_string());
                }
                if let Some(occurrence) = obj.get("occurrence").and_then(|v| v.as_u64()) {
                    positional_args.push(occurrence.to_string());
                }
            }
            "delete_function" => {
                if let Some(file_name) = obj
                    .get("file_name")
                    .and_then(|v| v.as_str())
                    .or_else(|| obj.get("path").and_then(|v| v.as_str()))
                {
                    positional_args.push(file_name.to_string());
                }
                if let Some(function_name) = obj.get("function_name").and_then(|v| v.as_str()) {
                    positional_args.push(function_name.to_string());
                }
            }
            "insert_text" => {
                if let Some(path) = obj.get("path").and_then(|v| v.as_str()) {
                    positional_args.push(path.to_string());
                }
                if let Some(line_number) = obj.get("line_number").and_then(|v| v.as_u64()) {
                    positional_args.push(line_number.to_string());
                }
                if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                    positional_args.push(text.to_string());
                }
            }
            "delete_text" => {
                if let Some(path) = obj.get("path").and_then(|v| v.as_str()) {
                    positional_args.push(path.to_string());
                }
                // Accept both keys for robustness: prefer the documented `text_to_delete` but fall back to `text`.
                if let Some(text_td) = obj.get("text_to_delete").and_then(|v| v.as_str()) {
                    positional_args.push(text_td.to_string());
                } else if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                    positional_args.push(text.to_string());
                }
                if let Some(occurrence) = obj.get("occurrence").and_then(|v| v.as_u64()) {
                    positional_args.push(occurrence.to_string());
                }
            }
            "overwrite_file" => {
                if let Some(path) = obj.get("path").and_then(|v| v.as_str()) {
                    positional_args.push(path.to_string());
                }
                if let Some(content) = obj.get("content").and_then(|v| v.as_str()) {
                    positional_args.push(content.to_string());
                }
            }
            "delete_line" => {
                if let Some(path) = obj.get("path").and_then(|v| v.as_str()) {
                    positional_args.push(path.to_string());
                }
                if let Some(start_line) = obj.get("start_line").and_then(|v| v.as_u64()) {
                    positional_args.push(start_line.to_string());
                }
                if let Some(end_line) = obj.get("end_line").and_then(|v| v.as_u64()) {
                    positional_args.push(end_line.to_string());
                }
            }
            // File Management Tools
            "delete_path" => {
                if let Some(path) = obj.get("path").and_then(|v| v.as_str()) {
                    positional_args.push(path.to_string());
                }
                if let Some(recursive) = obj.get("recursive").and_then(|v| v.as_bool()) {
                    positional_args.push(recursive.to_string());
                }
            }
            "copy_path" => {
                if let Some(src) = obj
                    .get("source")
                    .and_then(|v| v.as_str())
                    .or_else(|| obj.get("src").and_then(|v| v.as_str()))
                {
                    positional_args.push(src.to_string());
                    named_args.insert("source".to_string(), src.to_string());
                }
                if let Some(dest) = obj
                    .get("destination")
                    .and_then(|v| v.as_str())
                    .or_else(|| obj.get("dest").and_then(|v| v.as_str()))
                {
                    positional_args.push(dest.to_string());
                    named_args.insert("destination".to_string(), dest.to_string());
                }
                if let Some(recursive) = obj.get("recursive").and_then(|v| v.as_bool()) {
                    positional_args.push(recursive.to_string());
                    named_args.insert("recursive".to_string(), recursive.to_string());
                }
            }
            "move_path" => {
                if let Some(src) = obj
                    .get("source")
                    .and_then(|v| v.as_str())
                    .or_else(|| obj.get("src").and_then(|v| v.as_str()))
                {
                    positional_args.push(src.to_string());
                    named_args.insert("source".to_string(), src.to_string());
                }
                if let Some(dest) = obj
                    .get("destination")
                    .and_then(|v| v.as_str())
                    .or_else(|| obj.get("dest").and_then(|v| v.as_str()))
                {
                    positional_args.push(dest.to_string());
                    named_args.insert("destination".to_string(), dest.to_string());
                }
            }
            // File System Interaction Tools
            "list_directory" => {
                if let Some(path) = obj.get("path").and_then(|v| v.as_str()) {
                    positional_args.push(path.to_string());
                }
                if let Some(recursive) = obj.get("recursive").and_then(|v| v.as_bool()) {
                    positional_args.push(recursive.to_string());
                }
            }
            "get_file_info" => {
                if let Some(path) = obj.get("path").and_then(|v| v.as_str()) {
                    positional_args.push(path.to_string());
                }
            }
            // Tool for getting the current directory
            "get_current_directory" => {
                // No arguments needed for this tool
            }
            // Search and directory listing tools
            "find_files" => {
                if let Some(pattern) = obj.get("pattern").and_then(|v| v.as_str()) {
                    positional_args.push(pattern.to_string());
                }
                if let Some(dir) = obj.get("dir").and_then(|v| v.as_str()) {
                    positional_args.push(dir.to_string());
                }
            }
            "submit" => {
                // No arguments needed for submit
            }
            "classify_task" => {
                debug!("🔍 DEBUG convert_json_to_tool_args for classify_task:");
                debug!("   Input JSON: {}", serde_json::to_string(&args).unwrap_or_default());
                
                if let Some(task_type) = obj.get("task_type").and_then(|v| v.as_str()) {
                    debug!("   Found task_type: '{}'", task_type);
                    positional_args.push(task_type.to_string());
                } else {
                    debug!("   ❌ task_type field NOT FOUND in JSON!");
                    debug!("   Available keys: {:?}", obj.keys().collect::<Vec<_>>());
                }
            }
            _ => {
                // For unknown tools, try to convert everything as named arguments
                for (key, value) in obj {
                    if let Some(str_val) = value.as_str() {
                        named_args.insert(key.clone(), str_val.to_string());
                    } else if let Some(bool_val) = value.as_bool() {
                        named_args.insert(key.clone(), bool_val.to_string());
                    } else if let Some(num_val) = value.as_u64() {
                        named_args.insert(key.clone(), num_val.to_string());
                    } else {
                        named_args.insert(key.clone(), value.to_string());
                    }
                }
            }
        }
    }

    Ok(ToolArgs::with_named_args(positional_args, named_args))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_to_tool_args_simple() {
        let args = json!({
            "path": "/test/file.txt",
            "line_number": 42
        });

        let tool_args = json_to_tool_args("open", args).unwrap();
        let positional = &tool_args.args;

        assert_eq!(positional.len(), 2);
        assert_eq!(positional[0], "/test/file.txt");
        assert_eq!(positional[1], "42");
    }

    #[test]
    fn test_json_to_tool_args_edit_tool() {
        let args = json!({
            "path": "/test/file.txt",
            "old_text": "old content",
            "new_text": "new content",
            "mode": "replace"
        });

        let tool_args = json_to_tool_args("edit", args).unwrap();
        let positional = &tool_args.args;
        let named = &tool_args.named_args;

        assert_eq!(positional.len(), 3);
        assert_eq!(positional[0], "/test/file.txt");
        assert_eq!(positional[1], "old content");
        assert_eq!(positional[2], "new content");

        assert!(named.contains_key("mode"));
        assert_eq!(named.get("mode").unwrap(), "replace");
    }

    #[test]
    fn test_json_to_tool_args_copy_path() {
        let args = json!({
            "source": "templates/system_classification.md",
            "destination": "templates/system.md",
            "recursive": false
        });

        let tool_args = json_to_tool_args("copy_path", args).unwrap();
        let positional = &tool_args.args;
        let named = &tool_args.named_args;

        // Expect positional args populated in order: source, destination, recursive
        assert!(positional.len() >= 2);
        assert_eq!(positional[0], "templates/system_classification.md");
        assert_eq!(positional[1], "templates/system.md");
        // Recursive may be present as third positional arg
        if positional.len() >= 3 {
            assert_eq!(positional[2], "false");
        }

        // Also expect named args to include keys required by simpaticoder-tools
        assert!(named.contains_key("source"));
        assert!(named.contains_key("destination"));
        assert!(named.contains_key("recursive"));
    }

    #[test]
    fn test_json_to_tool_args_classify_task() {
        // Test the specific JSON format
        let args = json!({
            "task_type": "maintenance"
        });

        let tool_args = json_to_tool_args("classify_task", args).unwrap();
        let positional = &tool_args.args;
        let named = &tool_args.named_args;

        // classify_task expects positional arguments, not named
        assert_eq!(positional.len(), 1);
        assert_eq!(positional[0], "maintenance");
        assert!(named.is_empty()); // Should not have any named args for classify_task

        // Test all valid task types
        for task_type in &["bug_fix", "feature", "maintenance", "query"] {
            let args = json!({
                "task_type": task_type
            });

            let tool_args = json_to_tool_args("classify_task", args).unwrap();
            assert_eq!(tool_args.args.len(), 1);
            assert_eq!(tool_args.args[0], *task_type);
        }
    }
}
