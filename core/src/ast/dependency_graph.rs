use std::{collections::HashMap, path::PathBuf};

use crate::ast::entity_mapper::EntityChange;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct DownstreamImpact {
    pub changed_entity: String,
    pub impacted_file: PathBuf,
}

pub struct DependencyGraph {
    pub symbol_references: HashMap<String, Vec<PathBuf>>,
}

impl DependencyGraph {
    pub fn empty() -> Self {
        Self {
            symbol_references: HashMap::new(),
        }
    }

    pub fn find_downstream_impact(&self, changed_entities: &[EntityChange]) -> Vec<DownstreamImpact> {
        let mut out = Vec::new();
        for e in changed_entities {
            if let Some(files) = self.symbol_references.get(&e.entity_name) {
                for f in files {
                    if *f != e.file_path {
                        out.push(DownstreamImpact {
                            changed_entity: e.entity_name.clone(),
                            impacted_file: f.clone(),
                        });
                    }
                }
            }
        }
        out
    }

    pub fn build_from_repo(repo_root: &std::path::Path) -> Self {
        let mut symbol_references: HashMap<String, Vec<PathBuf>> = HashMap::new();
        for entry in WalkDir::new(repo_root).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path().to_path_buf();
            let ext = path.extension().and_then(|x| x.to_str()).unwrap_or_default();
            if !matches!(ext, "ts" | "tsx" | "js" | "jsx" | "rs" | "py" | "go") {
                continue;
            }
            let Ok(src) = std::fs::read_to_string(&path) else {
                continue;
            };
            for line in src.lines() {
                let t = line.trim();
                // Lightweight symbol index from import/use statements.
                if t.starts_with("import ") || t.starts_with("use ") || t.starts_with("from ") {
                    for token in t.split(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
                        if token.len() >= 3 && token.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
                            symbol_references.entry(token.to_string()).or_default().push(path.clone());
                        }
                    }
                }
            }
        }
        Self { symbol_references }
    }
}
