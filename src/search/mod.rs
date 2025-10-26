//! Search tools for file discovery and content search

use crate::core::{Tool, ToolArgs, ToolError, ToolResult};
use crate::state::ToolState;
use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;

pub mod filtering;
pub use filtering::ConfigurableFilter;

/// Maximum number of search results to return
const MAX_SEARCH_RESULTS: usize = 1000;

/// Search match result
#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub file: PathBuf,
    pub line_number: usize,
    pub line_content: String,
    pub match_start: usize,
    pub match_end: usize,
}

/// Tool for finding files by name or pattern
pub struct FindFileTool {
    name: String,
}

impl FindFileTool {
    pub fn new() -> Self {
        Self {
            name: "find_file".to_string(),
        }
    }

    /// Convert glob pattern to regex for more flexible matching
    fn glob_to_regex(pattern: &str) -> Result<Regex, ToolError> {
        let mut regex_pattern = String::new();
        let mut chars = pattern.chars().peekable();

        regex_pattern.push('^');

        while let Some(c) = chars.next() {
            match c {
                '*' => {
                    if chars.peek() == Some(&'*') {
                        chars.next(); // consume second *
                        if chars.peek() == Some(&'/') {
                            chars.next(); // consume /
                            regex_pattern.push_str("(?:.*/)?");
                        } else {
                            regex_pattern.push_str(".*");
                        }
                    } else {
                        regex_pattern.push_str("[^/]*");
                    }
                }
                '?' => regex_pattern.push_str("[^/]"),
                '[' => {
                    regex_pattern.push('[');
                    while let Some(c) = chars.next() {
                        if c == ']' {
                            regex_pattern.push(']');
                            break;
                        }
                        regex_pattern.push(c);
                    }
                }
                '.' | '^' | '$' | '(' | ')' | '{' | '}' | '|' | '+' | '\\' => {
                    regex_pattern.push('\\');
                    regex_pattern.push(c);
                }
                _ => regex_pattern.push(c),
            }
        }

        regex_pattern.push('$');
        Regex::new(&regex_pattern).map_err(ToolError::from)
    }
}

impl Tool for FindFileTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Finds all files with the given name or pattern in dir. If dir is not provided, searches in the current directory"
    }

    fn signature(&self) -> &str {
        "find_file <file_name> [<dir>]"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        if args.is_empty() {
            return Err(ToolError::InvalidArgs {
                message: "Usage: find_file <file_name> [<dir>]".to_string(),
            });
        }
        Ok(())
    }

    fn execute(&mut self, args: &ToolArgs, _state: &Arc<Mutex<ToolState>>) -> Result<ToolResult> {
        let pattern = args.get_arg(0).unwrap();
        let default_dir = "./".to_string();
        let search_dir = args.get_arg(1).unwrap_or(&default_dir);
        let search_path = Path::new(search_dir);

        // Check if directory exists
        if !search_path.exists() {
            return Ok(ToolResult::error(format!(
                "Directory {} not found",
                search_dir
            )));
        }

        if !search_path.is_dir() {
            return Ok(ToolResult::error(format!(
                "{} is not a directory",
                search_dir
            )));
        }

        // Convert glob pattern to regex
        let regex = Self::glob_to_regex(pattern)?;

        // Search for matching files
        let mut matches = Vec::new();
        let filter = ConfigurableFilter::new(None);
        let walker = WalkDir::new(search_path)
            .max_depth(100) // Reasonable depth limit
            .follow_links(false)
            .into_iter()
            // Only apply filtering to directories (control descent). Always allow files through
            // Always allow the search root to avoid pruning the entire search tree when it
            // matches exclusion rules (e.g., temp dirs that start with a dot).
            .filter_entry(|e| {
                let path = e.path();
                if path == search_path {
                    return true;
                }
                if path.is_dir() {
                    filter.should_include_path(path)
                } else {
                    true
                }
            });

        for entry in walker {
            match entry {
                Ok(entry) => {
                    if entry.file_type().is_file() {
                        let file_name = entry.file_name().to_string_lossy();
                        let relative_path = entry
                            .path()
                            .strip_prefix(search_path)
                            .unwrap_or(entry.path());
                        let path_str = relative_path.to_string_lossy();

                        // Match against both filename and relative path
                        if regex.is_match(&file_name) || regex.is_match(&path_str) {
                            matches.push(entry.path().to_path_buf());

                            // Prevent excessive results
                            if matches.len() >= MAX_SEARCH_RESULTS {
                                break;
                            }
                        }
                    }
                }
                Err(_) => continue, // Skip inaccessible files
            }
        }

        if matches.is_empty() {
            return Ok(ToolResult::success(format!(
                "No matches found for \"{}\" in {}",
                pattern,
                search_path.display()
            )));
        }

        // Sort results
        matches.sort();

        // Format results
        let mut result_text = format!(
            "Found {} matches for \"{}\" in {}:\n",
            matches.len(),
            pattern,
            search_path.display()
        );

        for path in &matches {
            result_text.push_str(&format!("{}\n", path.display()));
        }

        Ok(ToolResult::success_with_data(
            result_text.trim().to_string(),
            serde_json::json!({
                "pattern": pattern,
                "search_dir": search_dir,
                "matches": matches.iter().map(|p| p.to_string_lossy()).collect::<Vec<_>>(),
                "count": matches.len()
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_name": {
                    "type": "string",
                    "description": "The name of the file or pattern to search for. Supports shell-style wildcards (e.g., *.py, **/*.rs)"
                },
                "dir": {
                    "type": "string",
                    "description": "The directory to search in (if not provided, searches in the current directory)",
                    "default": "./"
                }
            },
            "required": ["file_name"]
        })
    }
}

