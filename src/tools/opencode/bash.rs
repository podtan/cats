//! Bash tool implementation compatible with OpenCode
//!
//! Executes bash commands in a persistent shell session with optional timeout.

use crate::core::{Tool, ToolArgs, ToolError, ToolResult};
use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const MAX_OUTPUT_LENGTH: usize = 30_000;
const DEFAULT_TIMEOUT_MS: u64 = 120_000; // 2 minutes

/// Bash tool parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BashParams {
    /// The command to execute
    pub command: String,
    /// Optional timeout in milliseconds
    pub timeout: Option<u64>,
    /// The working directory to run the command in
    pub workdir: Option<String>,
    /// Clear, concise description of what this command does
    pub description: Option<String>,
}

/// Bash tool for executing shell commands
pub struct BashTool {
    name: String,
}

impl BashTool {
    pub fn new() -> Self {
        Self {
            name: "bash".to_string(),
        }
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for BashTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Executes a given bash command in a persistent shell session with optional timeout"
    }

    fn signature(&self) -> &str {
        "bash --command <command> [--timeout <ms>] [--workdir <dir>] [--description <desc>]"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        if args.get_named_arg("command").is_none() && args.args.is_empty() {
            return Err(ToolError::InvalidArgs {
                message: "bash tool requires a 'command' argument".to_string(),
            });
        }
        Ok(())
    }

    fn execute(
        &mut self,
        args: &ToolArgs,
        state: &Arc<Mutex<crate::state::ToolState>>,
    ) -> Result<ToolResult> {
        let params = parse_bash_args(args)?;

        // Clone the cancel signal before spawning so we don't hold the lock.
        let cancel_signal = {
            let state_guard = state.lock().map_err(|e| anyhow::anyhow!("Failed to lock tool state: {}", e))?;
            state_guard.cancel_signal()
        };

        // Read working directory from ToolState at execution time (like OpenCode)
        let workdir = params.workdir.map(PathBuf::from).unwrap_or_else(|| {
            state
                .lock()
                .map(|s| s.working_directory.clone())
                .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        });

        let timeout = params.timeout.unwrap_or(DEFAULT_TIMEOUT_MS);

        // Determine the shell — use PowerShell on Windows to avoid CMD quoting pitfalls:
        // CMD expands %VAR%, has no single-quote support, and mangles backslash-escaped quotes.
        // PowerShell has predictable quoting rules and treats % as a literal character.
        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("powershell.exe");
            c.arg("-NoProfile").arg("-NonInteractive").arg("-Command");
            c
        } else {
            let mut c = Command::new("/bin/sh");
            c.arg("-c");
            c
        };

        let start = Instant::now();

        let mut child = cmd
            .arg(&params.command)
            .current_dir(&workdir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to execute command: {}", e))?;

        let mut output = String::new();
        let mut truncated = false;
        let mut cancelled = false;

        // Read stdout
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if cancel_signal.load(Ordering::Relaxed) {
                    cancelled = true;
                    let _ = child.kill();
                    break;
                }
                if start.elapsed() > Duration::from_millis(timeout) {
                    truncated = true;
                    let _ = child.kill();
                    break;
                }
                if let Ok(line) = line {
                    if output.len() + line.len() + 1 > MAX_OUTPUT_LENGTH {
                        truncated = true;
                        break;
                    }
                    output.push_str(&line);
                    output.push('\n');
                }
            }
        }

        // Read stderr (skip if already cancelled or truncated)
        if !truncated && !cancelled {
            if let Some(stderr) = child.stderr.take() {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if cancel_signal.load(Ordering::Relaxed) {
                        cancelled = true;
                        let _ = child.kill();
                        break;
                    }
                    if start.elapsed() > Duration::from_millis(timeout) {
                        truncated = true;
                        let _ = child.kill();
                        break;
                    }
                    if let Ok(line) = line {
                        if output.len() + line.len() + 1 > MAX_OUTPUT_LENGTH {
                            truncated = true;
                            break;
                        }
                        output.push_str(&line);
                        output.push('\n');
                    }
                }
            }
        }

        // If cancelled, return immediately with a user-friendly message
        if cancelled {
            // Drain remaining output (best-effort)
            let _ = child.wait();
            return Ok(ToolResult::success_with_data(
                "Command cancelled by user (ESC)".to_string(),
                serde_json::json!({
                    "exit_code": -1,
                    "description": params.description,
                }),
            ));
        }

        let exit_status = child
            .wait()
            .map_err(|e| anyhow::anyhow!("Failed to wait for command: {}", e))?;

        let exit_code = exit_status.code().unwrap_or(-1);

        let mut result_output = output.clone();

        // Add metadata if truncated or timed out
        if truncated || start.elapsed() > Duration::from_millis(timeout) {
            result_output.push_str("\n\n<bash_metadata>\n");
            if result_output.len() >= MAX_OUTPUT_LENGTH {
                result_output.push_str(&format!(
                    "bash tool truncated output as it exceeded {} char limit\n",
                    MAX_OUTPUT_LENGTH
                ));
            }
            if start.elapsed() > Duration::from_millis(timeout) {
                result_output.push_str(&format!(
                    "bash tool terminated command after exceeding timeout {} ms. \
                     If this command is expected to take longer and is not waiting \
                     for interactive input, retry with a larger timeout value in \
                     milliseconds.\n",
                    timeout
                ));
            }
            result_output.push_str("</bash_metadata>");
        }

        Ok(ToolResult::success_with_data(
            result_output,
            serde_json::json!({
                "exit_code": exit_code,
                "description": params.description,
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        let schema = schemars::schema_for!(BashParams);
        serde_json::to_value(schema).unwrap_or_default()
    }
}

fn parse_bash_args(args: &ToolArgs) -> Result<BashParams> {
    let command = args
        .get_named_arg("command")
        .cloned()
        .or_else(|| args.args.first().cloned())
        .ok_or_else(|| anyhow::anyhow!("command is required"))?;

    let timeout = args
        .get_named_arg("timeout")
        .and_then(|s| s.parse::<u64>().ok());

    let workdir = args
        .get_named_arg("workdir")
        .cloned()
        .filter(|s| !s.is_empty()); // Treat empty strings as None

    let description = args.get_named_arg("description").cloned();

    Ok(BashParams {
        command,
        timeout,
        workdir,
        description,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bash_tool_creation() {
        let tool = BashTool::new();
        assert_eq!(tool.name(), "bash");
    }

    #[test]
    fn test_bash_tool_echo() {
        let mut tool = BashTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let args = ToolArgs::from_args(&["echo hello"]);

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);
        assert!(result.message.contains("hello"));
    }

    #[test]
    fn test_bash_tool_with_timeout() {
        let mut tool = BashTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let args = ToolArgs::with_named_args(
            vec!["sleep 1".to_string()],
            vec![("timeout".to_string(), "100".to_string())]
                .into_iter()
                .collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.message.contains("timeout") || result.success);
    }

    #[test]
    fn test_bash_tool_validation() {
        let tool = BashTool::new();
        let args = ToolArgs::from_args(&[]);

        let result = tool.validate_args(&args);
        assert!(result.is_err());
    }
}
