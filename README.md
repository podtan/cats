# CATS - Coding Agent ToolS

[![Crates.io](https://img.shields.io/crates/v/cats.svg)](https://crates.io/crates/cats)
[![Documentation](https://docs.rs/cats/badge.svg)](https://docs.rs/cats)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/podtan/cats#license)
[![CI](https://github.com/podtan/cats/workflows/CI/badge.svg)](https://github.com/podtan/cats/actions)

A comprehensive toolkit for building AI-powered coding agents. CATS provides structured, LLM-friendly tools for file manipulation, code editing, search, and execution that work seamlessly with language models.

## Features

- **🔍 File Navigation**: Windowed file viewing with line-by-line navigation and scrolling
- **🔎 Search Tools**: Fast file discovery and content search across files and directories
- **✏️ Code Editing**: Intelligent text editing with search/replace, insert, delete operations
- **📂 File Management**: Create, move, copy, and delete files and directories
- **⚡ Command Execution**: Safe command execution with timeout and validation
- **📊 State Management**: Persistent tool state and session history
- **🗺️ Project Mapping**: Visualize project structure with intelligent elision
- **🎯 Task Classification**: Built-in task classification for agent workflows

## Quick Start

Add CATS to your `Cargo.toml`:

```toml
[dependencies]
cats = "0.1.0"
```

### Basic Usage

```rust
use cats::{create_tool_registry, ToolArgs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the tool registry
    let mut registry = create_tool_registry();
    
    // Execute a tool
    let args = ToolArgs::from_args(&["src/main.rs"]);
    let result = registry.execute_tool("open", &args)?;
    
    println!("{}", result.message);
    Ok(())
}
```

### With Custom Window Size

```rust
use cats::create_tool_registry_with_open_window_size;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create registry with custom window size for file viewing
    let mut registry = create_tool_registry_with_open_window_size(50);
    
    // Now 'open' tool will show 50 lines at a time
    Ok(())
}
```

## Available Tools

### File Navigation
- **`open`** - Opens a file and displays a window of lines
- **`goto`** - Jumps to a specific line number in the current file
- **`scroll_up`** - Scrolls the viewing window up
- **`scroll_down`** - Scrolls the viewing window down

### Search
- **`find_file`** - Search for files by name pattern
- **`search_file`** - Search for text within a specific file
- **`search_dir`** - Search for text across all files in a directory

### Editing
- **`create_file`** - Create a new file with content
- **`replace_text`** - Replace text using search/replace pattern
- **`insert_text`** - Insert text at a specific line
- **`delete_text`** - Delete a range of lines
- **`delete_line`** - Delete a specific line
- **`overwrite_file`** - Replace entire file contents
- **`delete_function`** - Delete a Rust function by name (Rust-aware)

### File Management
- **`delete_path`** - Delete a file or directory
- **`move_path`** - Move or rename a file/directory
- **`copy_path`** - Copy a file or directory
- **`create_directory`** - Create a new directory

### Execution
- **`run_command`** - Execute shell commands with timeout and validation

### Utilities
- **`_state`** - Display current tool state and context
- **`count_tokens`** - Count tokens in a file (requires `tiktoken` feature)
- **`filemap`** - Generate a project structure visualization
- **`submit`** - Mark task as complete
- **`classify_task`** - Classify task type for workflow routing

## Tool Execution Patterns

### Using Named Arguments (Recommended for LLMs)

```rust
use cats::{create_tool_registry, ToolArgs};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = create_tool_registry();
    
    // LLMs typically provide JSON with named parameters
    let mut args = HashMap::new();
    args.insert("file_path".to_string(), "src/main.rs".to_string());
    args.insert("insert_line".to_string(), "10".to_string());
    args.insert("new_str".to_string(), "// New comment".to_string());
    
    let tool_args = ToolArgs::from_named(args);
    let result = registry.execute_tool("insert_text", &tool_args)?;
    
    println!("{}", result.message);
    Ok(())
}
```

### Using Positional Arguments

```rust
use cats::{create_tool_registry, ToolArgs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = create_tool_registry();
    
    let args = ToolArgs::from_args(&["src", "TODO"]);
    let result = registry.execute_tool("search_dir", &args)?;
    
    println!("{}", result.message);
    Ok(())
}
```

## Features Flags

### `tiktoken` (Optional)

Enable token counting functionality using the `cl100k_base` tokenizer:

```toml
[dependencies]
cats = { version = "0.1.0", features = ["tiktoken"] }
```

With this feature enabled, you can use the `count_tokens` tool:

```rust
use cats::{create_tool_registry, ToolArgs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = create_tool_registry();
    let args = ToolArgs::from_args(&["src/main.rs"]);
    let result = registry.execute_tool("count_tokens", &args)?;
    println!("{}", result.message);  // "Total tokens: 1234"
    Ok(())
}
```

## Architecture

CATS is designed with LLM integration as a first-class concern:

- **Structured Outputs**: All tools return structured `ToolResult` types with clear success/error states
- **Schema Generation**: Tools provide JSON schemas for LLM function calling
- **Error Handling**: Comprehensive error messages with suggestions for resolution
- **State Tracking**: Maintains context about open files and operations
- **Token Awareness**: Optional token counting for context management

## Integration Examples

### With OpenAI Function Calling

```rust
use cats::create_tool_registry;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = create_tool_registry();
    
    // Get all tool schemas for OpenAI function calling
    let schemas = registry.get_all_schemas();
    
    // Send schemas to OpenAI API as available functions
    // ... your OpenAI integration code ...
    
    Ok(())
}
```

### As a CLI Tool

CATS includes a binary that can be used standalone:

```bash
# Install the CLI
cargo install cats

# Use tools from command line
cats open src/main.rs
cats search_dir . "TODO"
cats filemap src/
```

## Platform Support

- **Linux** (x86_64, aarch64) - Tier 1 (fully supported and tested)
- **macOS** (x86_64, aarch64) - Tier 2 (builds and tests pass)
- **Windows** (x86_64) - Tier 3 (best-effort support)

## Migration from `simpaticoder-tools`

CATS is the successor to `simpaticoder-tools`. The crate has been renamed and extracted for independent use:

```toml
# Old (deprecated)
[dependencies]
simpaticoder-tools = "0.1.0"

# New
[dependencies]
cats = "0.1.0"
```

Update imports:

```rust
// Old
use simpaticoder_tools::{create_tool_registry, ToolArgs};

// New  
use cats::{create_tool_registry, ToolArgs};
```

The API surface remains identical - only the crate name has changed.

## Documentation

- [API Documentation](https://docs.rs/cats)
- [Examples](https://github.com/podtan/cats/tree/main/examples)
- [Homepage](https://cats.podtan.com)

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

CATS is inspired by and builds upon concepts from:
- [SWE-agent](https://github.com/princeton-nlp/SWE-agent) - Agent-Computer Interface design
- The broader AI coding agent community

---

**Built for the future of AI-assisted software development.** 🚀
