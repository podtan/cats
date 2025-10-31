//! Assistant content generation for tool calls
//!
//! Generates human-friendly descriptions of tool calls, similar to
//! professional AI assistants (Zed, VS Code Copilot).

use serde_json::Value;

/// Simple tool call representation for assistant content generation
#[derive(Debug, Clone)]
pub struct ToolCallInfo {
    pub name: String,
    pub arguments: String,
}

impl ToolCallInfo {
    pub fn new(name: impl Into<String>, arguments: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments: arguments.into(),
        }
    }
}

/// Generate meaningful assistant content based on actual tool calls
///
/// This replaces generic "I'll execute the requested tools." with specific descriptions
/// like professional AI assistants do.
///
/// # Arguments
/// * `tool_calls` - Slice of tool call information
///
/// # Returns
/// * `String` - Human-friendly description of what the tools will do
pub fn generate_assistant_content(tool_calls: &[ToolCallInfo]) -> String {
    if tool_calls.is_empty() {
        return String::new(); // Empty content for tool-only responses
    }

    match tool_calls.len() {
        1 => {
            let tool_call = &tool_calls[0];
            match tool_call.name.as_str() {
                "classify_task" => {
                    "I'll analyze and classify this task to determine the best approach."
                        .to_string()
                }
                "read_file" | "open" => {
                    if let Ok(args) = serde_json::from_str::<Value>(&tool_call.arguments) {
                        if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
                            format!("I'll read the file at `{}`.", path)
                        } else {
                            "I'll read the specified file.".to_string()
                        }
                    } else {
                        "I'll read the file for you.".to_string()
                    }
                }
                "write_file" | "create_file" | "overwrite_file" => {
                    if let Ok(args) = serde_json::from_str::<Value>(&tool_call.arguments) {
                        if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
                            format!("I'll write content to `{}`.", path)
                        } else {
                            "I'll write to the specified file.".to_string()
                        }
                    } else {
                        "I'll write the file for you.".to_string()
                    }
                }
                "create_directory" => "I'll create the directory structure you need.".to_string(),
                "run_command" | "execute_command" => {
                    if let Ok(args) = serde_json::from_str::<Value>(&tool_call.arguments) {
                        if let Some(command) = args.get("command").and_then(|c| c.as_str()) {
                            format!("I'll execute the command: `{}`", command)
                        } else {
                            "I'll execute the specified command.".to_string()
                        }
                    } else {
                        "I'll execute the command for you.".to_string()
                    }
                }
                "submit" => "Task completed successfully.".to_string(),
                _ => format!(
                    "I'll use the {} tool to help with your request.",
                    tool_call.name
                ),
            }
        }
        2 => {
            let tool_names: Vec<&str> = tool_calls.iter().map(|tc| tc.name.as_str()).collect();
            format!(
                "I'll help you with that by using {} and {}.",
                tool_names[0], tool_names[1]
            )
        }
        _ => {
            let tool_names: Vec<&str> = tool_calls.iter().map(|tc| tc.name.as_str()).collect();
            format!(
                "I'll work on this using multiple tools: {}.",
                tool_names.join(", ")
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_assistant_content_single() {
        let tool_call = ToolCallInfo::new("read_file", r#"{"path": "/test/file.txt"}"#);

        let content = generate_assistant_content(&[tool_call]);
        assert!(content.contains("I'll read the file at `/test/file.txt`"));
    }

    #[test]
    fn test_generate_assistant_content_multiple() {
        let tool_calls = vec![
            ToolCallInfo::new("read_file", "{}"),
            ToolCallInfo::new("write_file", "{}"),
        ];

        let content = generate_assistant_content(&tool_calls);
        assert!(content.contains("I'll help you with that by using read_file and write_file"));
    }

    #[test]
    fn test_generate_assistant_content_classify() {
        let tool_call = ToolCallInfo::new("classify_task", r#"{"task_type": "bug_fix"}"#);

        let content = generate_assistant_content(&[tool_call]);
        assert!(content.contains("analyze and classify"));
    }

    #[test]
    fn test_generate_assistant_content_empty() {
        let content = generate_assistant_content(&[]);
        assert_eq!(content, "");
    }
}
