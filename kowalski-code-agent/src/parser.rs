use crate::config::CodeAgentConfig;
use crate::error::CodeAgentError;
use std::path::Path;
use tree_sitter::{Language, Parser};

/// A code parser that uses tree-sitter for parsing various programming languages
pub struct CodeParser {
    parser: Parser,
    config: CodeAgentConfig,
}

impl CodeParser {
    /// Creates a new code parser with the specified configuration
    pub fn new(config: CodeAgentConfig) -> Result<Self, CodeAgentError> {
        let mut parser = Parser::new();
        // Initialize with Rust language by default - BY HAND!!!
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .expect("Error loading Rust grammar");

        Ok(Self { parser, config })
    }

    /// Parses a file at the given path
    pub fn parse_file(&mut self, path: &Path) -> Result<(), CodeAgentError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| CodeAgentError::Parser(format!("Failed to read file: {}", e)))?;

        if content.len() > self.config.max_file_size {
            return Err(CodeAgentError::Parser("File too large".to_string()));
        }

        self.parse_content(&content)
    }

    /// Parses a string of code
    pub fn parse_content(&mut self, content: &str) -> Result<(), CodeAgentError> {
        self.parser
            .parse(content, None)
            .ok_or_else(|| CodeAgentError::Parser("Failed to parse content".to_string()))?;

        Ok(())
    }

    /// Sets the language for parsing
    pub fn set_language(&mut self, language: &Language) -> Result<(), CodeAgentError> {
        self.parser
            .set_language(language)
            .map_err(|e| CodeAgentError::Parser(format!("Failed to set language: {}", e)))
    }

    /// Gets the current parser
    pub fn parser(&self) -> &Parser {
        &self.parser
    }

    /// Gets a mutable reference to the current parser
    pub fn parser_mut(&mut self) -> &mut Parser {
        &mut self.parser
    }

    /// Gets the configuration
    pub fn config(&self) -> &CodeAgentConfig {
        &self.config
    }

    /// Gets a mutable reference to the configuration
    pub fn config_mut(&mut self) -> &mut CodeAgentConfig {
        &mut self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_creation() {
        let config = CodeAgentConfig::default();
        let parser = CodeParser::new(config);
        assert!(parser.is_ok());
    }

    #[test]
    fn test_parse_content() {
        let config = CodeAgentConfig::default();
        let mut parser = CodeParser::new(config).unwrap();
        let result = parser.parse_content("fn main() { println!(\"Hello, world!\"); }");
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_language() {
        let config = CodeAgentConfig::default();
        let mut parser = CodeParser::new(config).unwrap();
        let language = unsafe { tree_sitter_rust::language() };
        let result = parser.set_language(&language);
        assert!(result.is_ok());
    }
}
