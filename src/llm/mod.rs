//! LLM integration module for CATS tools
//!
//! This module provides utilities for integrating CATS tools with LLM providers
//! like OpenAI, Anthropic, and others that use function calling with JSON arguments.
//!
//! ## Features
//!
//! - **JSON Conversion**: Convert LLM function call JSON to CATS ToolArgs
//! - **Tool Execution**: Execute tools with proper logging and error handling
//! - **Result Handling**: Handle large tool results with truncation
//! - **Assistant Content**: Generate human-friendly descriptions of tool calls

pub mod assistant;
pub mod converter;
pub mod executor;
pub mod result_handler;

// Re-export main types
pub use assistant::generate_assistant_content;
pub use converter::json_to_tool_args;
pub use executor::{execute_tool_calls, execute_tool_calls_structured, ToolExecutionResult};
pub use result_handler::handle_large_result;
