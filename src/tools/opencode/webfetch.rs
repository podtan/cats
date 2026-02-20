//! WebFetch tool implementation compatible with OpenCode
//!
//! Fetches content from a specified URL and converts to the requested format.

use crate::core::{Tool, ToolArgs, ToolError, ToolResult};
use anyhow::Result;
use reqwest::blocking::Client;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const MAX_RESPONSE_SIZE: usize = 5 * 1024 * 1024; // 5MB
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_TIMEOUT: Duration = Duration::from_secs(120);

/// WebFetch tool parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WebFetchParams {
    /// The URL to fetch content from
    pub url: String,
    /// The format to return the content in (text, markdown, or html). Defaults to markdown.
    #[serde(default = "default_format")]
    pub format: String,
    /// Optional timeout in seconds (max 120)
    pub timeout: Option<u64>,
}

fn default_format() -> String {
    "markdown".to_string()
}

/// WebFetch tool for fetching web content
pub struct WebFetchTool {
    name: String,
    client: Client,
}

impl WebFetchTool {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            name: "webfetch".to_string(),
            client,
        }
    }
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Fetches content from a specified URL"
    }

    fn signature(&self) -> &str {
        "webfetch --url <url> [--format <format>] [--timeout <seconds>]"
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<(), ToolError> {
        if args.get_named_arg("url").is_none() && args.args.is_empty() {
            return Err(ToolError::InvalidArgs {
                message: "webfetch tool requires a 'url' argument".to_string(),
            });
        }
        Ok(())
    }

    fn execute(
        &mut self,
        args: &ToolArgs,
        _state: &Arc<Mutex<crate::state::ToolState>>,
    ) -> Result<ToolResult> {
        let params = parse_webfetch_args(args)?;

        // Validate URL
        let url = if params.url.starts_with("http://") || params.url.starts_with("https://") {
            params.url.clone()
        } else if params.url.starts_with("//") {
            format!("https:{}", params.url)
        } else {
            format!("https://{}", params.url)
        };

        let timeout = params
            .timeout
            .map(|t| Duration::from_secs(t.min(120)))
            .unwrap_or(DEFAULT_TIMEOUT);

        // Build request with appropriate Accept header
        let accept_header = match params.format.as_str() {
            "markdown" => "text/markdown;q=1.0, text/x-markdown;q=0.9, text/plain;q=0.8, text/html;q=0.7, */*;q=0.1",
            "text" => "text/plain;q=1.0, text/markdown;q=0.9, text/html;q=0.8, */*;q=0.1",
            "html" => "text/html;q=1.0, application/xhtml+xml;q=0.9, text/plain;q=0.8, text/markdown;q=0.7, */*;q=0.1",
            _ => "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        };

        let response = self
            .client
            .get(&url)
            .header("Accept", accept_header)
            .header("Accept-Language", "en-US,en;q=0.9")
            .timeout(timeout)
            .send()?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Request failed with status code: {}",
                response.status()
            ));
        }

        // Check content length
        if let Some(content_length) = response.content_length() {
            if content_length as usize > MAX_RESPONSE_SIZE {
                return Err(anyhow::anyhow!("Response too large (exceeds 5MB limit)"));
            }
        }

        // Get content type before consuming response
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("")
            .to_string();

        let bytes = response.bytes()?;
        if bytes.len() > MAX_RESPONSE_SIZE {
            return Err(anyhow::anyhow!("Response too large (exceeds 5MB limit)"));
        }

        let content = String::from_utf8_lossy(&bytes).to_string();

        // Convert content based on format
        let output = match params.format.as_str() {
            "markdown" => {
                if content_type.contains("text/html") {
                    html_to_markdown(&content)
                } else {
                    content
                }
            }
            "text" => {
                if content_type.contains("text/html") {
                    html_to_text(&content)
                } else {
                    content
                }
            }
            "html" | _ => content,
        };

        Ok(ToolResult::success_with_data(
            output,
            serde_json::json!({
                "url": params.url,
                "format": params.format,
                "content_type": content_type,
            }),
        ))
    }

    fn get_parameters_schema(&self) -> serde_json::Value {
        let schema = schemars::schema_for!(WebFetchParams);
        serde_json::to_value(schema).unwrap_or_default()
    }
}

fn parse_webfetch_args(args: &ToolArgs) -> Result<WebFetchParams> {
    let url = args
        .get_named_arg("url")
        .cloned()
        .or_else(|| args.args.first().cloned())
        .ok_or_else(|| anyhow::anyhow!("url is required"))?;

    let format = args
        .get_named_arg("format")
        .cloned()
        .unwrap_or_else(default_format);

    let timeout = args
        .get_named_arg("timeout")
        .and_then(|s| s.parse::<u64>().ok());

    Ok(WebFetchParams {
        url,
        format,
        timeout,
    })
}

