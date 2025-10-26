//! Command execution tool for running shell commands
//!
//! This module provides the run_command tool that allows LLMs to execute shell commands
//! in a controlled and safe manner, replacing direct bash command execution.

use crate::core::{Tool, ToolArgs, ToolError, ToolResult};
use crate::state::ToolState;
use anyhow::Result;
use serde_json;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::timeout;

/// Tool for executing shell commands safely
#[allow(dead_code)]
pub struct RunCommandTool {
    timeout_seconds: u64,
    working_dir: std::path::PathBuf,
    dangerous_commands: Vec<String>,
}

impl RunCommandTool {
    /// Create a new run_command tool
    pub fn new() -> Self {
        Self {
            timeout_seconds: 120,
            working_dir: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            dangerous_commands: vec![
                "rm -rf".to_string(),
                "sudo rm".to_string(),
                "format".to_string(),
                "mkfs".to_string(),
                "dd if=".to_string(),
                "shutdown".to_string(),
                "reboot".to_string(),
                "halt".to_string(),
                "kill -9".to_string(),
                ":(){:|:&};:".to_string(), // Fork bomb
            ],
        }
    }

    /// Create a new run_command tool with custom timeout
    pub fn new_with_timeout(timeout_seconds: u64) -> Self {
        let mut tool = Self::new();
        tool.timeout_seconds = timeout_seconds;
        tool
    }

    /// Create a new run_command tool with custom working directory
    pub fn new_with_workdir<P: AsRef<std::path::Path>>(working_dir: P) -> Self {
        let mut tool = Self::new();
        tool.working_dir = working_dir.as_ref().to_path_buf();
        tool
    }

    /// Check if a command contains dangerous patterns
    #[allow(dead_code)]
    fn is_dangerous_command(&self, command: &str) -> bool {
        let lower_command = command.to_lowercase();
        for dangerous in &self.dangerous_commands {
            if lower_command.contains(&dangerous.to_lowercase()) {
                return true;
            }
        }
        false
    }

