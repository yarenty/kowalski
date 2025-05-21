use serde::{Deserialize, Serialize};

/// Role: The AI's personality for this conversation.
/// "Roles are like costumes - they change how you act but not who you are."
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub description: String,
    pub audience: Option<Audience>,
    pub preset: Option<Preset>,
    pub style: Option<Style>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Audience {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Style {
    pub name: String,
    pub description: String,
}

impl Role {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            audience: None,
            preset: None,
            style: None,
        }
    }

    pub fn with_audience(mut self, audience: Audience) -> Self {
        self.audience = Some(audience);
        self
    }

    pub fn with_preset(mut self, preset: Preset) -> Self {
        self.preset = Some(preset);
        self
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    pub fn get_prompt(&self) -> String {
        format!(
            "You are {}. {}",
            self.name,
            self.description
        )
    }

    pub fn get_audience(&self) -> Option<&Audience> {
        self.audience.as_ref()
    }

    pub fn get_preset(&self) -> Option<&Preset> {
        self.preset.as_ref()
    }

    pub fn get_style(&self) -> Option<&Style> {
        self.style.as_ref()
    }
}

impl Audience {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
        }
    }

    pub fn get_prompt(&self) -> String {
        format!(
            "You are speaking to {}. {}",
            self.name,
            self.description
        )
    }
}

impl Preset {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
        }
    }

    pub fn get_prompt(&self) -> String {
        format!(
            "Use the following preset: {}. {}",
            self.name,
            self.description
        )
    }
}

impl Style {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
        }
    }

    pub fn get_prompt(&self) -> String {
        format!(
            "Use the following style: {}. {}",
            self.name,
            self.description
        )
    }
} 