//! List tool implementation compatible with OpenCode
//!
//! Lists directory contents with tree-like output.

use crate::core::{Tool, ToolArgs, ToolError, ToolResult};

macro_rules! debug {
    ($($arg:tt)*) => {
        if std::env::var("RUST_LOG").map(|v| v.to_lowercase().contains("debug")).unwrap_or(false) {
            eprintln!("[DEBUG LIST] {}", format!($($arg)*));
        }
    };
}
use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;

const LIMIT: usize = 100;

/// Default ignore patterns
const IGNORE_PATTERNS: &[&str] = &[
    "node_modules",
    "__pycache__",
    ".git",
    "dist",
    "build",
    "target",
    "vendor",
    "bin",
    "obj",
    ".idea",
    ".vscode",
    ".zig-cache",
    "zig-out",
    ".coverage",
    "coverage",
    "tmp",
    "temp",
    ".cache",
    "cache",
    "logs",
    ".venv",
    "venv",
    "env",
];

/// List tool parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListParams {
    /// The absolute path to the directory to list (must be absolute, not relative)
    pub path: Option<String>,
    /// List of glob patterns to ignore
    pub ignore: Option<Vec<String>>,
}

/// List tool for directory listing
pub struct ListTool {
    name: String,
}

impl ListTool {
    pub fn new() -> Self {
        Self {
            name: "list".to_string(),
        }
    }
}

