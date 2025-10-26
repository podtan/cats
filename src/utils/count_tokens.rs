use crate::core::{Tool, ToolArgs, ToolError, ToolResult};
use crate::state::ToolState;
use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Tool to count tokens using tiktoken_rs cl100k_base encoding.
pub struct CountTokensTool {
    name: String,
}

impl CountTokensTool {
    pub fn new() -> Self {
        Self {
            name: "count_tokens".to_string(),
        }
    }

    /// Helper to count tokens from a string using cl100k_base tokenizer if enabled, otherwise fallback to whitespace count.
    #[cfg(feature = "tiktoken")]
    fn count_from_text(text: &str) -> usize {
        match tiktoken_rs::cl100k_base() {
            Ok(bpe) => {
                let tokens = bpe.encode_with_special_tokens(text);
                tokens.len()
            }
            Err(_) => text.split_whitespace().count(),
        }
    }

    #[cfg(not(feature = "tiktoken"))]
    fn count_from_text(text: &str) -> usize {
        // Fallback simple tokenization: split on whitespace
        text.split_whitespace().count()
    }
}

impl Tool for CountTokensTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Count tokens in a file or provided content using cl100k_base tokenizer"
    }

    fn signature(&self) -> &str {
        "count_tokens <file_path> OR count_tokens --content=<text>"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        if args.is_empty() {
            return Err(ToolError::InvalidArgs {
                message: "Usage: count_tokens <file_path> or count_tokens --content=<text>"
                    .to_string(),
            });
        }
        Ok(())
    }

    fn execute(&mut self, args: &ToolArgs, _state: &Arc<Mutex<ToolState>>) -> Result<ToolResult> {
        // If named arg `content` present, use it. Otherwise, take first positional arg as file path.
        if let Some(content) = args.get_named_arg("content") {
            let count = Self::count_from_text(content);
            return Ok(ToolResult::success_with_data(
                format!("Total tokens: {}", count),
                serde_json::json!({"tokens": count}),
            ));
        }

        let path = args.get_arg(0).unwrap();
        let path_buf = PathBuf::from(path);

        if !path_buf.exists() {
            return Ok(ToolResult::error(format!("Path not found: {}", path)));
        }

        let content = fs::read_to_string(&path_buf)
            .map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?;

        let count = Self::count_from_text(&content);

        Ok(ToolResult::success_with_data(
            format!("Total tokens: {}", count),
            serde_json::json!({"tokens": count, "path": path}),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": { "type": "string", "description": "Path to file to count tokens for (positional)" },
                "content": { "type": "string", "description": "Direct content to count tokens for (named)" }
            },
            "required": []
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ToolArgs;
    use crate::state::ToolState;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_count_tokens_from_text() {
        let mut tool = CountTokensTool::new();
        let args = ToolArgs::from_args(&["--content=hello world"]);
        let state = Arc::new(Mutex::new(ToolState::new()));
        let res = tool.execute(&args, &state).unwrap();
        assert!(res.message.contains("Total tokens:"));
    }

    #[test]
    fn test_count_tokens_file() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "This is a small test file").unwrap();
        let mut tool = CountTokensTool::new();
        let args = ToolArgs::from_args(&[tmp.path().to_string_lossy().as_ref()]);
        let state = Arc::new(Mutex::new(ToolState::new()));
        let res = tool.execute(&args, &state).unwrap();
        assert!(res.message.contains("Total tokens:"));
    }
}
