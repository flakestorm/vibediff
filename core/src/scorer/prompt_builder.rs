use crate::{ast::entity_mapper::EntityChange, git::intent::IntentRecord};

pub const VIBE_SCORER_SYSTEM_PROMPT: &str =
    "You are VibeDiff, a strict semantic code auditor. Return valid JSON only.";

pub fn build_user_prompt(intent: &IntentRecord, entities: &[EntityChange]) -> String {
    let entities_json = serde_json::to_string_pretty(entities).unwrap_or_else(|_| "[]".to_string());
    format!(
        "## DEVELOPER INTENT\nCommit Message: {}\nCommit Type: {:?}\nScope: {}\n\n## CODE CHANGES\n{}\n",
        intent.commit_message,
        intent.commit_type,
        intent.scope.as_deref().unwrap_or("(none)"),
        entities_json
    )
}
