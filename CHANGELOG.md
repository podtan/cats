# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.20] - 2026-04-29

### Fixed
- **executor: compact log shows `__truncated` marker instead of `{}`** — when tool call
  arguments fail JSON parsing (e.g. truncated by max_tokens), the compact log now records
  `{"__truncated": true, "__len": N}` so the log clearly shows a truncation occurred
  rather than silently displaying empty arguments.
- **executor: EOF parse errors produce actionable message** — when the LLM response is
  truncated mid-JSON (detected via `EOF while parsing`), the error returned to the LLM now
  reads: `"tool call was truncated (response too large — N bytes). Break the content into
  smaller pieces or use bash with heredoc."` Applies to all tools (`write`, `bash`,
  `edit`, etc.) via both `execute_tool_calls` and `execute_tool_calls_structured`.

## [0.1.19] - 2026-04-29

### Fixed
- **multiedit: atomic failure message** — when one edit in a `multiedit` batch fails, the error now
  explicitly states that no changes were written to disk and instructs the LLM to re-submit all edits
  with a corrected `oldString` rather than assuming earlier edits were applied.
- **edit/multiedit: oldString not found hint** — error message now tells the LLM to use the `read`
  tool to fetch exact current file content instead of constructing `oldString` from memory.

## [0.1.18] - 2026-04-29

### Fixed
- **grep: panic on multibyte characters at line truncation boundary** — `&line[..MAX_LINE_LENGTH]`
  sliced by byte index. Now uses `char_indices()` to find the nearest valid char boundary.
- **result_handler: panic on multibyte characters at result truncation boundary** — `&result_message[..max_size_bytes]`
  sliced by byte index. Now uses `char_indices()` to find the nearest valid char boundary.

## [0.1.17] - 2026-04-29

### Fixed
- **read: panic on multibyte characters at line truncation boundary** — `&line[..MAX_LINE_LENGTH]`
  sliced by byte index which could fall inside a multibyte UTF-8 character (e.g. em dash `—`).
  Now uses `char_indices()` to find the nearest valid char boundary before truncating.

## [0.1.16] - 2026-04-26

### Fixed
- **edit/read/write/multiedit: camelCase field names rejected by validate_args** — `filePath`, `oldString`,
  `newString` were silently blocked before reaching the `parse_*_args` functions that already handled them.
  Now `validate_args` accepts both snake_case and camelCase variants for all four tools.
- **multiedit: camelCase fields in edits array** — `EditOperation` deserialization failed with
  `missing field 'old_string'` when LLMs sent `oldString`/`newString`/`replaceAll`. Added
  `#[serde(alias)]` attributes to accept both naming conventions.
- **edit: CRLF line-ending corruption in fallback strategies** — `line_trimmed_replacer`,
  `whitespace_normalized_replacer`, and `trimmed_boundary_replacer` used `str::lines()` (strips `\r`)
  then reconstructed blocks with `join("\n")`, making `str::find` fail on CRLF files. Switched to
  `split('\n')` throughout so `\r` is preserved in reconstructed substrings.
- **edit: multiple-match early exit blocked fallback strategies** — `replace_in_content` returned
  `Err("Found multiple matches")` immediately when the exact string appeared more than once, before
  trying any fuzzy fallback. The error is now only raised after all strategies are exhausted without
  finding a unique match (mirrors OpenCode behaviour).
- **read: offset was 0-based instead of 1-based** — LLMs trained on OpenCode expect `offset=1` to
  mean "start at line 1", but cats treated it as "skip 1 line". Changed to 1-based: `skip = offset - 1`.

### Added
- **edit: EscapeNormalizedReplacer** — new fallback strategy that unescapes literal `\n`, `\t`, `\r`, `\"`
  sequences LLMs sometimes emit due to JSON double-escaping, then retries the replacement.
- **edit: IndentationFlexibleReplacer** — new fallback strategy that strips minimum common indentation
  from both the search string and candidate blocks before comparing, tolerating 2-space vs 4-space
  mismatches and copy-paste indentation drift.
- **edit: BlockAnchorReplacer** — new fallback strategy that locates blocks by matching trimmed
  first/last lines (anchors), then scores middle-line similarity using Levenshtein distance (via the
  existing `edit-distance` crate). Accepts the best candidate above a 30% threshold when multiple
  anchor matches exist, enabling fuzzy matching on slightly-stale code blocks.

## [0.1.15] - 2026-04-15

### Added
- **Cooperative Cancellation for Tool Execution**: Added `CancelSignal` (Arc<AtomicBool>) infrastructure so running tools (especially bash) can be killed immediately when the user presses ESC
  - New `CancelSignal` struct with `request_cancel()`, `reset()`, and `is_cancelled()` methods
  - Added `cancel_signal` field to `ToolState` with accessor methods (`request_cancel()`, `reset_cancel()`, `is_cancelled()`, `cancel_signal()`)
  - Re-exported `CancelSignal` from `lib.rs` for downstream consumers
  - BashTool now clones the cancel signal before spawning child processes to avoid holding the Mutex lock
  - Added cancel checks in BashTool stdout and stderr read loops — on cancel, kills child process and returns early
  - Added cancel check before starting stderr read (skip if already cancelled)
  - On cancel, returns a user-friendly result: "Command cancelled by user (ESC)" with exit_code -1

## [0.1.14] - 2026-04-15

### Fixed
- **Edit Tool OOB Panics**: Prevented out-of-bounds panics in `whitespace_normalized_replacer` and `trimmed_boundary_replacer` when dealing with edge cases in text replacement
- **Edit Tool Empty Lines Panic**: Prevented panic in `line_trimmed_replacer` when search_lines becomes empty after trailing newline trim

## [0.1.13] - 2026-04-15

### Fixed
- **Grep/Glob Tool Silent Failures**: Fixed silent failures when LLM sends empty string values for `path` and `pattern` parameters
  - Made `path` parameter required (non-optional) in both `grep` and `glob` tools, matching the `list` tool pattern
  - Added explicit `grep` and `glob` handlers in `converter.rs` that filter out empty string values
  - Updated `validate_args` to check for non-empty pattern values (not just presence)
  - Updated `parse_grep_args` and `parse_glob_args` to filter empty patterns and paths with `.filter(|s| !s.is_empty())`
  - Error messages now clearly state "path parameter is required. Send '.' for current directory" instead of confusing "pattern argument" errors
  - Affects GLM-5 and GLM-5-Turbo models which were sending `{"path":"","pattern":"..."}` causing the pattern to be lost during JSON deserialization

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

[Unreleased]: https://github.com/podtan/cats/compare/v0.1.15...HEAD
[0.1.15]: https://github.com/podtan/cats/compare/v0.1.14...v0.1.15
[0.1.14]: https://github.com/podtan/cats/compare/v0.1.13...v0.1.14
[0.1.13]: https://github.com/podtan/cats/compare/v0.1.12...v0.1.13
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