/// Tool for searching content within a specific file
pub struct SearchFileTool {
    name: String,
}

impl SearchFileTool {
    pub fn new() -> Self {
        Self {
            name: "search_file".to_string(),
        }
    }

    fn search_in_file(
        &self,
        search_term: &str,
        file_path: &Path,
    ) -> Result<Vec<SearchMatch>, ToolError> {
        let content = fs::read_to_string(file_path).map_err(|_| ToolError::FileNotFound {
            path: file_path.to_string_lossy().to_string(),
        })?;

        let mut matches = Vec::new();
        // Treat user input as a literal string by default to avoid parse errors
        // when the caller provides unescaped regex metacharacters. If callers
        // want real regex behavior in future, add an explicit flag.
        let escaped = regex::escape(search_term);
        let regex = Regex::new(&escaped).map_err(ToolError::from)?;

        for (line_num, line) in content.lines().enumerate() {
            for mat in regex.find_iter(line) {
                matches.push(SearchMatch {
                    file: file_path.to_path_buf(),
                    line_number: line_num + 1,
                    line_content: line.to_string(),
                    match_start: mat.start(),
                    match_end: mat.end(),
                });

                // Limit matches per file
                if matches.len() >= MAX_SEARCH_RESULTS {
                    return Ok(matches);
                }
            }
        }

        Ok(matches)
    }
}

