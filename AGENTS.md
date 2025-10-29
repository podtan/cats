# AGENTS.md

This file provides guidance to AI coding agents when working with the CATS (Coding Agent ToolS) crate.

## Project Overview

CATS is a Rust library that provides a comprehensive toolkit for building AI-powered coding agents. It offers structured, LLM-friendly tools for file manipulation, code editing, search, and execution. The crate is published on crates.io and designed to be used as a library dependency or standalone CLI tool.

## Setup Commands

```bash
# Install dependencies
cargo build

# Run tests
cargo test

# Run tests with optional tiktoken feature
cargo test --all-features

# Build CLI binary
cargo build --bin cats

# Build and run CLI
cargo run --bin cats -- --help

# Build for release
cargo build --release

# Run specific test
cargo test <test_name>

# Check formatting
cargo fmt --check

# Run linter
cargo clippy -- -D warnings
```

## Code Style

- **Rust Edition**: 2021
- **Formatting**: Standard `rustfmt` configuration
- **Linting**: Pass all clippy warnings
- **Error Handling**: Use `anyhow::Result` for tool execution, `thiserror` for custom error types
- **Async**: Use Tokio runtime where needed
- **Documentation**: All public APIs must have rustdoc comments with examples

## Architecture

### Core Components

1. **Tool Registry** (`src/core.rs`)
   - Central registry for all tools
   - Tool validation and execution
   - Shared state management via `Arc<Mutex<ToolState>>`

2. **Tool Categories**
   - File Navigation (`src/file_navigation/mod.rs`)
   - Search (`src/search/mod.rs`)
   - Editing (`src/editing/mod.rs`)
   - Execution (`src/execution.rs`)
   - State (`src/state/mod.rs`)
   - Utils (`src/utils/mod.rs`)

3. **CLI Binary** (`src/main.rs`)
   - Uses `clap` for argument parsing
   - Dynamically generates subcommands from tool registry
   - Tools are listed and described automatically from `create_tool_registry()`

### Key Design Patterns

- **Trait-based Tools**: All tools implement the `Tool` trait
- **Windowed File Viewing**: Files are shown in configurable line windows (default 50 lines)
- **Stateful Navigation**: File positions and states are tracked across tool calls
- **Safe Command Execution**: Commands are validated and executed with timeouts
- **LLM-friendly Output**: All tool results include structured messages for language models

## Testing Instructions

- Run full test suite: `cargo test`
- Tests are organized by module (inline `mod tests`)
- Use `tempfile` crate for isolated test environments
- All tests must pass before committing
- Integration tests for tools in `src/*/tests.rs`
- CLI tests use `assert_cmd` and `predicates` crates

### Test Coverage

Ensure tests cover:
- Tool registration and execution
- Argument validation
- Error handling
- Edge cases (empty files, large files, etc.)
- State management across multiple tool calls

## Important Notes for Agents

### Tool System

- **Dynamic Tool Listing**: The CLI (`src/main.rs`) dynamically lists all tools from the registry
- **Never hardcode tool names**: Always use `registry.list_tools()` and `registry.get_tool()`
- **Tool descriptions**: Pulled directly from each tool's `description()` method

### Version Management

- **Public Crate**: CATS is published on crates.io - use semantic versioning carefully
- **Breaking Changes**: Major version bump (e.g., 0.1.x → 0.2.0)
- **New Features**: Minor version bump (e.g., 0.1.0 → 0.2.0 in 0.x versions)
- **Bug Fixes**: Patch version bump (e.g., 0.1.0 → 0.1.1)
- **Update CHANGELOG.md**: Always document changes in CHANGELOG.md

### Publishing to crates.io

```bash
# Verify package contents
cargo package --list

# Dry run (test publish without uploading)
cargo publish --dry-run

# Publish to crates.io (requires authentication)
cargo publish
```

### Adding New Tools

1. Implement the `Tool` trait
2. Add tool to appropriate module (file_navigation, editing, etc.)
3. Register in `create_tool_registry()` function in `src/lib.rs`
4. Add tests for the new tool
5. Update README.md with tool documentation
6. CLI will automatically pick up the new tool

### File Structure

```
cats/
├── Cargo.toml              # Package manifest with crates.io metadata
├── README.md               # Human-readable documentation
├── CHANGELOG.md            # Version history
├── LICENSE-MIT             # MIT license
├── LICENSE-APACHE          # Apache 2.0 license
├── src/
│   ├── lib.rs              # Library entry point, tool registry creation
│   ├── main.rs             # CLI binary
│   ├── core.rs             # Tool trait, registry, core types
│   ├── execution.rs        # Command execution tool
│   ├── file_navigation/    # File viewing and navigation tools
│   ├── search/             # File and content search tools
│   ├── editing/            # Text editing and file management tools
│   ├── state/              # State management
│   ├── linting/            # Code linting utilities
│   └── utils/              # Utility tools (tokens, filemap, etc.)
└── target/                 # Build artifacts (gitignored)
```

## Commit Guidelines

- Use conventional commits format: `type(scope): description`
- Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`
- Examples:
  - `feat(tools): add new file_diff tool`
  - `fix(cli): dynamically list tools from registry`
  - `docs(readme): update installation instructions`
  - `test(editing): add tests for delete_function tool`

## Dependencies

### Core Dependencies
- `anyhow` - Error handling
- `serde`, `serde_json` - Serialization
- `regex` - Pattern matching
- `glob`, `walkdir` - File system operations
- `tokio` - Async runtime
- `clap` - CLI argument parsing
- `thiserror` - Custom error types
- `tracing` - Logging
- `tempfile` - Temporary files for tests
- `edit-distance` - String similarity
- `toml` - Configuration parsing
- `syn` - Rust code parsing (for linting)

### Optional Dependencies
- `tiktoken-rs` - Token counting (feature-gated as "tiktoken")

## Security Considerations

- **Command Execution**: The `run_command` tool filters dangerous commands
- **File Operations**: Validate paths to prevent directory traversal
- **Timeout Protection**: All command executions have configurable timeouts
- **No Arbitrary Code Execution**: Tools are structured and validated

## Common Pitfalls

1. **Don't hardcode tool lists**: Use `registry.list_tools()` instead
2. **State lifetime**: Tools share state via `Arc<Mutex<ToolState>>`
3. **CLI lifetimes**: Clap requires `'static` strings - use `Box::leak()` for dynamic content
4. **Test isolation**: Always use `tempfile` for file system tests
5. **Error context**: Provide helpful error messages for LLMs to understand failures

## Integration with Simpaticoder

CATS was extracted from the Simpaticoder project and is now maintained as an independent crate. Changes here may affect Simpaticoder's tool system. Coordinate breaking changes carefully.
