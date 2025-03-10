use crate::utils::KowalskiError;

pub struct PaperCleaner {}

impl PaperCleaner {
    pub fn new() -> Self {
        Self {}
    }

    /// Cleans academic paper text by removing references and fixing line breaks
    ///
    /// # Arguments
    /// * `text` - The input text to clean
    ///
    /// # Returns
    /// * `Result<String, KowalskiError>` - The cleaned text or an error
    pub fn clean(&self, text: &str) -> Result<String, KowalskiError> {
        if text.is_empty() {
            return Err(KowalskiError::InvalidInput(
                "Input text cannot be empty".to_string(),
            ));
        }

        // Remove references section
        let text = Self::remove_references_section(text)?;

        // Fix line breaks with hyphens
        let text = Self::fix_hyphenated_line_breaks(&text)?;

        Ok(text)
    }

    /// Removes the references section and everything after it
    ///
    /// # Arguments
    /// * `text` - The input text
    ///
    /// # Returns
    /// * `Result<String, KowalskiError>` - The text without references or an error
    fn remove_references_section(text: &str) -> Result<String, KowalskiError> {
        // Common variations of the references section header
        let reference_headers = [
            "References",
            "REFERENCES",
            "Bibliography",
            "BIBLIOGRAPHY",
            "References and Notes",
            "REFERENCES AND NOTES",
        ];

        // Find the earliest occurrence of any reference header
        let mut earliest_pos = text.len();
        for header in reference_headers.iter() {
            if let Some(pos) = text.find(header) {
                earliest_pos = earliest_pos.min(pos);
            }
        }

        // If we found a references section, return only the text before it
        if earliest_pos < text.len() {
            Ok(text[..earliest_pos].to_string())
        } else {
            Ok(text.to_string())
        }
    }

    /// Fixes line breaks that occur in the middle of hyphenated words
    ///
    /// # Arguments
    /// * `text` - The input text
    ///
    /// # Returns
    /// * `Result<String, KowalskiError>` - The text with fixed line breaks or an error
    fn fix_hyphenated_line_breaks(text: &str) -> Result<String, KowalskiError> {
        // Split text into lines
        let lines: Vec<&str> = text.lines().collect();
        let mut result = String::new();
        let mut i = 0;

        while i < lines.len() {
            let current_line = lines[i].trim();

            // Check if current line ends with a hyphen
            if current_line.ends_with('-') {
                // Look ahead for the next line
                if i + 1 < lines.len() {
                    let next_line = lines[i + 1].trim();
                    // Combine the lines, removing the hyphen
                    result.push_str(&current_line[..current_line.len() - 1]);
                    result.push_str(next_line);
                    result.push(' ');
                    i += 2;
                    continue;
                }
            }

            result.push_str(current_line);
            result.push('\n');
            i += 1;
        }

        Ok(result.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_references() {
        let text = "Main content here.\n\nReferences\n1. First reference\n2. Second reference";
        let result = PaperCleaner::remove_references_section(text).unwrap();
        assert_eq!(result, "Main content here.\n\n");
    }

    #[test]
    fn test_fix_hyphenated_line_breaks() {
        let text = "This is a hyphen-\nated word.";
        let result = PaperCleaner::fix_hyphenated_line_breaks(text).unwrap();
        assert_eq!(result, "This is a hyphenated word.");
    }

    #[test]
    fn test_empty_input() {
        let result = PaperCleaner::clean("");
        assert!(matches!(result, Err(KowalskiError::InvalidInput(_))));
    }
}
