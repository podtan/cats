//! Core traits and types for the simpaticoder tools system

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// Error types for tool operations
#[derive(Debug, Error)]
pub enum ToolError {
    #[error("File not found: {path}")]
    FileNotFound { path: String },
    #[error("Invalid arguments: {message}")]
    InvalidArgs { message: String },
    #[error("Tool not found: {name}")]
    ToolNotFound { name: String },
    #[error("Linting failed: {errors:?}")]
    LintingFailed { errors: Vec<String> },
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Arguments passed to tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolArgs {
    pub args: Vec<String>,
    pub named_args: HashMap<String, String>,
}

impl ToolArgs {
    /// Create ToolArgs from command line arguments
    pub fn from_args(args: &[&str]) -> Self {
        let mut positional = Vec::new();
        let mut named = HashMap::new();

        for &arg in args {
            if arg.starts_with("--") {
                // Support --key=value and --key value? We'll support --key=value and --key= value
                if let Some(eq) = arg.find('=') {
                    let key = arg[2..eq].to_string();
                    let value = arg[eq + 1..].to_string();
                    named.insert(key, value);
                } else {
                    // Flag without value; store as true
                    let key = arg[2..].to_string();
                    named.insert(key, "true".to_string());
                }
            } else {
                positional.push(arg.to_string());
            }
        }

        Self {
            args: positional,
            named_args: named,
        }
    }

    /// Create ToolArgs with named arguments
    pub fn with_named_args(args: Vec<String>, named_args: HashMap<String, String>) -> Self {
        Self { args, named_args }
    }

    /// Get positional argument by index
    pub fn get_arg(&self, index: usize) -> Option<&String> {
        self.args.get(index)
    }

    /// Get named argument
    pub fn get_named_arg(&self, name: &str) -> Option<&String> {
        self.named_args.get(name)
    }

    /// Get argument count
    pub fn len(&self) -> usize {
        self.args.len()
    }

    /// Check if arguments are empty
    pub fn is_empty(&self) -> bool {
        self.args.is_empty()
    }
}

/// Result returned by tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl ToolResult {
    /// Create successful result
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
        }
    }

    /// Create successful result with data
    pub fn success_with_data(message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
        }
    }

    /// Create error result
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
        }
    }

    /// Create error result with data
    pub fn error_with_data(message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: Some(data),
        }
    }
}

/// Main trait for all tools
pub trait Tool: Send + Sync {
    /// Get the tool name
    fn name(&self) -> &str;

    /// Get the tool description
    fn description(&self) -> &str;

    /// Get the tool usage/signature
    fn signature(&self) -> &str;

    /// Validate arguments before execution
    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError>;

    /// Execute the tool with given arguments
    fn execute(
        &mut self,
        args: &ToolArgs,
        state: &Arc<Mutex<crate::state::ToolState>>,
    ) -> Result<ToolResult>;

    /// Get OpenAI function schema for this tool
    fn get_openai_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name(),
                "description": self.description(),
                "parameters": self.get_parameters_schema()
            }
        })
    }

    /// Get parameters schema - should be overridden by implementing tools
    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }
}

/// Registry for managing available tools
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
    state: Arc<Mutex<crate::state::ToolState>>,
}

impl ToolRegistry {
    /// Create a new empty tool registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            state: Arc::new(Mutex::new(crate::state::ToolState::new())),
        }
    }

    /// Register a tool
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        let name = tool.name().to_string();
        self.tools.insert(name, tool);
    }

    /// Execute a tool by name
    pub fn execute_tool(&mut self, name: &str, args: &ToolArgs) -> Result<ToolResult, ToolError> {
        let tool = self
            .tools
            .get_mut(name)
            .ok_or_else(|| ToolError::ToolNotFound {
                name: name.to_string(),
            })?;

        // Validate arguments
        tool.validate_args(args)?;

        // Execute tool
        tool.execute(args, &self.state)
            .map_err(|e| ToolError::InvalidArgs {
                message: e.to_string(),
            })
    }

    /// List all registered tool names
    pub fn list_tools(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Get tool by name
    pub fn get_tool(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    /// Get OpenAI function schemas for all tools
    pub fn get_all_schemas(&self) -> Vec<serde_json::Value> {
        self.tools
            .values()
            .map(|tool| tool.get_openai_schema())
            .collect()
    }

    /// Get the current tool state
    pub fn get_state(&self) -> Arc<Mutex<crate::state::ToolState>> {
        Arc::clone(&self.state)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock tool for testing
    struct MockTool {
        name: String,
    }

    impl MockTool {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
            }
        }
    }

    impl Tool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "Mock tool for testing"
        }

        fn signature(&self) -> &str {
            "mock_tool <arg>"
        }

        fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
            if args.is_empty() {
                return Err(ToolError::InvalidArgs {
                    message: "Mock tool requires at least one argument".to_string(),
                });
            }
            Ok(())
        }

        fn execute(
            &mut self,
            args: &ToolArgs,
            _state: &Arc<Mutex<crate::state::ToolState>>,
        ) -> Result<ToolResult> {
            Ok(ToolResult::success(format!(
                "Mock tool {} executed with {} args",
                self.name,
                args.len()
            )))
        }
    }

    #[test]
    fn test_tool_args_creation() {
        let args = ToolArgs::from_args(&["arg1", "arg2"]);
        assert_eq!(args.len(), 2);
        assert_eq!(args.get_arg(0), Some(&"arg1".to_string()));
        assert_eq!(args.get_arg(1), Some(&"arg2".to_string()));
    }

    #[test]
    fn test_tool_result_creation() {
        let result = ToolResult::success("Test message");
        assert!(result.success);
        assert_eq!(result.message, "Test message");
        assert!(result.data.is_none());

        let error_result = ToolResult::error("Error message");
        assert!(!error_result.success);
        assert_eq!(error_result.message, "Error message");
    }

    #[test]
    fn test_tool_registry() {
        let mut registry = ToolRegistry::new();
        let mock_tool = Box::new(MockTool::new("test_tool"));

        registry.register(mock_tool);

        let tools = registry.list_tools();
        assert!(tools.contains(&"test_tool".to_string()));

        // Test successful execution
        let args = ToolArgs::from_args(&["test_arg"]);
        let result = registry.execute_tool("test_tool", &args);
        assert!(result.is_ok());
        assert!(result.unwrap().success);

        // Test tool not found
        let result = registry.execute_tool("nonexistent", &args);
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_validation() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(MockTool::new("test_tool")));

        // Test validation failure
        let empty_args = ToolArgs::from_args(&[]);
        let result = registry.execute_tool("test_tool", &empty_args);
        assert!(result.is_err());
    }
}