/// Simple HTML to Markdown converter
fn html_to_markdown(html: &str) -> String {
    let mut result = html.to_string();

    // Remove script and style tags with content
    let script_pattern = regex::Regex::new(r"<script[^>]*>.*?</script>").unwrap();
    let style_pattern = regex::Regex::new(r"<style[^>]*>.*?</style>").unwrap();
    result = script_pattern.replace_all(&result, "").to_string();
    result = style_pattern.replace_all(&result, "").to_string();

    // Convert headings (h1-h6) - match each level separately to avoid backreferences
    for level in 1..=6 {
        let pattern = regex::Regex::new(&format!(r"<h{}[^>]*>(.*?)</h{}>", level, level)).unwrap();
        let prefix = "#".repeat(level);
        result = pattern
            .replace_all(&result, |caps: &regex::Captures| {
                let text = strip_html_tags(&caps[1]);
                format!("{} {}", prefix, text)
            })
            .to_string();
    }

    // Bold - match strong and b separately
    let strong_pattern = regex::Regex::new(r"<strong[^>]*>(.*?)</strong>").unwrap();
    result = strong_pattern
        .replace_all(&result, |caps: &regex::Captures| {
            format!("**{}**", strip_html_tags(&caps[1]))
        })
        .to_string();

    let b_pattern = regex::Regex::new(r"<b[^>]*>(.*?)</b>").unwrap();
    result = b_pattern
        .replace_all(&result, |caps: &regex::Captures| {
            format!("**{}**", strip_html_tags(&caps[1]))
        })
        .to_string();

    // Italic - match em and i separately
    let em_pattern = regex::Regex::new(r"<em[^>]*>(.*?)</em>").unwrap();
    result = em_pattern
        .replace_all(&result, |caps: &regex::Captures| {
            format!("*{}*", strip_html_tags(&caps[1]))
        })
        .to_string();

    let i_pattern = regex::Regex::new(r"<i[^>]*>(.*?)</i>").unwrap();
    result = i_pattern
        .replace_all(&result, |caps: &regex::Captures| {
            format!("*{}*", strip_html_tags(&caps[1]))
        })
        .to_string();

    // Links
    let link_pattern = regex::Regex::new(r#"<a[^>]*href="([^"]*)"[^>]*>(.*?)</a>"#).unwrap();
    result = link_pattern
        .replace_all(&result, |caps: &regex::Captures| {
            format!("[{}]({})", strip_html_tags(&caps[2]), &caps[1])
        })
        .to_string();

    // Line breaks
    result = result.replace("<br>", "\n");
    result = result.replace("<br/>", "\n");
    result = result.replace("<br />", "\n");

    // Paragraphs
    let p_pattern = regex::Regex::new(r"<p[^>]*>(.*?)</p>").unwrap();
    result = p_pattern
        .replace_all(&result, |caps: &regex::Captures| {
            format!("\n\n{}\n\n", strip_html_tags(&caps[1]))
        })
        .to_string();

    // Remove remaining HTML tags
    result = strip_html_tags(&result);

    // Clean up whitespace
    let whitespace_pattern = regex::Regex::new(r"\n{3,}").unwrap();
    result = whitespace_pattern.replace_all(&result, "\n\n").to_string();

    result.trim().to_string()
}

/// Simple HTML to plain text converter
fn html_to_text(html: &str) -> String {
    let mut result = html.to_string();

    // Remove script and style tags with content
    let script_pattern = regex::Regex::new(r"<script[^>]*>.*?</script>").unwrap();
    let style_pattern = regex::Regex::new(r"<style[^>]*>.*?</style>").unwrap();
    result = script_pattern.replace_all(&result, "").to_string();
    result = style_pattern.replace_all(&result, "").to_string();

    // Line breaks
    result = result.replace("<br>", "\n");
    result = result.replace("<br/>", "\n");
    result = result.replace("<br />", "\n");

    // Paragraphs
    let p_pattern = regex::Regex::new(r"<p[^>]*>(.*?)</p>").unwrap();
    result = p_pattern
        .replace_all(&result, |caps: &regex::Captures| {
            format!("\n\n{}\n\n", strip_html_tags(&caps[1]))
        })
        .to_string();

    // Remove all remaining HTML tags
    result = strip_html_tags(&result);

    // Decode HTML entities
    result = result.replace("&nbsp;", " ");
    result = result.replace("&amp;", "&");
    result = result.replace("&lt;", "<");
    result = result.replace("&gt;", ">");
    result = result.replace("&quot;", "\"");

    // Clean up whitespace
    let whitespace_pattern = regex::Regex::new(r"\n{3,}").unwrap();
    result = whitespace_pattern.replace_all(&result, "\n\n").to_string();

    result.trim().to_string()
}

fn strip_html_tags(html: &str) -> String {
    let tag_pattern = regex::Regex::new(r"<[^>]+>").unwrap();
    tag_pattern.replace_all(html, "").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webfetch_tool_creation() {
        let tool = WebFetchTool::new();
        assert_eq!(tool.name(), "webfetch");
    }

    #[test]
    fn test_webfetch_tool_validation() {
        let tool = WebFetchTool::new();
        let args = ToolArgs::from_args(&[]);

        let result = tool.validate_args(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_html_to_markdown() {
        let html = "<h1>Title</h1><p>This is <strong>bold</strong> text.</p>";
        let markdown = html_to_markdown(html);
        assert!(markdown.contains("# Title"));
        assert!(markdown.contains("**bold**"));
    }

    #[test]
    fn test_html_to_text() {
        let html = "<p>Hello</p><p>World</p>";
        let text = html_to_text(html);
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
    }

    #[test]
    fn test_parse_webfetch_args() {
        let args = ToolArgs::with_named_args(
            vec!["https://example.com".to_string()],
            vec![("format".to_string(), "text".to_string())]
                .into_iter()
                .collect(),
        );

        let params = parse_webfetch_args(&args).unwrap();
        assert_eq!(params.url, "https://example.com");
        assert_eq!(params.format, "text");
    }
}