impl Default for ListTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for ListTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Reads a directory from the local filesystem"
    }

    fn signature(&self) -> &str {
        "list [--path <directory>] [--ignore <pattern>]"
    }

    fn validate_args(&self, _args: &ToolArgs) -> Result<(), ToolError> {
        Ok(())
    }

    fn execute(
        &mut self,
        args: &ToolArgs,
        state: &Arc<Mutex<crate::state::ToolState>>,
    ) -> Result<ToolResult> {
        debug!("execute called with args: {:?}", args);
        let params = parse_list_args(args)?;
        debug!("parsed params: path={:?}, ignore={:?}", params.path, params.ignore);

        // Get working directory from ToolState at execution time
        let working_dir = state
            .lock()
            .map(|s| s.working_directory.clone())
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
        debug!("working_dir: {:?}", working_dir);

        let search_path = params
            .path
            .map(PathBuf::from)
            .unwrap_or_else(|| working_dir.clone());
        debug!("initial search_path: {:?}", search_path);

        let search_path = if search_path.is_absolute() {
            search_path
        } else {
            working_dir.join(&search_path)
        };
        debug!("resolved search_path: {:?}", search_path);

        if !search_path.exists() {
            debug!("path does not exist: {:?}", search_path);
            return Err(ToolError::FileNotFound {
                path: search_path.display().to_string(),
            }
            .into());
        }

        if !search_path.is_dir() {
            debug!("path is not a directory: {:?}", search_path);
            return Err(anyhow::anyhow!(
                "Path is not a directory: {}",
                search_path.display()
            ));
        }

        // Build ignore set
        let mut ignore_set: HashSet<String> =
            IGNORE_PATTERNS.iter().map(|s| s.to_string()).collect();
        if let Some(ignore) = &params.ignore {
            for pattern in ignore {
                ignore_set.insert(pattern.clone());
            }
        }
        debug!("ignore_set has {} patterns", ignore_set.len());

        // Collect files
        let mut files: Vec<String> = Vec::new();

        for entry in WalkDir::new(&search_path)
            .follow_links(false)
            .min_depth(1)
            .max_depth(10)
            .into_iter()
            .filter_entry(|e| {
                // Filter out ignored directories
                if e.file_type().is_dir() {
                    if let Some(name) = e.file_name().to_str() {
                        return !ignore_set.contains(name);
                    }
                }
                true
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            if files.len() >= LIMIT {
                break;
            }

            let relative = entry
                .path()
                .strip_prefix(&search_path)
                .unwrap_or(entry.path());

            files.push(relative.display().to_string());
        }

        // Build directory structure
        let mut dirs: HashSet<String> = HashSet::new();
        let mut files_by_dir: HashMap<String, Vec<String>> = HashMap::new();

        for file in &files {
            let parent = Path::new(file)
                .parent()
                .and_then(|p| p.to_str())
                .filter(|s| !s.is_empty())
                .unwrap_or(".")
                .to_string();

            // Add all parent directories
            let parts: Vec<&str> = parent.split('/').filter(|p| !p.is_empty()).collect();
            dirs.insert(".".to_string()); // Always include root
            for i in 0..parts.len() {
                let dir = parts[..=i].join("/");
                dirs.insert(dir);
            }

            // Add file to its directory
            let filename = Path::new(file)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            files_by_dir.entry(parent).or_default().push(filename);
        }

        // Render directory tree
        let output = format!(
            "{}\n{}",
            search_path.display(),
            render_dir(".", &dirs, &files_by_dir, 0)
        );
        debug!("found {} files, truncated: {}", files.len(), files.len() >= LIMIT);

        Ok(ToolResult::success_with_data(
            output,
            serde_json::json!({
                "count": files.len(),
                "truncated": files.len() >= LIMIT,
                "path": search_path.display().to_string(),
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        let schema = schemars::schema_for!(ListParams);
        serde_json::to_value(schema).unwrap_or_default()
    }
}

fn parse_list_args(args: &ToolArgs) -> Result<ListParams> {
    let path = args
        .get_named_arg("path")
        .cloned()
        .filter(|s| !s.is_empty()); // Treat empty strings as None

    let ignore = args
        .get_named_arg("ignore")
        .map(|s| s.split(',').map(|s| s.trim().to_string()).collect());

    Ok(ListParams { path, ignore })
}

fn render_dir(
    dir_path: &str,
    dirs: &HashSet<String>,
    files_by_dir: &HashMap<String, Vec<String>>,
    depth: usize,
) -> String {
    let mut output = String::new();

    // Get child directories (immediate children only)
    let mut children: Vec<&str> = dirs
        .iter()
        .filter(|d| {
            if **d == "." {
                return false;
            }
            let parent = Path::new(*d)
                .parent()
                .and_then(|p| p.to_str())
                .filter(|s| !s.is_empty())
                .unwrap_or(".");
            parent == dir_path
        })
        .map(|s| s.as_str())
        .collect();
    children.sort();

    let indent = "  ".repeat(depth);

    // Render subdirectories
    for child in children {
        let basename = Path::new(child)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(child);

        output.push_str(&format!("{}{}/\n", indent, basename));
        output.push_str(&render_dir(child, dirs, files_by_dir, depth + 1));
    }

    // Render files
    let file_indent = "  ".repeat(depth);
    if let Some(files) = files_by_dir.get(dir_path) {
        let mut sorted_files = files.clone();
        sorted_files.sort();

        for file in sorted_files {
            output.push_str(&format!("{}{}\n", file_indent, file));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_list_tool_creation() {
        let tool = ListTool::new();
        assert_eq!(tool.name(), "list");
    }

    #[test]
    fn test_list_tool_directory() {
        let temp_dir = TempDir::new().unwrap();

        // Create some test files
        fs::write(temp_dir.path().join("file1.txt"), "").unwrap();
        fs::write(temp_dir.path().join("file2.txt"), "").unwrap();
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();
        fs::write(temp_dir.path().join("subdir").join("file3.txt"), "").unwrap();

        let mut tool = ListTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let args = ToolArgs::with_named_args(
            vec![],
            vec![(
                "path".to_string(),
                temp_dir.path().to_str().unwrap().to_string(),
            )]
            .into_iter()
            .collect(),
        );

        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);
        assert!(result.message.contains("file1.txt"));
        assert!(result.message.contains("file2.txt"));
        assert!(result.message.contains("subdir/"));
    }

    #[test]
    fn test_list_tool_not_found() {
        let mut tool = ListTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let args = ToolArgs::with_named_args(
            vec![],
            vec![("path".to_string(), "/nonexistent/directory".to_string())]
                .into_iter()
                .collect(),
        );

        let result = tool.execute(&args, &state);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_tool_file_not_dir() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        fs::write(&file_path, "content").unwrap();

        let mut tool = ListTool::new();
        let state = Arc::new(Mutex::new(crate::state::ToolState::new()));
        let args = ToolArgs::with_named_args(
            vec![],
            vec![("path".to_string(), file_path.to_str().unwrap().to_string())]
                .into_iter()
                .collect(),
        );

        let result = tool.execute(&args, &state);
        assert!(result.is_err());
    }
}
