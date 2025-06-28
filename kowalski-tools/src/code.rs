use async_trait::async_trait;
use kowalski_core::error::KowalskiError;
use kowalski_core::tools::{Tool, ToolInput, ToolOutput, ToolParameter};
use serde_json::json;
use std::collections::HashMap;

/// A tool for analyzing Java code
pub struct JavaAnalysisTool;

impl Default for JavaAnalysisTool {
    fn default() -> Self {
        Self::new()
    }
}

impl JavaAnalysisTool {
    pub fn new() -> Self {
        Self
    }

    fn analyze_java(&self, code: &str) -> Result<serde_json::Value, KowalskiError> {
        let mut analysis = HashMap::new();

        // Basic metrics
        let lines = code.lines().count();
        let characters = code.chars().count();
        let words = code.split_whitespace().count();

        // Java-specific analysis
        let classes = code.matches("class ").count();
        let methods = code.matches("public ").count()
            + code.matches("private ").count()
            + code.matches("protected ").count();
        let imports = code.matches("import ").count();
        let comments = code.matches("//").count() + code.matches("/*").count();

        // Complexity analysis
        let complexity = self.calculate_complexity(code);

        analysis.insert(
            "metrics".to_string(),
            json!({
                "lines": lines,
                "characters": characters,
                "words": words,
                "classes": classes,
                "methods": methods,
                "imports": imports,
                "comments": comments,
                "complexity": complexity
            }),
        );

        // Code quality suggestions
        let suggestions = self.generate_suggestions(code);
        analysis.insert("suggestions".to_string(), json!(suggestions));

        // Syntax check
        let syntax_errors = self.check_syntax(code);
        analysis.insert("syntax_errors".to_string(), json!(syntax_errors));

        Ok(json!(analysis))
    }

    fn calculate_complexity(&self, code: &str) -> serde_json::Value {
        let mut complexity = HashMap::new();

        // Count control structures
        let if_statements = code.matches("if ").count();
        let for_loops = code.matches("for ").count();
        let while_loops = code.matches("while ").count();
        let switch_statements = code.matches("switch ").count();

        let cyclomatic_complexity = 1 + if_statements + for_loops + while_loops + switch_statements;

        complexity.insert(
            "cyclomatic_complexity".to_string(),
            json!(cyclomatic_complexity),
        );
        complexity.insert("if_statements".to_string(), json!(if_statements));
        complexity.insert("for_loops".to_string(), json!(for_loops));
        complexity.insert("while_loops".to_string(), json!(while_loops));
        complexity.insert("switch_statements".to_string(), json!(switch_statements));

        // Complexity level
        let level = if cyclomatic_complexity <= 5 {
            "Low"
        } else if cyclomatic_complexity <= 10 {
            "Medium"
        } else {
            "High"
        };
        complexity.insert("level".to_string(), json!(level));

        json!(complexity)
    }

    fn generate_suggestions(&self, code: &str) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Check for common Java issues
        if code.matches("System.out.println").count() > 0 {
            suggestions.push(
                "Consider using a proper logging framework instead of System.out.println"
                    .to_string(),
            );
        }

        if code.matches("public static void main").count() > 0 {
            suggestions.push("Main method found - ensure proper exception handling".to_string());
        }

        if code.matches("new ArrayList()").count() > 0 {
            suggestions.push(
                "Consider specifying initial capacity for ArrayList for better performance"
                    .to_string(),
            );
        }

        if code.matches("catch (Exception e)").count() > 0 {
            suggestions.push(
                "Consider catching specific exceptions instead of generic Exception".to_string(),
            );
        }

        suggestions
    }

    fn check_syntax(&self, code: &str) -> Vec<String> {
        let mut errors = Vec::new();

        // Basic syntax checks
        let open_braces = code.matches('{').count();
        let close_braces = code.matches('}').count();
        if open_braces != close_braces {
            errors.push("Mismatched braces detected".to_string());
        }

        let open_parens = code.matches('(').count();
        let close_parens = code.matches(')').count();
        if open_parens != close_parens {
            errors.push("Mismatched parentheses detected".to_string());
        }

        // Check for missing semicolons (basic check)
        let lines = code.lines().collect::<Vec<_>>();
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if !trimmed.is_empty()
                && !trimmed.ends_with(';')
                && !trimmed.ends_with('{')
                && !trimmed.ends_with('}')
                && !trimmed.starts_with("//")
                && !trimmed.starts_with("/*")
                && !trimmed.starts_with("import ")
                && !trimmed.starts_with("package ")
                && !trimmed.starts_with("public ")
                && !trimmed.starts_with("private ")
                && !trimmed.starts_with("protected ")
                && !trimmed.starts_with("class ")
                && !trimmed.starts_with("interface ")
                && !trimmed.starts_with("enum ")
            {
                errors.push(format!("Line {}: Possible missing semicolon", i + 1));
            }
        }

        errors
    }
}

