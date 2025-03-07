use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Style {
    Vector,
    Realistic,
    Artistic,
}

impl Style {
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

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "VECTOR" => Some(Style::Vector),
            "REALISTIC" => Some(Style::Realistic),
            "ARTISTIC" => Some(Style::Artistic),
            _ => None,
        }
    }
}

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

    #[test]
    fn test_style_prompts() {
        assert!(Style::Vector.get_prompt().contains("vector art style"));
        assert!(Style::Realistic.get_prompt().contains("realistic art style"));
        assert!(Style::Artistic.get_prompt().contains("artistic style"));
    }

    #[test]
    fn test_style_from_str() {
        assert_eq!(Style::from_str("VECTOR"), Some(Style::Vector));
        assert_eq!(Style::from_str("REALISTIC"), Some(Style::Realistic));
        assert_eq!(Style::from_str("ARTISTIC"), Some(Style::Artistic));
        assert_eq!(Style::from_str("UNKNOWN"), None);
    }
} 