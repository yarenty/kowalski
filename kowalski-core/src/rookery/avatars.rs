//! Penguin avatar ids for UI display (maps to `ui/src/assets/pinguins/<id>.png`).

/// Infer a stable avatar id from pipeline step `kind` and `name` (kebab-case step id).
pub fn infer_penguin_avatar(kind: &str, name: &str) -> String {
    let n = name.to_lowercase().replace('-', "_");
    let k = kind.to_lowercase();

    for (pat, avatar) in NAME_AVATAR_PATTERNS {
        if n.contains(pat) {
            return (*avatar).to_string();
        }
    }

    match k.as_str() {
        "ingest" => "ingest",
        "deliver" | "final" => "deliver",
        "ask" => "ask",
        "lint" => "lint",
        "compile" => "compile",
        "investigate" => "investigate",
        "structure" => "structure",
        "scaffold" => "scaffold",
        "process" | "step" => "process",
        _ => "default",
    }
    .into()
}

/// Fill missing `avatar` on every penguin in the draft.
pub fn assign_penguin_avatars(draft: &mut crate::rookery::types::RookeryDraft) {
    for p in &mut draft.penguins {
        if p.avatar.as_deref().unwrap_or("").trim().is_empty() {
            p.avatar = Some(infer_penguin_avatar(&p.kind, &p.name));
        }
    }
}

const NAME_AVATAR_PATTERNS: &[(&str, &str)] = &[
    ("mock_builder", "mock_builder"),
    ("mock", "mock_builder"),
    ("todo_generator", "todo_generator"),
    ("todo_list", "todo_generator"),
    ("todo", "todo_generator"),
    ("structure", "structure"),
    ("investigate", "investigate"),
    ("scaffold", "scaffold"),
    ("research", "researcher"),
    ("explorer", "explorer"),
    ("security", "security"),
    ("advisor", "advisor"),
    ("translator", "translator"),
    ("coordinator", "coordinator"),
    ("director", "director"),
    ("compile", "compile"),
    ("thinker", "thinker"),
    ("ingest", "ingest"),
    ("deliver", "deliver"),
    ("lint", "lint"),
    ("ask", "ask"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_kind_defaults() {
        assert_eq!(infer_penguin_avatar("ingest", "collect"), "ingest");
        assert_eq!(infer_penguin_avatar("deliver", "ship"), "deliver");
        assert_eq!(infer_penguin_avatar("ask", "qa"), "ask");
        assert_eq!(infer_penguin_avatar("lint", "check"), "lint");
    }

    #[test]
    fn infers_name_patterns() {
        assert_eq!(infer_penguin_avatar("step", "mock-builder"), "mock_builder");
        assert_eq!(
            infer_penguin_avatar("step", "todo-generator"),
            "todo_generator"
        );
        assert_eq!(infer_penguin_avatar("step", "investigate"), "investigate");
    }

    #[test]
    fn unknown_falls_back_to_default() {
        assert_eq!(infer_penguin_avatar("custom", "xyzzy"), "default");
    }
}
