# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.12] - 2026-06-01

### Fixed
- **Grep/Glob Tool Silent Failures**: Fixed silent failures when LLM sends empty string values for `path` and `pattern` parameters
  - Made `path` parameter required (non-optional) in both `grep` and `glob` tools, matching the `list` tool pattern
  - Added explicit `grep` and `glob` handlers in `converter.rs` that filter out empty string values
  - Updated `validate_args` to check for non-empty pattern values (not just presence)
  - Updated `parse_grep_args` and `parse_glob_args` to filter empty patterns and paths with `.filter(|s| !s.is_empty())`
  - Error messages now clearly state "path parameter is required. Send '.' for current directory" instead of confusing "pattern argument" errors
  - Affects GLM-5 and GLM-5-Turbo models which were sending `{"path":"","pattern":"..."}` causing the pattern to be lost during JSON deserialization

## [0.1.11] - 2026-03-10

### Fixed
- **List Tool Path Parameter**: Made `path` parameter required to prevent GLM-5 from sending null/empty values
  - Previously, optional path parameter caused issues with certain LLMs (GLM-5) that would send null or empty strings
  - Path is now a required parameter, ensuring explicit directory specification
  - Improves reliability when used with various LLM providers

### Changed
- **List Tool Debug Logging**: Replaced `tracing` crate with simple `debug!` macro for debug logging
  - Reduces dependency overhead for basic debug output
  - Simplifies the codebase by removing unnecessary tracing infrastructure

## [0.1.10] - 2026-03-10

### Fixed
- **Bash Tool Empty Workdir Parameter Bug**: Empty string workdir parameters are now treated as `None` instead of being processed as valid paths
  - Previously, when LLM provided `{"workdir": ""}`, the tool would fail with "No such file or directory" error
  - Added `.filter(|s| !s.is_empty())` in `parse_bash_args()` to convert empty strings to `None`
  - This prevents command execution failures and agent confusion when empty workdir is provided
  - Related to fix in 0.1.7 for list tool - same pattern applied to bash tool

## [0.1.9] - 2026-03-10

### Fixed
- **CLI Tool Argument Parsing**: Fixed command-line interface to properly parse JSON arguments
  - CLI now parses JSON arguments from strings like `{"path":"/home/leo"}` and converts them to named args
  - Previously, JSON strings were treated as positional args, causing tools to ignore them
  - This fix enables proper CLI usage: `cats list '{"path":"/home/leo"}'`
  - Combined with list tool JSON converter handler for complete fix

## [0.1.8] - 2026-03-10

### Fixed
- **JSON Converter for List Tool**: Added explicit handler for `list` tool in JSON-to-ToolArgs conversion
  - Previously, the `list` tool fell into the default case which could cause parameter conversion issues
  - Now properly handles empty path strings by filtering them out (treating empty as None)
  - Ensures robust parameter handling when LLM calls the list tool via function calling APIs
  - Related to fix in 0.1.7 - provides defense-in-depth for empty path parameter handling

## [0.1.7] - 2026-03-10

### Fixed
- **List Tool Empty Path Parameter Bug**: Empty string path parameters are now treated as `None` instead of being processed as valid paths
  - Previously, when LLM provided `{"path": ""}`, the tool would list the current working directory instead of the intended directory
  - Added `.filter(|s| !s.is_empty())` in `parse_list_args()` to convert empty strings to `None`
  - This prevents agent confusion and repeated tool calls when empty paths are provided
  - Fixes issue where agent would appear "stuck" repeatedly listing the wrong directory

## [0.1.6] - 2026-02-27

### Changed
- **TLS Backend**: Switched from native-tls to rustls for cross-compilation support
  - `reqwest` now uses `rustls-tls-webpki-roots` instead of default native-tls
  - Enables static linking with musl for portable binaries
  - No OpenSSL dependency required

## [0.1.5] - 2026-02-23

### Fixed
- **Working Directory Resolution**: Tools now read working directory from `ToolState` at execution time instead of capturing it at instantiation
  - This fixes the issue where tools would use the wrong directory when invoked from a different context
  - Mirrors OpenCode's approach using async context
  - Affected tools: `bash`, `read`, `write`, `edit`, `multiedit`, `glob`, `grep`, `list`

