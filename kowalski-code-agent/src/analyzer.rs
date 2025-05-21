use crate::error::CodeAgentError;
use crate::config::CodeAgentConfig;
use crate::parser::CodeParser;
use tree_sitter::{Node, Tree};
use std::collections::HashMap;

/// A code analyzer that performs various types of analysis on code
pub struct CodeAnalyzer {
    parser: CodeParser,
    config: CodeAgentConfig,
    metrics: HashMap<String, f64>,
}

impl CodeAnalyzer {
    /// Creates a new code analyzer with the specified configuration
    pub fn new(config: CodeAgentConfig) -> Result<Self, CodeAgentError> {
        let parser = CodeParser::new(config.clone())?;

        Ok(Self {
            parser,
            config,
            metrics: HashMap::new(),
        })
    }

    /// Analyzes a file at the given path
    pub fn analyze_file(&mut self, path: &str) -> Result<(), CodeAgentError> {
        self.parser.parse_file(path.as_ref())?;
        let tree = self.parser.parser_mut().parse(path, None)
            .ok_or_else(|| CodeAgentError::Parser("Failed to parse file".to_string()))?;
        self.analyze_tree(&tree)
    }

    /// Analyzes a string of code
    pub fn analyze_content(&mut self, content: &str) -> Result<(), CodeAgentError> {
        self.parser.parse_content(content)?;
        let tree = self.parser.parser_mut().parse(content.as_bytes(), None)
            .ok_or_else(|| CodeAgentError::Parser("Failed to parse content".to_string()))?;
        self.analyze_tree(&tree)
    }

    /// Analyzes a syntax tree
    fn analyze_tree(&mut self, tree: &Tree) -> Result<(), CodeAgentError> {
        if self.config.enable_complexity_analysis {
            self.analyze_complexity(tree)?;
        }

        if self.config.enable_duplication_detection {
            self.detect_duplication(tree)?;
        }

        if self.config.enable_code_metrics {
            self.calculate_metrics(tree)?;
        }

        Ok(())
    }

    /// Analyzes code complexity
    fn analyze_complexity(&mut self, tree: &Tree) -> Result<(), CodeAgentError> {
        let root_node = tree.root_node();
        let complexity = self.calculate_complexity(&root_node);
        self.metrics.insert("complexity".to_string(), complexity);
        Ok(())
    }

    /// Calculates code complexity for a node
    fn calculate_complexity(&self, node: &Node) -> f64 {
        let mut complexity = 1.0;

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                complexity += self.calculate_complexity(&child);
            }
        }

        complexity
    }

    /// Detects code duplication
    fn detect_duplication(&mut self, tree: &Tree) -> Result<(), CodeAgentError> {
        let root_node = tree.root_node();
        let duplication = self.find_duplicates(&root_node);
        self.metrics.insert("duplication".to_string(), duplication);
        Ok(())
    }

    /// Finds duplicate code patterns
    fn find_duplicates(&self, node: &Node) -> f64 {
        let mut duplicates = 0.0;
        let mut patterns = HashMap::new();

        self.collect_patterns(node, &mut patterns);

        for (_, count) in patterns {
            if count > 1 {
                duplicates += count as f64;
            }
        }

        duplicates
    }

    /// Collects code patterns from a node
    fn collect_patterns(&self, node: &Node, patterns: &mut HashMap<String, usize>) {
        let kind = node.kind();
        let pattern = format!("{}:{}", kind, node.start_position().row);

        *patterns.entry(pattern).or_insert(0) += 1;

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.collect_patterns(&child, patterns);
            }
        }
    }

    /// Calculates various code metrics
    fn calculate_metrics(&mut self, tree: &Tree) -> Result<(), CodeAgentError> {
        let root_node = tree.root_node();
        
        // Calculate lines of code
        let loc = self.calculate_loc(&root_node);
        self.metrics.insert("loc".to_string(), loc);

        // Calculate number of functions
        let functions = self.count_functions(&root_node);
        self.metrics.insert("functions".to_string(), functions);

        // Calculate number of classes
        let classes = self.count_classes(&root_node);
        self.metrics.insert("classes".to_string(), classes);

        Ok(())
    }

    /// Calculates lines of code
    fn calculate_loc(&self, node: &Node) -> f64 {
        let mut loc = 0.0;

        if node.child_count() == 0 {
            loc = 1.0;
        }

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                loc += self.calculate_loc(&child);
            }
        }

        loc
    }

    /// Counts number of functions
    fn count_functions(&self, node: &Node) -> f64 {
        let mut count = 0.0;

        if node.kind() == "function_item" {
            count = 1.0;
        }

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                count += self.count_functions(&child);
            }
        }

        count
    }

    /// Counts number of classes
    fn count_classes(&self, node: &Node) -> f64 {
        let mut count = 0.0;

        if node.kind() == "struct_item" || node.kind() == "enum_item" {
            count = 1.0;
        }

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                count += self.count_classes(&child);
            }
        }

        count
    }

    /// Gets the calculated metrics
    pub fn metrics(&self) -> &HashMap<String, f64> {
        &self.metrics
    }

    /// Gets a mutable reference to the calculated metrics
    pub fn metrics_mut(&mut self) -> &mut HashMap<String, f64> {
        &mut self.metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_creation() {
        let config = CodeAgentConfig::default();
        let analyzer = CodeAnalyzer::new(config);
        assert!(analyzer.is_ok());
    }

    #[test]
    fn test_analyze_content() {
        let config = CodeAgentConfig::default();
        let mut analyzer = CodeAnalyzer::new(config).unwrap();
        let result = analyzer.analyze_content("fn main() { println!(\"Hello, world!\"); }");
        assert!(result.is_ok());
    }
} 