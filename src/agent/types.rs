/// Types module: Because we need more than just strings and numbers
/// "Type systems are like relationship counselors - they prevent a lot of mistakes before they happen." - A Type Theorist

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<String>,
    pub stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StreamResponse {
    pub content: String,
    pub done: bool,
} 