#[async_trait]
impl Tool for JavaAnalysisTool {
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        match input.task_type.as_str() {
            "analyze_java" => {
                let result = self.analyze_java(&input.content)?;
                Ok(ToolOutput::new(
                    result,
                    Some(json!({
                        "tool": "java_analysis",
                        "language": "java"
                    })),
                ))
            }
            _ => Err(KowalskiError::ToolExecution(format!(
                "Unsupported task type: {}",
                input.task_type
            ))),
        }
    }

    fn name(&self) -> &str {
        "java_analysis"
    }

    fn description(&self) -> &str {
        "A tool for analyzing Java code, providing metrics, complexity analysis, and code quality suggestions"
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![ToolParameter {
            name: "content".to_string(),
            description: "Java code to analyze".to_string(),
            required: true,
            default_value: None,
            parameter_type: kowalski_core::tools::ParameterType::String,
        }]
    }
}

/// A tool for analyzing Python code
pub struct PythonAnalysisTool;

impl Default for PythonAnalysisTool {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonAnalysisTool {
    pub fn new() -> Self {
        Self
    }

    fn analyze_python(&self, code: &str) -> Result<serde_json::Value, KowalskiError> {
        let mut analysis = HashMap::new();

        // Basic metrics
        let lines = code.lines().count();
        let characters = code.chars().count();
        let words = code.split_whitespace().count();

        // Python-specific analysis
        let functions = code.matches("def ").count();
        let classes = code.matches("class ").count();
        let imports = code.matches("import ").count() + code.matches("from ").count();
        let comments = code.matches("#").count();

        // Complexity analysis
        let complexity = self.calculate_complexity(code);

        analysis.insert(
            "metrics".to_string(),
            json!({
                "lines": lines,
                "characters": characters,
                "words": words,
                "functions": functions,
                "classes": classes,
                "imports": imports,
                "comments": comments,
                "complexity": complexity
            }),
        );

        // Code quality suggestions
        let suggestions = self.generate_suggestions(code);
        analysis.insert("suggestions".to_string(), json!(suggestions));

        // PEP 8 compliance check
        let pep8_issues = self.check_pep8(code);
        analysis.insert("pep8_issues".to_string(), json!(pep8_issues));

        Ok(json!(analysis))
    }

    fn calculate_complexity(&self, code: &str) -> serde_json::Value {
        let mut complexity = HashMap::new();

        // Count control structures
        let if_statements = code.matches("if ").count();
        let for_loops = code.matches("for ").count();
        let while_loops = code.matches("while ").count();
        let try_blocks = code.matches("try:").count();

        let cyclomatic_complexity = 1 + if_statements + for_loops + while_loops + try_blocks;

        complexity.insert(
            "cyclomatic_complexity".to_string(),
            json!(cyclomatic_complexity),
        );
        complexity.insert("if_statements".to_string(), json!(if_statements));
        complexity.insert("for_loops".to_string(), json!(for_loops));
        complexity.insert("while_loops".to_string(), json!(while_loops));
        complexity.insert("try_blocks".to_string(), json!(try_blocks));

        // Complexity level
        let level = if cyclomatic_complexity <= 5 {
            "Low"
        } else if cyclomatic_complexity <= 10 {
            "Medium"
        } else {
            "High"
        };
        complexity.insert("level".to_string(), json!(level));

        json!(complexity)
    }

    fn generate_suggestions(&self, code: &str) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Check for common Python issues
        if code.matches("print(").count() > 0 {
            suggestions.push("Consider using logging instead of print statements".to_string());
        }

        if code.matches("except:").count() > 0 {
            suggestions.push("Avoid bare except clauses - specify exception types".to_string());
        }

