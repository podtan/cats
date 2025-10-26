//! Linting integration for various languages

use anyhow::Result;

pub trait Linter {
    fn name(&self) -> &str;
    fn lint(&self, content: &str, file_type: &str) -> Result<Vec<String>>;
    fn supports_file_type(&self, file_type: &str) -> bool;
}

// Placeholder for linting implementation