impl Tool for SearchFileTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Searches for search_term in file. If file is not provided, searches in the current open file"
    }

    fn signature(&self) -> &str {
        "search_file <search_term> [<file>]"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        if args.is_empty() {
            return Err(ToolError::InvalidArgs {
                message: "Usage: search_file <search_term> [<file>]".to_string(),
            });
        }
        Ok(())
    }

    fn execute(&mut self, args: &ToolArgs, state: &Arc<Mutex<ToolState>>) -> Result<ToolResult> {
        let search_term = args.get_arg(0).unwrap();

        let target_file = if let Some(file_arg) = args.get_arg(1) {
            PathBuf::from(file_arg)
        } else {
            // Use current open file
            let state_guard = state
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock state: {}", e))?;
            state_guard
                .current_file
                .clone()
                .ok_or_else(|| anyhow::anyhow!("No file specified and no file is currently open"))?
        };

        // Check if file exists
        if !target_file.exists() {
            return Ok(ToolResult::error(format!(
                "File not found: {}",
                target_file.display()
            )));
        }

        // Search in the file
        let matches = self.search_in_file(search_term, &target_file)?;

        if matches.is_empty() {
            return Ok(ToolResult::success(format!(
                "No matches found for \"{}\" in {}",
                search_term,
                target_file.display()
            )));
        }

        // Format results
        let mut result_text = format!(
            "Found {} matches for \"{}\" in {}:\n\n",
            matches.len(),
            search_term,
            target_file.display()
        );

        for (i, m) in matches.iter().enumerate() {
            result_text.push_str(&format!(
                "{}. Line {}: {}\n",
                i + 1,
                m.line_number,
                m.line_content.trim()
            ));

            // Add visual indicator of match position
            let indent = format!("{}. Line {}: ", i + 1, m.line_number);
            let spaces = " ".repeat(indent.len() + m.match_start);
            let highlight = "^".repeat(m.match_end - m.match_start);
            result_text.push_str(&format!("{}{}\n\n", spaces, highlight));
        }

        Ok(ToolResult::success_with_data(
            result_text.trim().to_string(),
            serde_json::json!({
                "search_term": search_term,
                "file": target_file.to_string_lossy(),
                "matches": matches.iter().map(|m| {
                    serde_json::json!({
                        "line_number": m.line_number,
                        "line_content": m.line_content,
                        "match_start": m.match_start,
                        "match_end": m.match_end
                    })
                }).collect::<Vec<_>>(),
                "count": matches.len()
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "search_term": {
                    "type": "string",
                    "description": "The term to search for (supports regex patterns)"
                },
                "file": {
                    "type": "string",
                    "description": "The file to search in (if not provided, searches in the current open file)"
                }
            },
            "required": ["search_term"]
        })
    }
}

/// Tool for searching content across directories
pub struct SearchDirTool {
    name: String,
}

impl SearchDirTool {
    pub fn new() -> Self {
        Self {
            name: "search_dir".to_string(),
        }
    }

    fn search_in_directory(
        &self,
        search_term: &str,
        dir_path: &Path,
    ) -> Result<HashMap<PathBuf, Vec<SearchMatch>>, ToolError> {
        // Treat user input as a literal string by default to avoid parse errors
        let escaped = regex::escape(search_term);
        let regex = Regex::new(&escaped).map_err(ToolError::from)?;
        let mut all_matches = HashMap::new();
        let mut total_matches = 0;

        let filter = ConfigurableFilter::new(None);
        let walker = WalkDir::new(dir_path)
            .max_depth(100)
            .follow_links(false)
            .into_iter()
            // Only apply filtering to directories (control descent). Always allow files through
            // Always allow the search root to avoid pruning the entire search tree when it
            // matches exclusion rules (e.g., temp dirs that start with a dot).
            .filter_entry(|e| {
                let path = e.path();
                if path == dir_path {
                    return true;
                }
                if path.is_dir() {
                    filter.should_include_path(path)
                } else {
                    true
                }
            });

        for entry in walker {
            if total_matches >= MAX_SEARCH_RESULTS {
                break;
            }

            match entry {
                Ok(entry) => {
                    if entry.file_type().is_file() {
                        let path = entry.path();
                        // Use configurable filter to skip files based on extension/hidden/etc.
                        if !filter.should_include_path(path) {
                            continue;
                        }

                        // Try to read file content
                        if let Ok(content) = fs::read_to_string(path) {
                            let mut file_matches = Vec::new();

                            for (line_num, line) in content.lines().enumerate() {
                                for mat in regex.find_iter(line) {
                                    file_matches.push(SearchMatch {
                                        file: path.to_path_buf(),
                                        line_number: line_num + 1,
                                        line_content: line.to_string(),
                                        match_start: mat.start(),
                                        match_end: mat.end(),
                                    });

                                    total_matches += 1;
                                    if total_matches >= MAX_SEARCH_RESULTS {
                                        break;
                                    }
                                }

                                if total_matches >= MAX_SEARCH_RESULTS {
                                    break;
                                }
                            }

                            if !file_matches.is_empty() {
                                all_matches.insert(path.to_path_buf(), file_matches);
                            }
                        }
                    }
                }
                Err(_) => continue, // Skip inaccessible files
            }
        }

        Ok(all_matches)
    }
}