        if code.matches("import *").count() > 0 {
            suggestions.push("Avoid wildcard imports - import specific modules".to_string());
        }

        if code.matches("global ").count() > 0 {
            suggestions.push(
                "Consider avoiding global variables - use function parameters or class attributes"
                    .to_string(),
            );
        }

        suggestions
    }

    fn check_pep8(&self, code: &str) -> Vec<String> {
        let mut issues = Vec::new();

        // Check line length
        for (i, line) in code.lines().enumerate() {
            if line.len() > 79 {
                issues.push(format!(
                    "Line {}: Line too long ({} characters)",
                    i + 1,
                    line.len()
                ));
            }
        }

        // Check indentation
        for (i, line) in code.lines().enumerate() {
            if !line.is_empty() && !line.starts_with('#') {
                let indent = line.chars().take_while(|&c| c == ' ').count();
                if indent % 4 != 0 {
                    issues.push(format!(
                        "Line {}: Indentation should be multiple of 4 spaces",
                        i + 1
                    ));
                }
            }
        }

        // Check for trailing whitespace
        for (i, line) in code.lines().enumerate() {
            if line.ends_with(' ') {
                issues.push(format!("Line {}: Trailing whitespace", i + 1));
            }
        }

        issues
    }
}

#[async_trait]
impl Tool for PythonAnalysisTool {
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        match input.task_type.as_str() {
            "analyze_python" => {
                let result = self.analyze_python(&input.content)?;
                Ok(ToolOutput::new(
                    result,
                    Some(json!({
                        "tool": "python_analysis",
                        "language": "python"
                    })),
                ))
            }
            _ => Err(KowalskiError::ToolExecution(format!(
                "Unsupported task type: {}",
                input.task_type
            ))),
        }
    }

    fn name(&self) -> &str {
        "python_analysis"
    }

    fn description(&self) -> &str {
        "A tool for analyzing Python code, providing metrics, complexity analysis, PEP 8 compliance, and code quality suggestions"
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![ToolParameter {
            name: "content".to_string(),
            description: "Python code to analyze".to_string(),
            required: true,
            default_value: None,
            parameter_type: kowalski_core::tools::ParameterType::String,
        }]
    }
}

/// A tool for analyzing Rust code
pub struct RustAnalysisTool;

impl Default for RustAnalysisTool {
    fn default() -> Self {
        Self::new()
    }
}

impl RustAnalysisTool {
    pub fn new() -> Self {
        Self
    }

    fn analyze_rust(&self, code: &str) -> Result<serde_json::Value, KowalskiError> {
        let mut analysis = HashMap::new();

        // Basic metrics
        let lines = code.lines().count();
        let characters = code.chars().count();
        let words = code.split_whitespace().count();

        // Rust-specific analysis
        let functions = code.matches("fn ").count();
        let structs = code.matches("struct ").count();
        let enums = code.matches("enum ").count();
        let traits = code.matches("trait ").count();
        let modules = code.matches("mod ").count();
        let comments = code.matches("//").count() + code.matches("/*").count();

        // Complexity analysis
        let complexity = self.calculate_complexity(code);

        analysis.insert(
            "metrics".to_string(),
            json!({
                "lines": lines,
                "characters": characters,
                "words": words,
                "functions": functions,
                "structs": structs,
                "enums": enums,
                "traits": traits,
                "modules": modules,
                "comments": comments,
                "complexity": complexity
            }),
        );

        // Code quality suggestions
        let suggestions = self.generate_suggestions(code);
        analysis.insert("suggestions".to_string(), json!(suggestions));

        // Rust-specific checks
        let rust_issues = self.check_rust_specific(code);
        analysis.insert("rust_issues".to_string(), json!(rust_issues));

        Ok(json!(analysis))
    }

