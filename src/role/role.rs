/// Role: Because every AI needs a personality, even if it's as dry as a desert.
/// "Roles are like costumes - they make you look different, but you're still the same person underneath."
///
/// This struct defines how the AI should behave in conversations.
/// Think of it as giving your AI a personality makeover, but without the expensive therapy.
use serde::{Deserialize, Serialize};
use crate::role::Audience;
use crate::role::Preset;
use crate::role::Style;

/// The main enum that defines what kind of AI personality we're dealing with.
/// "Translators are like bilingual dictionaries - they know the words, but not the soul."
/// "Illustrators are like artists - they see the world differently, usually through a haze of coffee."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    /// For when you want the AI to translate things
    /// "Translation is like a game of telephone - it starts clear and ends in chaos."
    Translator {
        audience: Option<Audience>,
        preset: Option<Preset>,
    },
    /// For when you want the AI to draw things
    /// "Illustration is like cooking - it's all fun until someone asks for a refund."
    Illustrator { style: Option<Style> },
}

impl Role {
    /// Gets the prompt for this role.
    /// "Getting the prompt is like writing a speech - it's all about the delivery."
    pub fn get_prompt(&self) -> &'static str {
        match self {
            Role::Translator { .. } => {
                r#"You are a highly skilled AI trained in language comprehension and simplification. 
                I would like you to read the following text and simplify it. Do not use first person.
                Remember the key here is to simplify, not necessarily summarize.
                Provide only the output don't reply as if you're talking to someone."#
            }
            Role::Illustrator { .. } => {
                r#"I would like you to read the following prompt and generate an illustration for it.
                Use images, pictures and visuals."#
            }
        }
    }

    /// Gets the audience for this role.
    /// "Getting the audience is like reading minds - impossible but we try anyway."
    pub fn get_audience(&self) -> Option<Audience> {
        match self {
            Role::Translator { audience, .. } => *audience,
            Role::Illustrator { .. } => None,
        }
    }

    /// Gets the preset for this role.
    /// "Getting the preset is like following a recipe - it works until it doesn't."
    pub fn get_preset(&self) -> Option<Preset> {
        match self {
            Role::Translator { preset, .. } => *preset,
            Role::Illustrator { .. } => None,
        }
    }

    /// Gets the style for this role.
    /// "Getting the style is like finding your fashion sense - it's a journey."
    pub fn get_style(&self) -> Option<Style> {
        match self {
            Role::Translator { .. } => None,
            Role::Illustrator { style } => *style,
        }
    }

    /// Creates a translator role that's ready to translate like a boss.
    /// "Translators are like bilingual dictionaries - they know the words, but not the soul."
    pub fn translator(audience: Option<Audience>, preset: Option<Preset>) -> Self {
        Role::Translator { audience, preset }
    }

    /// Creates an illustrator role that's ready to draw like Picasso (or at least try).
    /// "Illustrators are like artists - they see the world differently, usually through a haze of coffee."
    #[allow(dead_code)]
    pub fn illustrator(style: Option<Style>) -> Self {
        Role::Illustrator { style }
    }

    /// Creates a role from a string, because apparently we can't trust users to use enums directly.
    /// "String parsing is like fortune telling - it works until it doesn't."
    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "TRANSLATOR" => Some(Role::Translator {
                audience: None,
                preset: None,
            }),
            "ILLUSTRATOR" => Some(Role::Illustrator { style: None }),
            _ => None,
        }
    }
}

/// Makes the role printable, because apparently we need to see what we're dealing with.
/// "Display implementations are like mirrors - they show us what we want to see."
impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Translator { .. } => write!(f, "TRANSLATOR"),
            Role::Illustrator { .. } => write!(f, "ILLUSTRATOR"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::role::Audience;
    use crate::role::Preset;
    use crate::role::Style;

    /// Tests the role prompts
    /// "Testing prompts is like testing jokes - if you have to explain them, they're not working."
    #[test]
    fn test_role_prompts() {
        assert!(
            Role::Translator {
                audience: None,
                preset: None
            }
            .get_prompt()
            .contains("simplify")
        );
        assert!(
            Role::Illustrator { style: None }
                .get_prompt()
                .contains("illustration")
        );
    }

    /// Tests the role string parsing
    /// "Testing string parsing is like testing fortune cookies - it's mostly guesswork."
    #[test]
    fn test_role_from_str() {
        assert_eq!(
            Role::from_str("TRANSLATOR"),
            Some(Role::Translator {
                audience: None,
                preset: None
            })
        );
        assert_eq!(
            Role::from_str("translator"),
            Some(Role::Translator {
                audience: None,
                preset: None
            })
        );
        assert_eq!(Role::from_str("UNKNOWN"), None);
    }

    /// Tests the translator configuration
    /// "Testing translators is like testing relationships - it's complicated and usually ends in tears."
    #[test]
    fn test_translator_with_config() {
        let role = Role::translator(Some(Audience::Scientist), Some(Preset::Questions));
        assert_eq!(role.get_audience(), Some(Audience::Scientist));
        assert_eq!(role.get_preset(), Some(Preset::Questions));
    }

    /// Tests the illustrator configuration
    /// "Testing illustrators is like testing artists - subjective and slightly chaotic."
    #[test]
    fn test_illustrator_with_style() {
        let role = Role::illustrator(Some(Style::Vector));
        assert_eq!(role.get_style(), Some(Style::Vector));
    }
}
