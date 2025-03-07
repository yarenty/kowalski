/// Preset: Because every AI needs a script, even if it's improvising.
/// "Presets are like microwave settings - they work, but nobody knows why."
///
/// This enum defines different preset behaviors that the AI should follow.
/// Think of it as giving your AI a personality, but without the expensive therapy sessions.
use serde::{Deserialize, Serialize};

/// The main enum that defines how the AI should behave.
/// "Presets are like personality types - they're all different but equally annoying."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Preset {
    /// For when you want the AI to simplify things
    /// "Simplification is like dieting - it's easier said than done."
    Simplify,
    /// For when you want the AI to explain terminology
    /// "Terminology is like a foreign language - it sounds fancy until you understand it."
    Terminology,
    /// For when you want the AI to discuss applications
    /// "Applications are like ideas - they're great until you try to implement them."
    Applications,
    /// For when you want the AI to be optimistic
    /// "Optimism is like a weather forecast - it's usually wrong but we keep checking."
    Optimistic,
    /// For when you want the AI to analyze things
    /// "Analysis is like detective work - it's all fun until you find the body."
    Analyzed,
    /// For when you want the AI to list takeaways
    /// "Takeaways are like leftovers - they're better the next day."
    Takeaways,
    /// For when you want the AI to ask questions
    /// "Questions are like children - they never stop coming."
    Questions,
}

impl Preset {
    /// Gets the prompt for this preset.
    /// "Getting the prompt is like getting directions - they make sense until you try to follow them."
    pub fn get_prompt(&self) -> &'static str {
        match self {
            Preset::Simplify => {
                r#"Respectfully and with dignity, explain the content as if you were speaking to a 
                newcomer to the topic."#
            }
            Preset::Terminology => {
                r#"Identify the key terminology and concepts in point form and explain each using 
                analogies and comparisons. Break down the acronyms and medical jargon, taking extra 
                care to be as accurate and correct as possible."#
            }
            Preset::Applications => {
                r#"Describe the applications of the content, and the implications that this research 
                has on the field. Answer with why this research is important and necessary."#
            }
            Preset::Optimistic => {
                r#"Optimistically identify the directions that this research can go, and the potential 
                benefits for the user."#
            }
            Preset::Analyzed => {
                r#"Objectively and realistically analyze the key results and outcomes of the content. 
                list the most promising and clear statistics if provided in the content."#
            }
            Preset::Takeaways => {
                r#"List the key takeaways from the content. They should be comprehensive and make no 
                inferences beyond that the information provided in the content."#
            }
            Preset::Questions => {
                r#"Answer the 6 questions in a list: 
                (1) What do the author(s) want to know (motivation)?
                (2) What did they do (approach/methods)?
                (3) Why was it done that way (context within the field)?
                (4) What do the results show (figures and data tables)?
                (5) How did the author(s) interpret the results (interpretation/discussion)?
                (6) What should be done next?
                (Regarding this last question, the author(s) may provide some suggestions in the 
                discussion, but the key is to ask yourself what you think should come next.)"#
            }
        }
    }

    /// Creates a preset from a string, because apparently we can't trust users to use enums directly.
    /// "String parsing is like fortune telling - it works until it doesn't."
    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "SIMPLIFY" => Some(Preset::Simplify),
            "TERMINOLOGY" => Some(Preset::Terminology),
            "APPLICATIONS" => Some(Preset::Applications),
            "OPTIMISTIC" => Some(Preset::Optimistic),
            "ANALYZED" => Some(Preset::Analyzed),
            "TAKEAWAYS" => Some(Preset::Takeaways),
            "QUESTIONS" => Some(Preset::Questions),
            _ => None,
        }
    }
}

/// Makes the preset printable, because apparently we need to see what we're dealing with.
/// "Display implementations are like mirrors - they show us what we want to see."
impl std::fmt::Display for Preset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Preset::Simplify => write!(f, "SIMPLIFY"),
            Preset::Terminology => write!(f, "TERMINOLOGY"),
            Preset::Applications => write!(f, "APPLICATIONS"),
            Preset::Optimistic => write!(f, "OPTIMISTIC"),
            Preset::Analyzed => write!(f, "ANALYZED"),
            Preset::Takeaways => write!(f, "TAKEAWAYS"),
            Preset::Questions => write!(f, "QUESTIONS"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests the preset prompts
    /// "Testing presets is like testing recipes - it's all about the taste."
    #[test]
    fn test_preset_prompts() {
        assert!(Preset::Simplify.get_prompt().contains("newcomer"));
        assert!(Preset::Terminology.get_prompt().contains("terminology"));
        assert!(Preset::Applications.get_prompt().contains("applications"));
        assert!(Preset::Optimistic.get_prompt().contains("optimistically"));
        assert!(Preset::Analyzed.get_prompt().contains("analyze"));
        assert!(Preset::Takeaways.get_prompt().contains("takeaways"));
        assert!(Preset::Questions.get_prompt().contains("questions"));
    }

    /// Tests the preset string parsing
    /// "Testing string parsing is like testing fortune cookies - it's mostly guesswork."
    #[test]
    fn test_preset_from_str() {
        assert_eq!(Preset::from_str("SIMPLIFY"), Some(Preset::Simplify));
        assert_eq!(Preset::from_str("simplify"), Some(Preset::Simplify));
        assert_eq!(Preset::from_str("UNKNOWN"), None);
    }
}
