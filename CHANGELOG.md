# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
