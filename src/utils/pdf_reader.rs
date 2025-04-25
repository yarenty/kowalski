use crate::utils::KowalskiError;
use pdf_extract::extract_text;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub enum PdfReaderError {
    FileError(std::io::Error),
    PdfError(String),
    InvalidPath(String),
}

impl fmt::Display for PdfReaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PdfReaderError::FileError(e) => write!(f, "File error: {}", e),
            PdfReaderError::PdfError(e) => write!(f, "PDF error: {}", e),
            PdfReaderError::InvalidPath(e) => write!(f, "Invalid path: {}", e),
        }
    }
}

impl Error for PdfReaderError {}

impl From<std::io::Error> for PdfReaderError {
    fn from(err: std::io::Error) -> Self {
        PdfReaderError::FileError(err)
    }
}

pub struct PdfReader {}

impl PdfReader {
    pub fn new() -> Self {
        Self {}
    }

    /// Reads text content from a PDF file
    ///
    /// # Arguments
    /// * `file_path` - Path to the PDF file
    ///
    /// # Returns
    /// * `Result<String, KowalskiError>` - The extracted text content or an error
    pub fn read_pdf(&self, file_path: &str) -> Result<String, KowalskiError> {
        // Verify file exists and is a PDF
        if !Path::new(file_path).exists() {
            return Err(KowalskiError::InvalidPath(format!(
                "File not found: {}",
                file_path
            )));
        }

        if !file_path.to_lowercase().ends_with(".pdf") {
            return Err(KowalskiError::InvalidPath("File must be a PDF".to_string()));
        }

        // Extract text directly from the PDF file
        match extract_text(file_path) {
            Ok(text) => Ok(text),
            Err(e) => Err(KowalskiError::PdfError(e.to_string())),
        }
    }

    /// Reads text content from a PDF file and saves it to a text file
    ///
    /// # Arguments
    /// * `pdf_path` - Path to the PDF file
    /// * `output_path` - Path where the text file should be saved
    ///
    /// # Returns
    /// * `Result<(), KowalskiError>` - Success or error
    #[allow(dead_code)]
    pub fn pdf_to_text(pdf_path: &str, output_path: &str) -> Result<(), KowalskiError> {
        let reader = Self::new();
        let text = reader.read_pdf(pdf_path)?;
        fs::write(output_path, text)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn read_pdf_file(pdf_path: &str) -> Result<String, KowalskiError> {
        let reader = Self::new();
        reader.read_pdf(pdf_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_file() {
        let reader = PdfReader::new();
        let result = reader.read_pdf("nonexistent.pdf");
        assert!(matches!(result, Err(KowalskiError::InvalidPath(_))));
    }

    #[test]
    fn test_invalid_extension() {
        let reader = PdfReader::new();
        let result = reader.read_pdf("test.txt");
        assert!(matches!(result, Err(KowalskiError::InvalidPath(_))));
    }
}
