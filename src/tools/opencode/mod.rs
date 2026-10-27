//! OpenCode-compatible tool set
//!
//! This module provides tools compatible with OpenCode's tool interface.
//! These tools follow the same patterns and behaviors as the original OpenCode implementation.

mod bash;
mod edit;
mod glob;
mod grep;
mod list;
mod multiedit;
mod read;
mod todo;
mod webfetch;
mod websearch;
mod write;

pub use bash::BashTool;
pub use edit::EditTool;
pub use glob::GlobTool;
pub use grep::GrepTool;
pub use list::ListTool;
pub use multiedit::MultiEditTool;
pub use read::ReadTool;
pub use todo::{TodoReadTool, TodoWriteTool};
pub use webfetch::WebFetchTool;
pub use websearch::WebSearchTool;
pub use write::WriteTool;

use crate::core::ToolRegistry;

/// Build a `reqwest::blocking::Client` OUTSIDE any async context.
///
/// reqwest's blocking client creates its own tokio runtime at build time —
/// building it inside an async worker PANICS ("Cannot drop a runtime in a
/// context where blocking is not allowed"; observed with reqwest 0.12.28 +
/// tokio 1.54.1 when the tool registry is created inside the workflow's
/// async context — Ravand 2026-10-27). A plain thread has no async
/// context: the runtime is created and kept there, and only the client
/// handle crosses back.
pub(crate) fn build_blocking_client(
    builder: reqwest::blocking::ClientBuilder,
) -> reqwest::blocking::Client {
    std::thread::scope(|s| {
        s.spawn(move || builder.build().unwrap_or_else(|_| reqwest::blocking::Client::new()))
            .join()
            .unwrap_or_else(|_| reqwest::blocking::Client::new())
    })
}

/// Initialize the tool registry with OpenCode-compatible tools
pub fn create_tool_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    // Core tools
    registry.register(Box::new(BashTool::new()));
    registry.register(Box::new(ReadTool::new()));
    registry.register(Box::new(WriteTool::new()));
    registry.register(Box::new(EditTool::new()));
    registry.register(Box::new(GlobTool::new()));
    registry.register(Box::new(GrepTool::new()));
    registry.register(Box::new(ListTool::new()));

    // Extended tools
    registry.register(Box::new(MultiEditTool::new()));
    registry.register(Box::new(WebFetchTool::new()));
    registry.register(Box::new(WebSearchTool::new()));
    registry.register(Box::new(TodoWriteTool::new()));
    registry.register(Box::new(TodoReadTool::new()));

    registry
}

/// Initialize the tool registry with OpenCode-compatible tools and custom window size
pub fn create_tool_registry_with_open_window_size(
    _open_window_size: Option<usize>,
) -> ToolRegistry {
    create_tool_registry()
}
