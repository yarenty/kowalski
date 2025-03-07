/// Style: Because every AI needs a fashion sense, even if it's questionable.
/// "Styles are like fashion trends - they come and go, but the cringe remains."
/// 
/// This enum defines different artistic styles that the AI should use when creating illustrations.
/// Think of it as giving your AI an art degree, but without the student debt.
use serde::{Deserialize, Serialize};

/// The main enum that defines how the AI should draw things.
/// "Artistic styles are like coffee orders - they're all different but equally pretentious."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Style {
    /// For when you want something that looks like it was drawn by a computer
    /// "Vector art is like a robot - it's precise but lacks soul."
    Vector,
    /// For when you want something that looks like it was drawn by a human
    /// "Realistic art is like a photograph - it's accurate but boring."
    Realistic,
    /// For when you want something that looks like it was drawn by an artist on drugs
    /// "Abstract art is like modern poetry - nobody understands it but everyone pretends to."
    Artistic,
}

impl Style {
    /// Gets the prompt for this style.
    /// "Getting the prompt is like getting fashion advice - it's subjective and usually wrong."
    pub fn get_prompt(&self) -> &'static str {
        match self {
            Style::Vector => {
                r#"Use a vector art style. Minimalist, clean, and sharp, vector artwork is comprised of straight lines and
                points with intentional curves."#
            }
            Style::Realistic => {
                r#"Use a realistic art style. Detailed, natural, and most resembling what your prompt would look like in real life."#
            }
            Style::Artistic => {
                r#"Use an artistic style. Creative, stylistic, and opinionated. Pair with more samples to generate several different styles."#
            }
        }
    }

    /// Creates a style from a string, because apparently we can't trust users to use enums directly.
    /// "String parsing is like fortune telling - it works until it doesn't."
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "VECTOR" => Some(Style::Vector),
            "REALISTIC" => Some(Style::Realistic),
            "ARTISTIC" => Some(Style::Artistic),
            _ => None,
        }
    }
}

/// Makes the style printable, because apparently we need to see what we're dealing with.
/// "Display implementations are like mirrors - they show us what we want to see."
impl std::fmt::Display for Style {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Style::Vector => write!(f, "VECTOR"),
            Style::Realistic => write!(f, "REALISTIC"),
            Style::Artistic => write!(f, "ARTISTIC"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests the style prompts
    /// "Testing styles is like testing fashion - it's all about the look."
    #[test]
    fn test_style_prompts() {
        assert!(Style::Vector.get_prompt().contains("vector art style"));
        assert!(Style::Realistic.get_prompt().contains("realistic art style"));
        assert!(Style::Artistic.get_prompt().contains("artistic style"));
    }

    /// Tests the style string parsing
    /// "Testing string parsing is like testing fortune cookies - it's mostly guesswork."
    #[test]
    fn test_style_from_str() {
        assert_eq!(Style::from_str("VECTOR"), Some(Style::Vector));
        assert_eq!(Style::from_str("REALISTIC"), Some(Style::Realistic));
        assert_eq!(Style::from_str("ARTISTIC"), Some(Style::Artistic));
        assert_eq!(Style::from_str("UNKNOWN"), None);
    }
} 