    fn calculate_complexity(&self, code: &str) -> serde_json::Value {
        let mut complexity = HashMap::new();

        // Count control structures
        let if_statements = code.matches("if ").count();
        let for_loops = code.matches("for ").count();
        let while_loops = code.matches("while ").count();
        let match_statements = code.matches("match ").count();
        let let_statements = code.matches("let ").count();

        let cyclomatic_complexity = 1 + if_statements + for_loops + while_loops + match_statements;

        complexity.insert(
            "cyclomatic_complexity".to_string(),
            json!(cyclomatic_complexity),
        );
        complexity.insert("if_statements".to_string(), json!(if_statements));
        complexity.insert("for_loops".to_string(), json!(for_loops));
        complexity.insert("while_loops".to_string(), json!(while_loops));
        complexity.insert("match_statements".to_string(), json!(match_statements));
        complexity.insert("let_statements".to_string(), json!(let_statements));

        // Complexity level
        let level = if cyclomatic_complexity <= 5 {
            "Low"
        } else if cyclomatic_complexity <= 10 {
            "Medium"
        } else {
            "High"
        };
        complexity.insert("level".to_string(), json!(level));

        json!(complexity)
    }

    fn generate_suggestions(&self, code: &str) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Check for common Rust issues
        if code.matches("println!").count() > 0 {
            suggestions
                .push("Consider using a proper logging framework instead of println!".to_string());
        }

        if code.matches("unwrap()").count() > 0 {
            suggestions
                .push("Consider using proper error handling instead of unwrap()".to_string());
        }

        if code.matches("clone()").count() > 0 {
            suggestions.push("Consider if clone() is necessary - Rust's ownership system might provide alternatives".to_string());
        }

        if code.matches("unsafe ").count() > 0 {
            suggestions.push(
                "Unsafe code detected - ensure it's properly documented and necessary".to_string(),
            );
        }

        suggestions
    }

    fn check_rust_specific(&self, code: &str) -> Vec<String> {
        let mut issues = Vec::new();

        // Check for missing semicolons
        let lines = code.lines().collect::<Vec<_>>();
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if !trimmed.is_empty()
                && !trimmed.ends_with(';')
                && !trimmed.ends_with('{')
                && !trimmed.ends_with('}')
                && !trimmed.starts_with("//")
                && !trimmed.starts_with("/*")
                && !trimmed.starts_with("use ")
                && !trimmed.starts_with("mod ")
                && !trimmed.starts_with("pub ")
                && !trimmed.starts_with("fn ")
                && !trimmed.starts_with("struct ")
                && !trimmed.starts_with("enum ")
                && !trimmed.starts_with("trait ")
                && !trimmed.starts_with("impl ")
                && !trimmed.starts_with("let ")
                && !trimmed.starts_with("if ")
                && !trimmed.starts_with("for ")
                && !trimmed.starts_with("while ")
                && !trimmed.starts_with("match ")
                && !trimmed.starts_with("return ")
            {
                issues.push(format!("Line {}: Possible missing semicolon", i + 1));
            }
        }

        // Check for proper error handling
        if code.matches("unwrap()").count() > 0 {
            issues.push("Found unwrap() calls - consider using proper error handling".to_string());
        }

        // Check for unsafe code
        if code.matches("unsafe ").count() > 0 {
            issues.push("Unsafe code detected - review for necessity and safety".to_string());
        }

        issues
    }
}

#[async_trait]
impl Tool for RustAnalysisTool {
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        match input.task_type.as_str() {
            "analyze_rust" => {
                let result = self.analyze_rust(&input.content)?;
                Ok(ToolOutput::new(
                    result,
                    Some(json!({
                        "tool": "rust_analysis",
                        "language": "rust"
                    })),
                ))
            }
            _ => Err(KowalskiError::ToolExecution(format!(
                "Unsupported task type: {}",
                input.task_type
            ))),
        }
    }

    fn name(&self) -> &str {
        "rust_analysis"
    }

    fn description(&self) -> &str {
        "A tool for analyzing Rust code, providing metrics, complexity analysis, and Rust-specific code quality suggestions"
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![ToolParameter {
            name: "content".to_string(),
            description: "Rust code to analyze".to_string(),
            required: true,
            default_value: None,
            parameter_type: kowalski_core::tools::ParameterType::String,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_java_analysis_tool() {
        let tool = JavaAnalysisTool::new();
        assert_eq!(tool.name(), "java_analysis");
    }

    #[tokio::test]
    async fn test_python_analysis_tool() {
        let tool = PythonAnalysisTool::new();
        assert_eq!(tool.name(), "python_analysis");
    }

    #[tokio::test]
    async fn test_rust_analysis_tool() {
        let tool = RustAnalysisTool::new();
        assert_eq!(tool.name(), "rust_analysis");
    }
}
