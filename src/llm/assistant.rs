//! Assistant content generation for tool calls.
//!
//! When the LLM makes a tool call without providing its own text content,
//! the OpenAI API still requires an assistant message (it cannot be null).
//! We return an empty string — the UI already shows tool call status via
//! ToolPending/ToolDone events with spinners, so verbal narration is
//! redundant clutter.

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

/// Generate assistant content for tool calls.
///
/// Returns an empty string — the OpenAI API requires a non-null assistant
/// message before tool calls, but the actual narration is handled by the
/// UI layer (tool call spinners, status lines, etc.).
pub fn generate_assistant_content(_tool_calls: &[ToolCallInfo]) -> String {
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_assistant_content_single() {
        let tool_call = ToolCallInfo::new("read_file", r#"{"path": "/test/file.txt"}"#);
        let content = generate_assistant_content(&[tool_call]);
        assert_eq!(content, "");
    }

    #[test]
    fn test_generate_assistant_content_multiple() {
        let tool_calls = vec![
            ToolCallInfo::new("read_file", "{}"),
            ToolCallInfo::new("write_file", "{}"),
        ];
        let content = generate_assistant_content(&tool_calls);
        assert_eq!(content, "");
    }

    #[test]
    fn test_generate_assistant_content_empty() {
        let content = generate_assistant_content(&[]);
        assert_eq!(content, "");
    }
}