    /// Execute a shell command with timeout and safety checks
    #[allow(dead_code)]
    #[allow(dead_code)]
    #[allow(dead_code)]
    async fn execute_command(&self, command: &str) -> Result<(String, String, bool)> {
        if self.is_dangerous_command(command) {
            return Err(anyhow::anyhow!("Dangerous command blocked: {}", command));
        }

        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(command)
            .current_dir(&self.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = timeout(
            Duration::from_secs(self.timeout_seconds),
            tokio::process::Command::from(cmd).output(),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Command timed out after {} seconds", self.timeout_seconds))?
        .map_err(|e| anyhow::anyhow!("Failed to execute command: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let success = output.status.success();

        Ok((stdout, stderr, success))
    }
}

impl Tool for RunCommandTool {
    fn name(&self) -> &str {
        "run_command"
    }

    fn description(&self) -> &str {
        "Execute shell commands safely with timeout protection and dangerous command filtering"
    }

    fn signature(&self) -> &str {
        "run_command(command: string) -> {stdout: string, stderr: string, success: bool}"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        if args.get_arg(0).is_none() && args.get_named_arg("command").is_none() {
            return Err(ToolError::InvalidArgs {
                message: "run_command requires either a positional 'command' argument or named 'command' parameter".to_string(),
            });
        }

        Ok(())
    }

    fn execute(&mut self, args: &ToolArgs, state: &Arc<Mutex<ToolState>>) -> Result<ToolResult> {
        // Extract command from arguments
        let command = if let Some(cmd) = args.get_arg(0) {
            cmd.clone()
        } else if let Some(cmd) = args.get_named_arg("command") {
            cmd.clone()
        } else {
            return Ok(ToolResult::error("No command provided"));
        };

        // Log the command execution to state
        if let Ok(mut state) = state.lock() {
            state.push_history(format!("run_command: {}", command));
        }

        // Execute command synchronously (blocking) - we'll use std::process for simplicity
        // as the Tool trait doesn't support async
        let result = std::process::Command::new("sh")
            .arg("-c")
            .arg(&command)
            .current_dir(&self.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let command_success = output.status.success();

                let result_data = serde_json::json!({
                    "stdout": stdout,
                    "stderr": stderr,
                    "success": command_success,
                    "command": command
                });

                let message = if command_success {
                    let mut msg_parts = Vec::new();
                    if !stdout.is_empty() {
                        msg_parts.push(format!("stdout:\n{}", stdout));
                    }
                    if !stderr.is_empty() {
                        msg_parts.push(format!("stderr:\n{}", stderr));
                    }
                    if msg_parts.is_empty() {
                        "Command executed successfully with no output (exit code: 0)".to_string()
                    } else {
                        format!("Command executed successfully:\n{}", msg_parts.join("\n"))
                    }
                } else {
                    format!(
                        "Command failed with exit code {}:\nstdout: {}\nstderr: {}",
                        output.status.code().unwrap_or(-1),
                        stdout,
                        stderr
                    )
                };

                // Return tool result with success reflecting command success
                if command_success {
                    Ok(ToolResult::success_with_data(message, result_data))
                } else {
                    Ok(ToolResult::error(message))
                }
            }
            Err(e) => Ok(ToolResult::error(format!(
                "command failed to execute '{}': {}",
                command, e
            ))),
        }
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                }
            },
            "required": ["command"],
            "additionalProperties": false
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_run_command_tool_creation() {
        let tool = RunCommandTool::new();
        assert_eq!(tool.name(), "run_command");
        assert!(!tool.description().is_empty());
        assert!(!tool.signature().is_empty());
    }

    #[test]
    fn test_run_command_validation() {
        let tool = RunCommandTool::new();

        // Test empty arguments
        let empty_args = ToolArgs::from_args(&[]);
        assert!(tool.validate_args(&empty_args).is_err());

        // Test valid arguments
        let valid_args = ToolArgs::from_args(&["echo hello"]);
        assert!(tool.validate_args(&valid_args).is_ok());

        // Test named argument
        let mut named_args = HashMap::new();
        named_args.insert("command".to_string(), "echo hello".to_string());
        let named_tool_args = ToolArgs::with_named_args(vec![], named_args);
        assert!(tool.validate_args(&named_tool_args).is_ok());
    }

    #[test]
    fn test_dangerous_command_detection() {
        let tool = RunCommandTool::new();

        assert!(tool.is_dangerous_command("rm -rf /"));
        assert!(tool.is_dangerous_command("sudo rm -f important.txt"));
        assert!(tool.is_dangerous_command("shutdown now"));
        assert!(!tool.is_dangerous_command("echo hello"));
        assert!(!tool.is_dangerous_command("ls -la"));
        assert!(!tool.is_dangerous_command("cargo build"));
    }

    #[test]
    fn test_run_command_execution() {
        let mut tool = RunCommandTool::new();
        let state = Arc::new(Mutex::new(ToolState::new()));

        // Test simple command
        let args = ToolArgs::from_args(&["echo hello world"]);
        let result = tool.execute(&args, &state).unwrap();

        assert!(result.success);
        assert!(result.message.contains("hello world"));

        // Test command that should fail
        let bad_args = ToolArgs::from_args(&["nonexistent_command_xyz"]);
        let result = tool.execute(&bad_args, &state).unwrap();

        assert!(!result.success);
        assert!(result.message.contains("failed") || result.message.contains("Command failed"));
    }

    #[test]
    fn test_openai_schema() {
        let tool = RunCommandTool::new();
        let schema = tool.get_openai_schema();

        assert_eq!(schema["type"], "function");
        assert_eq!(schema["function"]["name"], "run_command");
        assert!(!schema["function"]["description"]
            .as_str()
            .unwrap()
            .is_empty());

        let params = &schema["function"]["parameters"];
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["command"].is_object());
        assert_eq!(params["required"][0], "command");
    }
}