### Added
- `ToolRegistry::set_working_directory()` - Set the working directory for all tools
- `ToolRegistry::get_working_directory()` - Get the current working directory from tool state

## [0.1.4] - 2026-02-21

### Changed
- **BREAKING**: Default feature changed from `old` to `opencode`
- `create_tool_registry_with_open_window_size()` now takes `Option<usize>` instead of `usize`

### Added
- OpenCode-compatible tool set is now the default:
  - `bash` - Execute shell commands
  - `read` - Read file contents with windowing
  - `write` - Write content to files
  - `edit` - Edit files with str_replace
  - `glob` - Find files by pattern
  - `grep` - Search file contents
  - `ls` - List directory contents
  - `webfetch` - Fetch web content (optional, requires `reqwest`)
  - `question` - Ask user questions
  - `task` - Launch sub-agents (placeholder)
  - `todo` - Todo list management
  - `patch` - Patch files (placeholder)
  - `lsp` - LSP integration (placeholder)
  - `skill` - Skill execution (placeholder)

### Removed
- `old` toolset is no longer the default (still available via `default = ["old"]` feature)

## [0.1.1] - 2025-10-29

### Fixed
- CLI now dynamically lists all available tools from registry instead of hardcoded subset
- Removed non-existent tools (`edit`, `insert`) from CLI help output
- Added missing tools to CLI help: `run_command`, all editing tools (`create_file`, `replace_text`, `insert_text`, `delete_text`, `delete_line`, `overwrite_file`, `delete_function`), file management tools (`delete_path`, `move_path`, `copy_path`, `create_directory`), and `classify_task`
- CLI help output now stays automatically synchronized with the tool registry

## [0.1.0] - 2025-10-26

### Added
- Initial public release of CATS (Coding Agent ToolS)
- Extracted from Simpaticoder monorepo as independent crate
- Complete tool suite for AI coding agents:
  - File Navigation tools: `open`, `goto`, `scroll_up`, `scroll_down`
  - Search tools: `find_file`, `search_file`, `search_dir`
  - Editing tools: `create_file`, `replace_text`, `insert_text`, `delete_text`, `delete_line`, `overwrite_file`, `delete_function`
  - File Management tools: `delete_path`, `move_path`, `copy_path`, `create_directory`
  - Execution tools: `run_command`
  - Utility tools: `_state`, `filemap`, `submit`, `classify_task`
  - Optional `count_tokens` tool (requires `tiktoken` feature)
- Comprehensive documentation and examples
- Dual-license: MIT OR Apache-2.0
- CI/CD pipeline for automated testing
- Cross-platform support (Linux, macOS, Windows)

### Changed
- Renamed from `simpaticoder-tools` to `cats`
- Updated all internal references and documentation
- Published to crates.io as standalone crate
- Repository: https://github.com/podtan/cats

### Notes
- This is the first independent release
- Formerly part of the Simpaticoder project
- API surface is stable and production-ready
- No backward compatibility with `simpaticoder-tools` crate name

[Unreleased]: https://github.com/podtan/cats/compare/v0.1.12...HEAD
[0.1.12]: https://github.com/podtan/cats/compare/v0.1.11...v0.1.12
[0.1.11]: https://github.com/podtan/cats/compare/v0.1.10...v0.1.11
[0.1.10]: https://github.com/podtan/cats/compare/v0.1.9...v0.1.10
[0.1.9]: https://github.com/podtan/cats/compare/v0.1.8...v0.1.9
[0.1.8]: https://github.com/podtan/cats/compare/v0.1.7...v0.1.8
[0.1.7]: https://github.com/podtan/cats/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/podtan/cats/releases/tag/v0.1.6
[0.1.5]: https://github.com/podtan/cats/releases/tag/v0.1.5
[0.1.4]: https://github.com/podtan/cats/releases/tag/v0.1.4
[0.1.1]: https://github.com/podtan/cats/releases/tag/v0.1.1
[0.1.0]: https://github.com/podtan/cats/releases/tag/v0.1.0