impl Tool for SearchDirTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Searches for search_term in all files in dir. If dir is not provided, searches in the current directory"
    }

    fn signature(&self) -> &str {
        "search_dir <search_term> [<dir>]"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        if args.is_empty() {
            return Err(ToolError::InvalidArgs {
                message: "Usage: search_dir <search_term> [<dir>]".to_string(),
            });
        }
        Ok(())
    }

    fn execute(&mut self, args: &ToolArgs, _state: &Arc<Mutex<ToolState>>) -> Result<ToolResult> {
        let search_term = args.get_arg(0).unwrap();
        let default_dir = "./".to_string();
        let search_dir = args.get_arg(1).unwrap_or(&default_dir);
        let search_path = Path::new(search_dir);

        // Check if directory exists
        if !search_path.exists() {
            return Ok(ToolResult::error(format!(
                "Directory {} not found",
                search_dir
            )));
        }

        if !search_path.is_dir() {
            return Ok(ToolResult::error(format!(
                "{} is not a directory",
                search_dir
            )));
        }

        // Search in directory
        let matches = self.search_in_directory(search_term, search_path)?;

        if matches.is_empty() {
            return Ok(ToolResult::success(format!(
                "No matches found for \"{}\" in {}",
                search_term,
                search_path.display()
            )));
        }

        // Count total matches
        let total_matches: usize = matches.values().map(|v| v.len()).sum();

        // Format results
        let mut result_text = format!(
            "Found {} matches for \"{}\" in {} across {} files:\n\n",
            total_matches,
            search_term,
            search_path.display(),
            matches.len()
        );

        // Sort files by number of matches (descending)
        let mut file_matches: Vec<_> = matches.iter().collect();
        file_matches.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

        for (file_path, file_matches) in file_matches.iter().take(20) {
            // Limit to top 20 files
            result_text.push_str(&format!(
                "📁 {} ({} matches):\n",
                file_path.display(),
                file_matches.len()
            ));

            for (i, m) in file_matches.iter().take(5).enumerate() {
                // Limit to 5 matches per file
                result_text.push_str(&format!(
                    "  {}. Line {}: {}\n",
                    i + 1,
                    m.line_number,
                    m.line_content.trim()
                ));
            }

            if file_matches.len() > 5 {
                result_text.push_str(&format!(
                    "  ... and {} more matches\n",
                    file_matches.len() - 5
                ));
            }

            result_text.push('\n');
        }

        if matches.len() > 20 {
            result_text.push_str(&format!(
                "... and {} more files with matches\n",
                matches.len() - 20
            ));
        }

        // Prepare structured data
        let mut files_data = Vec::new();
        for (file_path, file_matches) in matches.iter() {
            files_data.push(serde_json::json!({
                "file": file_path.to_string_lossy(),
                "match_count": file_matches.len(),
                "matches": file_matches.iter().take(10).map(|m| {
                    serde_json::json!({
                        "line_number": m.line_number,
                        "line_content": m.line_content,
                        "match_start": m.match_start,
                        "match_end": m.match_end
                    })
                }).collect::<Vec<_>>()
            }));
        }

        Ok(ToolResult::success_with_data(
            result_text.trim().to_string(),
            serde_json::json!({
                "search_term": search_term,
                "search_dir": search_dir,
                "total_matches": total_matches,
                "file_count": matches.len(),
                "files": files_data
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "search_term": {
                    "type": "string",
                    "description": "The term to search for (supports regex patterns)"
                },
                "dir": {
                    "type": "string",
                    "description": "The directory to search in (if not provided, searches in the current directory)",
                    "default": "./"
                }
            },
            "required": ["search_term"]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let file_path = dir.path().join(name);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = fs::File::create(&file_path).unwrap();
        write!(file, "{}", content).unwrap();
        file_path
    }

    #[test]
    fn test_find_file_tool() {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(&temp_dir, "test.txt", "content");
        create_test_file(&temp_dir, "test.rs", "rust content");
        create_test_file(&temp_dir, "subdir/test.py", "python content");

        let mut tool = FindFileTool::new();
        let state = Arc::new(Mutex::new(ToolState::new()));

        // Test finding all .txt files
        let args = ToolArgs::from_args(&["*.txt", temp_dir.path().to_str().unwrap()]);
        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);
        assert!(result.message.contains("test.txt"));

        // Test finding files in subdirectories
        let args = ToolArgs::from_args(&["*.py", temp_dir.path().to_str().unwrap()]);
        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);
        assert!(result.message.contains("test.py"));
    }

    #[test]
    fn test_search_file_tool() {
        let temp_dir = TempDir::new().unwrap();
        let content = "line 1\ntest content here\nline 3 with test\nfinal line";
        let file_path = create_test_file(&temp_dir, "search_test.txt", content);

        let mut tool = SearchFileTool::new();
        let state = Arc::new(Mutex::new(ToolState::new()));

        let args = ToolArgs::from_args(&["test", file_path.to_str().unwrap()]);
        let result = tool.execute(&args, &state).unwrap();

        assert!(result.success);
        assert!(result.message.contains("2 matches"));
        assert!(result.message.contains("Line 2"));
        assert!(result.message.contains("Line 3"));
    }

    #[test]
    fn test_search_dir_tool() {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(&temp_dir, "file1.txt", "this has foo in it");
        create_test_file(&temp_dir, "file2.rs", "no matches here");
        create_test_file(&temp_dir, "subdir/file3.py", "foo is here too");

        let mut tool = SearchDirTool::new();
        let state = Arc::new(Mutex::new(ToolState::new()));

        let args = ToolArgs::from_args(&["foo", temp_dir.path().to_str().unwrap()]);
        let result = tool.execute(&args, &state).unwrap();

        assert!(result.success);
        assert!(result.message.contains("2 matches"));
        assert!(result.message.contains("2 files"));
    }

    #[test]
    fn test_glob_to_regex() {
        let regex = FindFileTool::glob_to_regex("*.rs").unwrap();
        assert!(regex.is_match("test.rs"));
        assert!(!regex.is_match("test.txt"));

        let regex = FindFileTool::glob_to_regex("**/*.py").unwrap();
        assert!(regex.is_match("subdir/test.py"));
        assert!(regex.is_match("test.py"));

        let regex = FindFileTool::glob_to_regex("test?.txt").unwrap();
        assert!(regex.is_match("test1.txt"));
        assert!(regex.is_match("testa.txt"));
        assert!(!regex.is_match("test12.txt"));
    }

    #[test]
    fn test_find_file_respects_exclude_dirs() {
        let temp_dir = TempDir::new().unwrap();
        // Create an excluded directory and a visible file
        create_test_file(&temp_dir, "node_modules/hidden.txt", "secret");
        create_test_file(&temp_dir, "visible.txt", "visible content");

        let mut tool = FindFileTool::new();
        let state = Arc::new(Mutex::new(ToolState::new()));

        let args = ToolArgs::from_args(&["*.txt", temp_dir.path().to_str().unwrap()]);
        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);
        // Should find visible.txt but not node_modules/hidden.txt
        assert!(result.message.contains("visible.txt"));
        assert!(!result.message.contains("node_modules/hidden.txt"));
    }

    #[test]
    fn test_search_dir_skips_excluded_extensions() {
        let temp_dir = TempDir::new().unwrap();
        // Create an excluded-extension file and a normal text file, both containing the term
        create_test_file(&temp_dir, "binary.exe", "secret_term");
        create_test_file(&temp_dir, "file.txt", "secret_term");

        let mut tool = SearchDirTool::new();
        let state = Arc::new(Mutex::new(ToolState::new()));

        let args = ToolArgs::from_args(&["secret_term", temp_dir.path().to_str().unwrap()]);
        let result = tool.execute(&args, &state).unwrap();
        assert!(result.success);
        // Should find only the .txt occurrence
        assert!(result.message.contains("file.txt"));
        assert!(!result.message.contains("binary.exe"));
    }
}
