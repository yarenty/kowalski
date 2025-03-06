use std::fs;
use std::path::Path;
use pdf_extract::extract_text;
use std::error::Error;
use std::fmt;

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

pub struct PdfReader;

impl PdfReader {
    /// Reads text content from a PDF file
    /// 
    /// # Arguments
    /// * `file_path` - Path to the PDF file
    /// 
    /// # Returns
    /// * `Result<String, PdfReaderError>` - The extracted text content or an error
    pub fn read_pdf(file_path: &str) -> Result<String, PdfReaderError> {
        // Verify file exists and is a PDF
        if !Path::new(file_path).exists() {
            return Err(PdfReaderError::InvalidPath(format!("File not found: {}", file_path)));
        }

        if !file_path.to_lowercase().ends_with(".pdf") {
            return Err(PdfReaderError::InvalidPath("File must be a PDF".to_string()));
        }

        // Read the PDF file
        let bytes = fs::read(file_path)?;

        // Extract text from PDF
        match extract_text(&bytes) {
            Ok(text) => Ok(text),
            Err(e) => Err(PdfReaderError::PdfError(e.to_string())),
        }
    }

    /// Reads text content from a PDF file and saves it to a text file
    /// 
    /// # Arguments
    /// * `pdf_path` - Path to the PDF file
    /// * `output_path` - Path where the text file should be saved
    /// 
    /// # Returns
    /// * `Result<(), PdfReaderError>` - Success or error
    pub fn pdf_to_text(pdf_path: &str, output_path: &str) -> Result<(), PdfReaderError> {
        let text = Self::read_pdf(pdf_path)?;
        fs::write(output_path, text)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_file() {
        let result = PdfReader::read_pdf("nonexistent.pdf");
        assert!(matches!(result, Err(PdfReaderError::InvalidPath(_))));
    }

    #[test]
    fn test_invalid_extension() {
        let result = PdfReader::read_pdf("test.txt");
        assert!(matches!(result, Err(PdfReaderError::InvalidPath(_))));
    }
} 