//! Large tool result handling for LLM context limits
//!
//! Provides utilities to handle tool results that exceed size limits,
//! either through truncation or replacement with warnings.

/// Configuration for result handling
#[derive(Debug, Clone)]
pub struct ResultHandlerConfig {
    /// Maximum size in bytes for tool results
    pub max_size_bytes: usize,
    /// Whether to truncate large results (true) or replace with warning (false)
    pub truncate_enabled: bool,
}

impl Default for ResultHandlerConfig {
    fn default() -> Self {
        Self {
            max_size_bytes: 256000,
            truncate_enabled: true,
        }
    }
}

/// Handle large tool results by truncating or warning
///
/// # Arguments
/// * `tool_name` - Name of the tool that generated the result
/// * `result_message` - The result message from the tool
/// * `config` - Configuration for result handling
///
/// # Returns
/// * `String` - Processed result (original, truncated, or warning message)
pub fn handle_large_result(
    tool_name: &str,
    result_message: &str,
    config: &ResultHandlerConfig,
) -> String {
    let result_size = result_message.len();

    if result_size > config.max_size_bytes {
        if config.truncate_enabled {
            // Truncate and add warning message for LLM
            let truncated = if result_message.len() > config.max_size_bytes {
                &result_message[..config.max_size_bytes]
            } else {
                result_message
            };

            format!(
                "{}\n\n⚠️ WARNING: Tool result was truncated due to size limit ({} bytes > {} bytes). \
                The result may be incomplete. Please refine your query to get more specific information \
                or use more targeted search parameters to reduce result size.",
                truncated, result_size, config.max_size_bytes
            )
        } else {
            // Don't include the actual result, just warn the LLM
            format!(
                "⚠️ RESULT TOO LARGE: Tool '{}' returned {} bytes (limit: {} bytes). \
                The result was not included to prevent API payload errors. \
                Please refine your query to be more specific and reduce the result size. \
                Consider using:\n\
                - More specific search terms\n\
                - Smaller file ranges\n\
                - Targeted grep patterns\n\
                - Pagination or limited result counts",
                tool_name, result_size, config.max_size_bytes
            )
        }
    } else {
        result_message.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_large_result_small() {
        let config = ResultHandlerConfig::default();
        let small_result = "This is a small result";
        let processed = handle_large_result("test_tool", small_result, &config);

        assert_eq!(processed, small_result);
    }

    #[test]
    fn test_handle_large_result_large_with_truncation() {
        let config = ResultHandlerConfig {
            max_size_bytes: 1000,
            truncate_enabled: true,
        };

        // Create a result that exceeds the size limit
        let large_result = "x".repeat(1500);
        let processed = handle_large_result("test_tool", &large_result, &config);

        // Should be truncated and contain warning
        assert!(processed.contains("⚠️ WARNING: Tool result was truncated"));
        assert!(processed.len() < large_result.len() + 500); // Accounting for warning message
    }

    #[test]
    fn test_handle_large_result_large_without_truncation() {
        let config = ResultHandlerConfig {
            max_size_bytes: 1000,
            truncate_enabled: false,
        };

        // Create a result that exceeds the size limit
        let large_result = "x".repeat(1500);
        let processed = handle_large_result("test_tool", &large_result, &config);

        // Should not include the actual result, just warning
        assert!(processed.contains("⚠️ RESULT TOO LARGE"));
        assert!(!processed.contains("xxx")); // Original content not included
    }
}
