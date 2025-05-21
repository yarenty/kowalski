use serde::{Deserialize, Serialize};
use kowalski_core::config::Config;
use kowalski_agent_template::config::TemplateAgentConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAgentConfig {
    /// Base template configuration
    pub template: TemplateAgentConfig,

    /// Maximum file size to process (in bytes)
    pub max_file_size: usize,

    /// Maximum number of files to process in a single operation
    pub max_files_per_operation: usize,

    /// Whether to enable syntax highlighting
    pub enable_syntax_highlighting: bool,

    /// Whether to enable code formatting
    pub enable_code_formatting: bool,

    /// Whether to enable code analysis
    pub enable_code_analysis: bool,

    /// Whether to enable code refactoring
    pub enable_code_refactoring: bool,

    /// Whether to enable documentation generation
    pub enable_documentation: bool,

    /// Whether to enable test generation
    pub enable_test_generation: bool,

    /// Whether to enable dependency analysis
    pub enable_dependency_analysis: bool,

    /// Whether to enable security analysis
    pub enable_security_analysis: bool,

    /// Whether to enable performance analysis
    pub enable_performance_analysis: bool,

    /// Whether to enable code metrics
    pub enable_code_metrics: bool,

    /// Whether to enable code duplication detection
    pub enable_duplication_detection: bool,

    /// Whether to enable code complexity analysis
    pub enable_complexity_analysis: bool,

    /// Whether to enable code coverage analysis
    pub enable_coverage_analysis: bool,

    /// Whether to enable code style checking
    pub enable_style_checking: bool,

    /// Whether to enable code linting
    pub enable_linting: bool,

    /// Whether to enable code type checking
    pub enable_type_checking: bool,

    /// Whether to enable code static analysis
    pub enable_static_analysis: bool,

    /// Whether to enable code dynamic analysis
    pub enable_dynamic_analysis: bool,

    /// Whether to enable code profiling
    pub enable_profiling: bool,

    /// Whether to enable code debugging
    pub enable_debugging: bool,

    /// Whether to enable code tracing
    pub enable_tracing: bool,

    /// Whether to enable code logging
    pub enable_logging: bool,

    /// Whether to enable code monitoring
    pub enable_monitoring: bool,

    /// Whether to enable code metrics collection
    pub enable_metrics_collection: bool,

    /// Whether to enable code reporting
    pub enable_reporting: bool,

    /// Whether to enable code visualization
    pub enable_visualization: bool,

    /// Whether to enable code documentation
    pub enable_documentation_generation: bool,

    /// Whether to enable code refactoring
    pub enable_refactoring: bool,

    /// Whether to enable code optimization
    pub enable_optimization: bool,

    /// Whether to enable code security
    pub enable_security: bool,

    /// Whether to enable code performance
    pub enable_performance: bool,

    /// Whether to enable code quality
    pub enable_quality: bool,

    /// Whether to enable code maintainability
    pub enable_maintainability: bool,

    /// Whether to enable code reliability
    pub enable_reliability: bool,

    /// Whether to enable code portability
    pub enable_portability: bool,

    /// Whether to enable code reusability
    pub enable_reusability: bool,

    /// Whether to enable code testability
    pub enable_testability: bool,

    /// Whether to enable code understandability
    pub enable_understandability: bool,

    /// Whether to enable code modifiability
    pub enable_modifiability: bool,

    /// Whether to enable code efficiency
    pub enable_efficiency: bool,

    /// Whether to enable code effectiveness
    pub enable_effectiveness: bool,

    /// Whether to enable code correctness
    pub enable_correctness: bool,

    /// Whether to enable code completeness
    pub enable_completeness: bool,

    /// Whether to enable code consistency
    pub enable_consistency: bool,

    /// Whether to enable code traceability
    pub enable_traceability: bool,

    /// Whether to enable code verifiability
    pub enable_verifiability: bool,

}

impl Default for CodeAgentConfig {
    fn default() -> Self {
        Self {
            template: TemplateAgentConfig::default(),
            max_file_size: 1024 * 1024, // 1MB
            max_files_per_operation: 100,
            enable_syntax_highlighting: true,
            enable_code_formatting: true,
            enable_code_analysis: true,
            enable_code_refactoring: true,
            enable_documentation: true,
            enable_test_generation: true,
            enable_dependency_analysis: true,
            enable_security_analysis: true,
            enable_performance_analysis: true,
            enable_code_metrics: true,
            enable_duplication_detection: true,
            enable_complexity_analysis: true,
            enable_coverage_analysis: true,
            enable_style_checking: true,
            enable_linting: true,
            enable_type_checking: true,
            enable_static_analysis: true,
            enable_dynamic_analysis: true,
            enable_profiling: true,
            enable_debugging: true,
            enable_tracing: true,
            enable_logging: true,
            enable_monitoring: true,
            enable_metrics_collection: true,
            enable_reporting: true,
            enable_visualization: true,
            enable_documentation_generation: true,
            enable_refactoring: true,
            enable_optimization: true,
            enable_security: true,
            enable_performance: true,
            enable_quality: true,
            enable_maintainability: true,
            enable_reliability: true,
            enable_portability: true,
            enable_reusability: true,
            enable_testability: true,
            enable_understandability: true,
            enable_modifiability: true,
            enable_efficiency: true,
            enable_effectiveness: true,
            enable_correctness: true,
            enable_completeness: true,
            enable_consistency: true,
            enable_traceability: true,
            enable_verifiability: true,
        }
    }
}

impl From<Config> for CodeAgentConfig {
    fn from(config: Config) -> Self {
        let mut code_config = Self::default();
        code_config.template = TemplateAgentConfig::from(config);
        code_config
    }
} 