use anyhow::Result;
use tree_sitter::Parser;
use uuid::Uuid;

use crate::{
    ast::{
        entity_mapper::{ChangeKind, EntityChange, EntityType, SideEffect},
        languages::{go, python, rust, typescript},
        upwalker::find_enclosing_anchor,
    },
    git::diff_parser::{DiffHunk, Language},
};

pub struct AstEngine;

impl AstEngine {
    pub fn analyze_file(
        file_path: &std::path::Path,
        source: &str,
        hunks: &[DiffHunk],
        language: Language,
    ) -> Result<Vec<EntityChange>> {
        match language {
            Language::TypeScript => Self::analyze_typescript_file(file_path, source, hunks),
            Language::Rust => Self::analyze_with_language(file_path, source, hunks, language, &rust::language(), &["function_item", "impl_item", "struct_item", "trait_item", "mod_item"]),
            Language::Python => Self::analyze_with_language(file_path, source, hunks, language, &python::language(), &["function_definition", "class_definition", "async_function_definition"]),
            Language::Go => Self::analyze_with_language(file_path, source, hunks, language, &go::language(), &["function_declaration", "method_declaration", "type_declaration"]),
            Language::Unknown => Ok(vec![]),
        }
    }

    pub fn analyze_typescript_file(
        file_path: &std::path::Path,
        source: &str,
        hunks: &[DiffHunk],
    ) -> Result<Vec<EntityChange>> {
        let mut parser = Parser::new();
        parser.set_language(&typescript::language())?;
        let Some(tree) = parser.parse(source, None) else {
            return Ok(vec![]);
        };
        let mut out = Vec::new();
        for hunk in hunks {
            for (line, text) in &hunk.added_lines {
                let line_zero = line.saturating_sub(1) as usize;
                let anchor = find_enclosing_anchor(&tree, line_zero, typescript::anchor_kinds());
                let (entity_name, entity_type) = if let Some(node) = anchor {
                    let kind = node.kind().to_string();
                    (
                        extract_symbol_name(text).unwrap_or(kind.clone()),
                        map_node_kind_to_entity_type(&kind),
                    )
                } else {
                    (
                        extract_symbol_name(text).unwrap_or_else(|| "module".to_string()),
                        EntityType::Module,
                    )
                };
                out.push(EntityChange {
                    entity_id: Uuid::new_v4(),
                    file_path: file_path.to_path_buf(),
                    language: Language::TypeScript,
                    entity_type,
                    entity_name: entity_name.clone(),
                    fully_qualified_name: format!("{}::{entity_name}", file_path.display()),
                    change_kind: detect_change_kind(hunk, *line),
                    changed_lines: *line..=*line,
                    side_effects: detect_side_effects(text),
                });
            }
        }
        Ok(out)
    }

    fn analyze_with_language(
        file_path: &std::path::Path,
        source: &str,
        hunks: &[DiffHunk],
        language: Language,
        ts_lang: &tree_sitter::Language,
        anchors: &[&str],
    ) -> Result<Vec<EntityChange>> {
        let mut parser = Parser::new();
        parser.set_language(ts_lang)?;
        let Some(tree) = parser.parse(source, None) else {
            return Ok(vec![]);
        };
        let mut out = Vec::new();
        for hunk in hunks {
            for (line, text) in &hunk.added_lines {
                let line_zero = line.saturating_sub(1) as usize;
                let anchor = find_enclosing_anchor(&tree, line_zero, anchors);
                let entity_name = anchor.map(|n| n.kind().to_string()).unwrap_or_else(|| "module".to_string());
                out.push(EntityChange {
                    entity_id: Uuid::new_v4(),
                    file_path: file_path.to_path_buf(),
                    language: language.clone(),
                    entity_type: EntityType::Function,
                    entity_name: extract_symbol_name(text).unwrap_or(entity_name.clone()),
                    fully_qualified_name: format!("{}::{entity_name}", file_path.display()),
                    change_kind: detect_change_kind(hunk, *line),
                    changed_lines: *line..=*line,
                    side_effects: detect_side_effects(text),
                });
            }
        }
        Ok(out)
    }
}

fn map_node_kind_to_entity_type(kind: &str) -> EntityType {
    match kind {
        "function_declaration" | "function_expression" | "arrow_function" => EntityType::Function,
        "method_definition" => EntityType::Method,
        "class_declaration" => EntityType::Class,
        "interface_declaration" => EntityType::Interface,
        "type_alias_declaration" => EntityType::Type,
        _ => EntityType::Module,
    }
}

fn detect_side_effects(line: &str) -> Vec<SideEffect> {
    let mut v = Vec::new();
    if line.contains("fetch(") || line.contains("axios.") {
        v.push(SideEffect::ExternalApiCall);
    }
    if line.contains("globalThis.") || line.contains("window.") {
        v.push(SideEffect::GlobalStateRead);
    }
    if line.contains("process.env") {
        v.push(SideEffect::EnvVarAccess);
    }
    v
}

fn detect_change_kind(hunk: &DiffHunk, line: u32) -> ChangeKind {
    let has_add = hunk.added_lines.iter().any(|(l, _)| *l == line);
    let has_remove = hunk.removed_lines.iter().any(|(l, _)| *l == line);
    match (has_add, has_remove) {
        (true, false) => ChangeKind::Added,
        (false, true) => ChangeKind::Removed,
        _ => ChangeKind::Modified,
    }
}

fn extract_symbol_name(line: &str) -> Option<String> {
    let stripped = line.trim_start_matches('+').trim_start_matches('-').trim();
    for kw in ["function ", "class ", "interface ", "type ", "fn ", "def ", "func "] {
        if let Some(rest) = stripped.strip_prefix(kw) {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' )
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}
