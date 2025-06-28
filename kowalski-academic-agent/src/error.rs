use kowalski_core::error::KowalskiError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AcademicAgentError {
    #[error("Core error: {0}")]
    Core(#[from] KowalskiError),

    #[error("Paper parsing error: {0}")]
    PaperParsing(String),

    #[error("Citation error: {0}")]
    Citation(String),

    #[error("Bibliography error: {0}")]
    Bibliography(String),

    #[error("Format conversion error: {0}")]
    FormatConversion(String),

    #[error("Reference error: {0}")]
    Reference(String),

    #[error("Academic search error: {0}")]
    AcademicSearch(String),

    #[error("Content processing error: {0}")]
    ContentProcessing(String),
}

impl From<reqwest::Error> for AcademicAgentError {
    fn from(err: reqwest::Error) -> Self {
        AcademicAgentError::AcademicSearch(err.to_string())
    }
}
