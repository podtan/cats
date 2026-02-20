//! WebSearch tool implementation compatible with OpenCode
//!
//! Searches the web using Exa AI API.

use crate::core::{Tool, ToolArgs, ToolError, ToolResult};
use anyhow::Result;
use reqwest::blocking::Client;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const API_BASE_URL: &str = "https://mcp.exa.ai";
const DEFAULT_NUM_RESULTS: u8 = 8;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(25);

/// WebSearch tool parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WebSearchParams {
    /// Websearch query
    pub query: String,
    /// Number of search results to return (default: 8)
    pub num_results: Option<u8>,
    /// Live crawl mode - 'fallback': use live crawling as backup, 'preferred': prioritize live crawling
    pub livecrawl: Option<String>,
    /// Search type - 'auto': balanced, 'fast': quick results, 'deep': comprehensive search
    #[serde(rename = "type")]
    pub search_type: Option<String>,
    /// Maximum characters for context string optimized for LLMs
    pub context_max_characters: Option<usize>,
}

/// MCP Search Request
#[derive(Debug, Serialize)]
struct McpSearchRequest {
    jsonrpc: String,
    id: u8,
    method: String,
    params: McpSearchParams,
}

#[derive(Debug, Serialize)]
struct McpSearchParams {
    name: String,
    arguments: McpSearchArguments,
}

#[derive(Debug, Serialize)]
struct McpSearchArguments {
    query: String,
    #[serde(rename = "type")]
    search_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    numResults: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    livecrawl: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    contextMaxCharacters: Option<usize>,
}

/// MCP Search Response
#[derive(Debug, Deserialize)]
struct McpSearchResponse {
    result: McpSearchResult,
}

#[derive(Debug, Deserialize)]
struct McpSearchResult {
    content: Vec<McpContent>,
}

#[derive(Debug, Deserialize)]
struct McpContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

/// WebSearch tool for searching the web
pub struct WebSearchTool {
    name: String,
    client: Client,
}

impl WebSearchTool {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            name: "websearch".to_string(),
            client,
        }
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Searches the web using Exa AI"
    }

    fn signature(&self) -> &str {
        "websearch --query <query> [--num-results <n>] [--livecrawl <mode>] [--type <type>]"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        if args.get_named_arg("query").is_none() && args.args.is_empty() {
            return Err(ToolError::InvalidArgs {
                message: "websearch tool requires a 'query' argument".to_string(),
            });
        }
        Ok(())
    }

    fn execute(
        &mut self,
        args: &ToolArgs,
        _state: &Arc<Mutex<crate::state::ToolState>>,
    ) -> Result<ToolResult> {
        let params = parse_websearch_args(args)?;

        if params.query.is_empty() {
            return Err(ToolError::InvalidArgs {
                message: "query cannot be empty".to_string(),
            }
            .into());
        }

        let search_request = McpSearchRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "tools/call".to_string(),
            params: McpSearchParams {
                name: "web_search_exa".to_string(),
                arguments: McpSearchArguments {
                    query: params.query.clone(),
                    search_type: params.search_type.unwrap_or_else(|| "auto".to_string()),
                    numResults: params.num_results.or(Some(DEFAULT_NUM_RESULTS)),
                    livecrawl: params.livecrawl.or(Some("fallback".to_string())),
                    contextMaxCharacters: params.context_max_characters,
                },
            },
        };

        let response = self
            .client
            .post(format!("{}/mcp", API_BASE_URL))
            .header("Accept", "application/json, text/event-stream")
            .header("Content-Type", "application/json")
            .json(&search_request)
            .send()?;

        if !response.status().is_success() {
            let error_text = response.text().unwrap_or_default();
            return Err(anyhow::anyhow!("Search error: {}", error_text));
        }

        let response_text = response.text()?;

        // Parse SSE response
        for line in response_text.lines() {
            if line.starts_with("data: ") {
                let data: &str = &line[6..];
                if let Ok(search_response) = serde_json::from_str::<McpSearchResponse>(data) {
                    if let Some(content) = search_response.result.content.first() {
                        return Ok(ToolResult::success_with_data(
                            content.text.clone(),
                            serde_json::json!({
                                "query": params.query,
                                "num_results": params.num_results,
                            }),
                        ));
                    }
                }
            }
        }

        Ok(ToolResult::success_with_data(
            "No search results found. Please try a different query.".to_string(),
            serde_json::json!({
                "query": params.query,
                "results": 0,
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        let schema = schemars::schema_for!(WebSearchParams);
        serde_json::to_value(schema).unwrap_or_default()
    }
}

fn parse_websearch_args(args: &ToolArgs) -> Result<WebSearchParams> {
    let query = args
        .get_named_arg("query")
        .cloned()
        .or_else(|| args.args.first().cloned())
        .ok_or_else(|| anyhow::anyhow!("query is required"))?;

    let num_results = args
        .get_named_arg("num_results")
        .or_else(|| args.get_named_arg("numResults"))
        .and_then(|s| s.parse::<u8>().ok());

    let livecrawl = args.get_named_arg("livecrawl").cloned();

    let search_type = args.get_named_arg("type").cloned();

    let context_max_characters = args
        .get_named_arg("context_max_characters")
        .and_then(|s| s.parse::<usize>().ok());

    Ok(WebSearchParams {
        query,
        num_results,
        livecrawl,
        search_type,
        context_max_characters,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websearch_tool_creation() {
        let tool = WebSearchTool::new();
        assert_eq!(tool.name(), "websearch");
    }

    #[test]
    fn test_websearch_tool_validation() {
        let tool = WebSearchTool::new();
        let args = ToolArgs::from_args(&[]);

        let result = tool.validate_args(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_websearch_args() {
        let args = ToolArgs::with_named_args(
            vec!["rust programming".to_string()],
            vec![
                ("num_results".to_string(), "5".to_string()),
                ("type".to_string(), "fast".to_string()),
            ]
            .into_iter()
            .collect(),
        );

        let params = parse_websearch_args(&args).unwrap();
        assert_eq!(params.query, "rust programming");
        assert_eq!(params.num_results, Some(5));
        assert_eq!(params.search_type, Some("fast".to_string()));
    }
}
