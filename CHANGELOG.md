# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/podtan/cats/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/podtan/cats/releases/tag/v0.1.0
