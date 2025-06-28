use kowalski_core::error::KowalskiError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CodeAgentError {
    #[error("Core error: {0}")]
    Core(#[from] KowalskiError),

    #[error("Parser error: {0}")]
    Parser(String),

    #[error("Analyzer error: {0}")]
    Analyzer(String),

    #[error("Refactoring error: {0}")]
    Refactoring(String),

    #[error("Documentation error: {0}")]
    Documentation(String),

    #[error("Test generation error: {0}")]
    TestGeneration(String),

    #[error("Dependency analysis error: {0}")]
    DependencyAnalysis(String),

    #[error("Security analysis error: {0}")]
    SecurityAnalysis(String),

    #[error("Performance analysis error: {0}")]
    PerformanceAnalysis(String),

    #[error("Code metrics error: {0}")]
    CodeMetrics(String),

    #[error("Duplication detection error: {0}")]
    DuplicationDetection(String),

    #[error("Complexity analysis error: {0}")]
    ComplexityAnalysis(String),

    #[error("Coverage analysis error: {0}")]
    CoverageAnalysis(String),

    #[error("Style checking error: {0}")]
    StyleChecking(String),

    #[error("Linting error: {0}")]
    Linting(String),

    #[error("Type checking error: {0}")]
    TypeChecking(String),

    #[error("Static analysis error: {0}")]
    StaticAnalysis(String),

    #[error("Dynamic analysis error: {0}")]
    DynamicAnalysis(String),

    #[error("Profiling error: {0}")]
    Profiling(String),

    #[error("Debugging error: {0}")]
    Debugging(String),

    #[error("Tracing error: {0}")]
    Tracing(String),

    #[error("Logging error: {0}")]
    Logging(String),

    #[error("Monitoring error: {0}")]
    Monitoring(String),

    #[error("Metrics collection error: {0}")]
    MetricsCollection(String),

    #[error("Reporting error: {0}")]
    Reporting(String),

    #[error("Visualization error: {0}")]
    Visualization(String),

    #[error("Documentation generation error: {0}")]
    DocumentationGeneration(String),

    #[error("Test generation error: {0}")]
    TestGenerationError(String),

    #[error("Refactoring error: {0}")]
    RefactoringError(String),

    #[error("Optimization error: {0}")]
    Optimization(String),

    #[error("Security error: {0}")]
    Security(String),

    #[error("Performance error: {0}")]
    Performance(String),

    #[error("Quality error: {0}")]
    Quality(String),

    #[error("Maintainability error: {0}")]
    Maintainability(String),

    #[error("Reliability error: {0}")]
    Reliability(String),

    #[error("Portability error: {0}")]
    Portability(String),

    #[error("Reusability error: {0}")]
    Reusability(String),

    #[error("Testability error: {0}")]
    Testability(String),

    #[error("Understandability error: {0}")]
    Understandability(String),

    #[error("Modifiability error: {0}")]
    Modifiability(String),

    #[error("Efficiency error: {0}")]
    Efficiency(String),

    #[error("Effectiveness error: {0}")]
    Effectiveness(String),

    #[error("Correctness error: {0}")]
    Correctness(String),

    #[error("Completeness error: {0}")]
    Completeness(String),

    #[error("Consistency error: {0}")]
    Consistency(String),

    #[error("Traceability error: {0}")]
    Traceability(String),

    #[error("Verifiability error: {0}")]
    Verifiability(String),
}
