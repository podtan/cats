//! Tool execution orchestration for LLM function calls
//!
//! Provides utilities to execute tool calls with proper error handling,
//! result processing, and optional callbacks for logging and custom behavior.

use crate::ToolRegistry;
use anyhow::Result;
use serde_json::Value;

use super::converter::json_to_tool_args;
use super::result_handler::{handle_large_result, ResultHandlerConfig};

/// Result from a single tool execution
#[derive(Debug, Clone)]
pub struct ToolExecutionResult {
    /// Tool call ID (for provider correlation)
    pub tool_call_id: String,
    /// Tool name
    pub tool_name: String,
    /// Result content or error message
    pub content: String,
    /// Whether the execution succeeded
    pub success: bool,
}

/// Simple tool call representation for execution
#[derive(Debug, Clone)]
pub struct ToolCallRequest {
    /// Tool call ID
    pub id: String,
    /// Tool name
    pub name: String,
    /// JSON arguments as string
    pub arguments: String,
}

impl ToolCallRequest {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            arguments: arguments.into(),
        }
    }
}

/// Optional callback for logging and custom behavior
pub trait ExecutionCallback {
    /// Called before executing a tool
    fn on_tool_start(&mut self, tool_name: &str, args: &str);

    /// Called after tool execution (success or failure)
    fn on_tool_complete(&mut self, tool_name: &str, args: &str, result: &str, success: bool);

    /// Called for compact logging (JSON format)
    fn on_compact_log(&mut self, compact_json: &str);
}

/// Default no-op callback
pub struct NoOpCallback;

impl ExecutionCallback for NoOpCallback {
    fn on_tool_start(&mut self, _tool_name: &str, _args: &str) {}
    fn on_tool_complete(&mut self, _tool_name: &str, _args: &str, _result: &str, _success: bool) {}
    fn on_compact_log(&mut self, _compact_json: &str) {}
}

/// Execute tool calls and return combined result string
///
/// # Arguments
/// * `registry` - Tool registry to execute tools from
/// * `tool_calls` - Vector of tool call requests
/// * `result_config` - Configuration for result handling
/// * `callback` - Optional callback for logging and custom behavior
///
/// # Returns
/// * `Result<String>` - Combined results from all tool executions
pub fn execute_tool_calls(
    registry: &mut ToolRegistry,
    tool_calls: Vec<ToolCallRequest>,
    result_config: &ResultHandlerConfig,
    callback: &mut dyn ExecutionCallback,
) -> Result<String> {
    let mut results = Vec::new();

    for tool_call in tool_calls {
        let tool_name = &tool_call.name;
        let tool_args_str = &tool_call.arguments;

        // Log the compact tool call JSON
        let compact_json = serde_json::json!({
            "name": tool_name,
            "arguments": serde_json::from_str::<Value>(tool_args_str).unwrap_or(serde_json::json!({}))
        });
        if let Ok(compact_str) = serde_json::to_string(&compact_json) {
            callback.on_compact_log(&compact_str);
        }

        callback.on_tool_start(tool_name, tool_args_str);

        let args: Value = match serde_json::from_str(tool_args_str) {
            Ok(args) => args,
            Err(e) => {
                let error_msg = format!("Failed to parse tool arguments for {}: {}", tool_name, e);
                callback.on_tool_complete(tool_name, tool_args_str, &error_msg, false);
                results.push(format!("Tool: {}\nError: {}", tool_name, error_msg));
                continue;
            }
        };

        // Convert JSON args to ToolArgs format
        let tool_args = match json_to_tool_args(tool_name, args) {
            Ok(args) => args,
            Err(e) => {
                let error_msg = format!("Failed to convert arguments for {}: {}", tool_name, e);
                callback.on_tool_complete(tool_name, tool_args_str, &error_msg, false);
                results.push(format!("Tool: {}\nError: {}", tool_name, error_msg));
                continue;
            }
        };

        match registry.execute_tool(tool_name, &tool_args) {
            Ok(result) => {
                callback.on_tool_complete(tool_name, tool_args_str, &result.message, true);

                // Apply large result handling
                let processed_result = handle_large_result(tool_name, &result.message, result_config);
                results.push(format!("Tool: {}\nResult: {}", tool_name, processed_result));
            }
            Err(e) => {
                let error_msg = format!("Tool execution failed for {}: {}", tool_name, e);
                callback.on_tool_complete(tool_name, tool_args_str, &error_msg, false);
                results.push(format!("Tool: {}\nError: {}", tool_name, error_msg));
            }
        }
    }

    Ok(results.join("\n\n"))
}

