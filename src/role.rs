use serde::{Deserialize, Serialize};
use crate::audience::Audience;
use crate::preset::Preset;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    Translator {
        audience: Option<Audience>,
        preset: Option<Preset>,
    },
    Illustrator,
}

impl Role {
    pub fn get_prompt(&self) -> &'static str {
        match self {
            Role::Translator { .. } => {
                r#"You are a highly skilled AI trained in language comprehension and simplification. 
                I would like you to read the following text and simplify it. Do not use first person.
                Remember the key here is to simplify, not necessarily summarize.
                Provide only the output don't reply as if you're talking to someone."#
            }
            Role::Illustrator => {
                r#"I would like you to read the following prompt and generate an illustration for it.
                Use images, pictures and visuals."#
            }
        }
    }

    pub fn get_audience(&self) -> Option<Audience> {
        match self {
            Role::Translator { audience, .. } => *audience,
            Role::Illustrator => None,
        }
    }

    pub fn get_preset(&self) -> Option<Preset> {
        match self {
            Role::Translator { preset, .. } => *preset,
            Role::Illustrator => None,
        }
    }

    pub fn translator(audience: Option<Audience>, preset: Option<Preset>) -> Self {
        Role::Translator { audience, preset }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "TRANSLATOR" => Some(Role::Translator { audience: None, preset: None }),
            "ILLUSTRATOR" => Some(Role::Illustrator),
            _ => None,
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Translator { .. } => write!(f, "TRANSLATOR"),
            Role::Illustrator => write!(f, "ILLUSTRATOR"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audience::Audience;
    use crate::preset::Preset;

    #[test]
    fn test_role_prompts() {
        assert!(Role::Translator { audience: None, preset: None }.get_prompt().contains("simplify"));
        assert!(Role::Illustrator.get_prompt().contains("illustration"));
    }

    #[test]
    fn test_role_from_str() {
        assert_eq!(Role::from_str("TRANSLATOR"), Some(Role::Translator { audience: None, preset: None }));
        assert_eq!(Role::from_str("translator"), Some(Role::Translator { audience: None, preset: None }));
        assert_eq!(Role::from_str("UNKNOWN"), None);
    }

    #[test]
    fn test_translator_with_config() {
        let role = Role::translator(Some(Audience::Scientist), Some(Preset::Questions));
        assert_eq!(role.get_audience(), Some(Audience::Scientist));
        assert_eq!(role.get_preset(), Some(Preset::Questions));
    }
} 