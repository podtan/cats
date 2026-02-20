//! Todo tools implementation compatible with OpenCode
//!
//! Manages a structured task list for coding sessions.

use crate::core::{Tool, ToolArgs, ToolError, ToolResult};
use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Todo item
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TodoItem {
    /// Brief description of the task
    pub content: String,
    /// Current status of the task (pending, in_progress, completed, cancelled)
    pub status: String,
    /// Priority level (high, medium, low)
    pub priority: String,
}

/// TodoWrite tool parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TodoWriteParams {
    /// The updated todo list
    pub todos: Vec<TodoItem>,
}

/// TodoWrite tool for managing task list
pub struct TodoWriteTool {
    name: String,
}

impl TodoWriteTool {
    pub fn new() -> Self {
        Self {
            name: "todowrite".to_string(),
        }
    }
}

impl Default for TodoWriteTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for TodoWriteTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Use this tool to create and manage a structured task list for your current coding session"
    }

    fn signature(&self) -> &str {
        "todowrite --todos <json>"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        if args.get_named_arg("todos").is_none() && args.args.is_empty() {
            return Err(ToolError::InvalidArgs {
                message: "todowrite tool requires a 'todos' argument".to_string(),
            });
        }
        Ok(())
    }

    fn execute(
        &mut self,
        args: &ToolArgs,
        _state: &Arc<Mutex<crate::state::ToolState>>,
    ) -> Result<ToolResult> {
        let params = parse_todowrite_args(args)?;

        // Validate todos
        for (i, todo) in params.todos.iter().enumerate() {
            if todo.content.is_empty() {
                return Err(anyhow::anyhow!("Todo item {} has empty content", i + 1).into());
            }

            let valid_status = matches!(
                todo.status.as_str(),
                "pending" | "in_progress" | "completed" | "cancelled"
            );
            if !valid_status {
                return Err(anyhow::anyhow!(
                    "Todo item {} has invalid status '{}'. Valid values: pending, in_progress, completed, cancelled",
                    i + 1,
                    todo.status
                ).into());
            }

            let valid_priority = matches!(todo.priority.as_str(), "high" | "medium" | "low");
            if !valid_priority {
                return Err(anyhow::anyhow!(
                    "Todo item {} has invalid priority '{}'. Valid values: high, medium, low",
                    i + 1,
                    todo.priority
                )
                .into());
            }
        }

        // Count by status
        let pending_count = params
            .todos
            .iter()
            .filter(|t| t.status == "pending" || t.status == "in_progress")
            .count();

        // Store todos in state (using a simple approach - in a real app, you'd have a dedicated todo store)
        // For now, we just return the todos
        let output = format!(
            "Updated todo list ({} active tasks):\n{}",
            pending_count,
            format_todos(&params.todos)
        );

        Ok(ToolResult::success_with_data(
            output,
            serde_json::json!({
                "todos": params.todos,
                "pending_count": pending_count,
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        let schema = schemars::schema_for!(TodoWriteParams);
        serde_json::to_value(schema).unwrap_or_default()
    }
}

fn parse_todowrite_args(args: &ToolArgs) -> Result<TodoWriteParams> {
    let todos_json = args
        .get_named_arg("todos")
        .cloned()
        .or_else(|| args.args.first().cloned())
        .ok_or_else(|| anyhow::anyhow!("todos is required"))?;

    let todos: Vec<TodoItem> = serde_json::from_str(&todos_json)
        .map_err(|e| anyhow::anyhow!("Failed to parse todos JSON: {}", e))?;

    Ok(TodoWriteParams { todos })
}

fn format_todos(todos: &[TodoItem]) -> String {
    let mut output = String::new();

    for (i, todo) in todos.iter().enumerate() {
        let status_icon = match todo.status.as_str() {
            "completed" => "✓",
            "in_progress" => "►",
            "cancelled" => "✗",
            _ => "○",
        };

        let priority_indicator = match todo.priority.as_str() {
            "high" => " [HIGH]",
            "low" => " [LOW]",
            _ => "",
        };

        output.push_str(&format!(
            "{} {}. {}{}\n",
            status_icon,
            i + 1,
            todo.content,
            priority_indicator
        ));
    }

    output
}

/// TodoRead tool for reading the current todo list
pub struct TodoReadTool {
    name: String,
}

impl TodoReadTool {
    pub fn new() -> Self {
        Self {
            name: "todoread".to_string(),
        }
    }
}

impl Default for TodoReadTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for TodoReadTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Use this tool to read your todo list"
    }

    fn signature(&self) -> &str {
        "todoread"
    }

    fn validate_args(&self, _args: &ToolArgs) -> Result<(), ToolError> {
        Ok(())
    }

    fn execute(
        &mut self,
        _args: &ToolArgs,
        _state: &Arc<Mutex<crate::state::ToolState>>,
    ) -> Result<ToolResult> {
        // In a real implementation, this would read from a session-based store
        // For now, we return an empty todo list
        let output = "Current todo list is empty. Use todowrite to create tasks.";

        Ok(ToolResult::success_with_data(
            output.to_string(),
            serde_json::json!({
                "todos": [],
                "pending_count": 0,
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_todowrite_tool_creation() {
        let tool = TodoWriteTool::new();
        assert_eq!(tool.name(), "todowrite");
    }

    #[test]
    fn test_todoread_tool_creation() {
        let tool = TodoReadTool::new();
        assert_eq!(tool.name(), "todoread");
    }

    #[test]
    fn test_todowrite_tool() {
        let mut tool = TodoWriteTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let todos = serde_json::json!([
            {"content": "Task 1", "status": "pending", "priority": "high"},
            {"content": "Task 2", "status": "in_progress", "priority": "medium"}
        ])
        .to_string();

        let args = ToolArgs::with_named_args(
            vec![],
            vec![("todos".to_string(), todos)].into_iter().collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);
        assert!(result.message.contains("Task 1"));
        assert!(result.message.contains("Task 2"));
    }

    #[test]
    fn test_todowrite_invalid_status() {
        let mut tool = TodoWriteTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let todos = serde_json::json!([
            {"content": "Task 1", "status": "invalid", "priority": "high"}
        ])
        .to_string();

        let args = ToolArgs::with_named_args(
            vec![],
            vec![("todos".to_string(), todos)].into_iter().collect(),
        );

        let result = tool.execute(&args, &state);
        assert!(result.is_err());
    }

    #[test]
    fn test_todoread_tool() {
        let mut tool = TodoReadTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let args = ToolArgs::from_args(&[]);

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_todowrite_validation() {
        let tool = TodoWriteTool::new();
        let args = ToolArgs::from_args(&[]);

        let result = tool.validate_args(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_format_todos() {
        let todos = vec![
            TodoItem {
                content: "Task 1".to_string(),
                status: "pending".to_string(),
                priority: "high".to_string(),
            },
            TodoItem {
                content: "Task 2".to_string(),
                status: "completed".to_string(),
                priority: "medium".to_string(),
            },
        ];

        let formatted = format_todos(&todos);
        assert!(formatted.contains("Task 1"));
        assert!(formatted.contains("Task 2"));
        assert!(formatted.contains("[HIGH]"));
    }
}
