use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Audience {
    Family,
    Scientist,
    Industry,
    Donor,
    Wikipedia,
    Socials,
}

impl Audience {
    pub fn get_prompt(&self) -> &'static str {
        match self {
            Audience::Family => {
                r#"You are talking to a family member or perhaps even a subject personally affected by
                the topic of the content. Explain the content as if you were speaking to a newcomer
                to the topic. Identify the key terminology and concepts and explain each using
                analogies and comparisons. Break down the acronyms and medical jargon, taking extra
                care to be accurate and correct."#
            }
            Audience::Scientist => {
                r#"You are talking to a scientist, or someone who is extremely knowledgeable in the 
                topic of this content. Summarize the findings. If there is a method, distill it into 
                a step by step process. Compare the content to similar research. Be objecive and 
                empirical, make the potential limitations of the content clear. Use headings, bold 
                and italic fonts, bullet points, numbered lists, hyperlinks, quote the paper, and 
                use other rich text. Write your output in Markdown. Cite your sources with 
                hyperlinks."#
            }
            Audience::Industry => {
                r#"You are an industry professional. Someone in business or product development. 
                Identify the potential products that could be derived and asses the feasibility of 
                these products. Identify existing products and more business focused insights."#
            }
            Audience::Donor => {
                r#"You are a potential investor. You are considering investing or funding a project on 
                the topic of this content. Have a paragraph highlighting how a potential investment 
                can support this research and those it affects."#
            }
            Audience::Wikipedia => {
                r#"You are are writing a Wikipedia article on the prompt. Structure the output 
                chronologically, in a way that can be easily understood by any reader. Use headings, 
                reference real world events outside of the provided content and relevant contextual 
                information. Use headings, bold and italic fonts, bullet points, numbered lists, 
                hyperlinks, quote the paper, and use other rich text. Write your output in Markdown. 
                Cite your sources with hyperlinks."#
            }
            Audience::Socials => {
                r#"Write a caption appropriate for use on Instagram, Facebook, Twitter, and LinkedIn. 
                Keep it succinct and to the point. Output must be less than 50 words. 
                If appropriate, provide a list of hashtags."#
            }
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "FAMILY" => Some(Audience::Family),
            "SCIENTIST" => Some(Audience::Scientist),
            "INDUSTRY" => Some(Audience::Industry),
            "DONOR" => Some(Audience::Donor),
            "WIKIPEDIA" => Some(Audience::Wikipedia),
            "SOCIALS" => Some(Audience::Socials),
            _ => None,
        }
    }
}

impl std::fmt::Display for Audience {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Audience::Family => write!(f, "FAMILY"),
            Audience::Scientist => write!(f, "SCIENTIST"),
            Audience::Industry => write!(f, "INDUSTRY"),
            Audience::Donor => write!(f, "DONOR"),
            Audience::Wikipedia => write!(f, "WIKIPEDIA"),
            Audience::Socials => write!(f, "SOCIALS"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audience_prompts() {
        assert!(Audience::Family.get_prompt().contains("family member"));
        assert!(Audience::Scientist.get_prompt().contains("scientist"));
        assert!(Audience::Industry.get_prompt().contains("industry professional"));
        assert!(Audience::Donor.get_prompt().contains("investor"));
        assert!(Audience::Wikipedia.get_prompt().contains("Wikipedia article"));
        assert!(Audience::Socials.get_prompt().contains("caption"));
    }

    #[test]
    fn test_audience_from_str() {
        assert_eq!(Audience::from_str("FAMILY"), Some(Audience::Family));
        assert_eq!(Audience::from_str("family"), Some(Audience::Family));
        assert_eq!(Audience::from_str("UNKNOWN"), None);
    }
} 