/// Execute tool calls and return structured results
///
/// Returns individual tool results that can be sent as separate tool messages
/// to LLM providers.
///
/// # Arguments
/// * `registry` - Tool registry to execute tools from
/// * `tool_calls` - Vector of tool call requests
/// * `result_config` - Configuration for result handling
/// * `callback` - Optional callback for logging and custom behavior
///
/// # Returns
/// * `Result<Vec<ToolExecutionResult>>` - Individual tool execution results
pub fn execute_tool_calls_structured(
    registry: &mut ToolRegistry,
    tool_calls: Vec<ToolCallRequest>,
    result_config: &ResultHandlerConfig,
    callback: &mut dyn ExecutionCallback,
) -> Result<Vec<ToolExecutionResult>> {
    let mut results = Vec::new();

    for tool_call in tool_calls {
        let tool_name = &tool_call.name;
        let tool_args_str = &tool_call.arguments;
        let tool_call_id = tool_call.id.clone();

        // Log the compact tool call JSON
        let compact_json = serde_json::json!({
            "name": tool_name,
            "arguments": serde_json::from_str::<Value>(tool_args_str).unwrap_or(serde_json::json!({}))
        });
        if let Ok(compact_str) = serde_json::to_string(&compact_json) {
            callback.on_compact_log(&compact_str);
        }

        callback.on_tool_start(tool_name, tool_args_str);

        let args: Value = match serde_json::from_str(tool_args_str) {
            Ok(args) => args,
            Err(e) => {
                let error_msg = format!("Failed to parse tool arguments for {}: {}", tool_name, e);
                callback.on_tool_complete(tool_name, tool_args_str, &error_msg, false);
                results.push(ToolExecutionResult {
                    tool_call_id,
                    tool_name: tool_name.clone(),
                    content: error_msg,
                    success: false,
                });
                continue;
            }
        };

        // Convert JSON args to ToolArgs format
        let tool_args = match json_to_tool_args(tool_name, args) {
            Ok(args) => args,
            Err(e) => {
                let error_msg = format!("Failed to convert arguments for {}: {}", tool_name, e);
                callback.on_tool_complete(tool_name, tool_args_str, &error_msg, false);
                results.push(ToolExecutionResult {
                    tool_call_id,
                    tool_name: tool_name.clone(),
                    content: error_msg,
                    success: false,
                });
                continue;
            }
        };

        match registry.execute_tool(tool_name, &tool_args) {
            Ok(result) => {
                callback.on_tool_complete(tool_name, tool_args_str, &result.message, true);

                // Apply large result handling
                let processed_result = handle_large_result(tool_name, &result.message, result_config);
                results.push(ToolExecutionResult {
                    tool_call_id,
                    tool_name: tool_name.clone(),
                    content: processed_result,
                    success: true,
                });
            }
            Err(e) => {
                let error_msg = format!("Tool execution failed for {}: {}", tool_name, e);
                callback.on_tool_complete(tool_name, tool_args_str, &error_msg, false);
                results.push(ToolExecutionResult {
                    tool_call_id,
                    tool_name: tool_name.clone(),
                    content: error_msg,
                    success: false,
                });
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create_tool_registry;

    struct TestCallback {
        logs: Vec<String>,
    }

    impl TestCallback {
        fn new() -> Self {
            Self { logs: Vec::new() }
        }
    }

    impl ExecutionCallback for TestCallback {
        fn on_tool_start(&mut self, tool_name: &str, _args: &str) {
            self.logs.push(format!("START: {}", tool_name));
        }

        fn on_tool_complete(&mut self, tool_name: &str, _args: &str, _result: &str, success: bool) {
            self.logs
                .push(format!("COMPLETE: {} ({})", tool_name, success));
        }

        fn on_compact_log(&mut self, compact_json: &str) {
            self.logs.push(format!("COMPACT: {}", compact_json));
        }
    }

    #[test]
    fn test_execute_tool_calls_success() {
        let mut registry = create_tool_registry();
        let config = ResultHandlerConfig::default();
        let mut callback = TestCallback::new();

        let tool_calls = vec![ToolCallRequest::new(
            "call_1",
            "_state",
            "{}",
        )];

        let result = execute_tool_calls(&mut registry, tool_calls, &config, &mut callback);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("Tool: _state"));
        assert!(!callback.logs.is_empty());
    }

    #[test]
    fn test_execute_tool_calls_structured() {
        let mut registry = create_tool_registry();
        let config = ResultHandlerConfig::default();
        let mut callback = NoOpCallback;

        let tool_calls = vec![ToolCallRequest::new(
            "call_1",
            "_state",
            "{}",
        )];

        let results =
            execute_tool_calls_structured(&mut registry, tool_calls, &config, &mut callback);
        assert!(results.is_ok());

        let results = results.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].tool_call_id, "call_1");
        assert_eq!(results[0].tool_name, "_state");
        assert!(results[0].success);
    }

    #[test]
    fn test_execute_tool_calls_invalid_json() {
        let mut registry = create_tool_registry();
        let config = ResultHandlerConfig::default();
        let mut callback = NoOpCallback;

        let tool_calls = vec![ToolCallRequest::new(
            "call_1",
            "_state",
            "invalid json",
        )];

        let results =
            execute_tool_calls_structured(&mut registry, tool_calls, &config, &mut callback);
        assert!(results.is_ok());

        let results = results.unwrap();
        assert_eq!(results.len(), 1);
        assert!(!results[0].success);
        assert!(results[0].content.contains("Failed to parse"));
    }
}
