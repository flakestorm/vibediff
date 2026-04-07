use std::{collections::HashMap, path::PathBuf};

use crate::git::diff_parser::DiffHunk;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct CssImpact {
    pub css_entity: String,
    pub impacted_file: PathBuf,
}

pub struct CssImpactAnalyzer {
    pub class_usage_index: HashMap<String, Vec<PathBuf>>,
}

impl CssImpactAnalyzer {
    pub fn empty() -> Self {
        Self {
            class_usage_index: HashMap::new(),
        }
    }

    pub fn analyze_css_hunk(&self, hunk: &DiffHunk) -> Vec<CssImpact> {
        let mut impacts = Vec::new();
        for (_, line) in &hunk.removed_lines {
            if let Some(class_name) = extract_css_class(line) {
                if let Some(files) = self.class_usage_index.get(&class_name) {
                    for f in files {
                        impacts.push(CssImpact {
                            css_entity: class_name.clone(),
                            impacted_file: f.clone(),
                        });
                    }
                }
            }
        }
        impacts
    }

    pub fn build_from_repo(repo_root: &std::path::Path) -> Self {
        let mut class_usage_index: HashMap<String, Vec<PathBuf>> = HashMap::new();
        for entry in WalkDir::new(repo_root).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path().to_path_buf();
            let ext = path.extension().and_then(|x| x.to_str()).unwrap_or_default();
            if !matches!(ext, "ts" | "tsx" | "js" | "jsx" | "html" | "css" | "scss") {
                continue;
            }
            let Ok(src) = std::fs::read_to_string(&path) else {
                continue;
            };
            for token in src.split_whitespace() {
                if let Some(cls) = token.strip_prefix("class=\"").or_else(|| token.strip_prefix("className=\"")) {
                    let cleaned = cls.trim_matches('"').trim_matches('\'');
                    for c in cleaned.split_whitespace() {
                        if c.len() >= 2 {
                            class_usage_index.entry(c.to_string()).or_default().push(path.clone());
                        }
                    }
                }
            }
        }
        Self { class_usage_index }
    }
}

fn extract_css_class(line: &str) -> Option<String> {
    let t = line.trim();
    if let Some(rest) = t.strip_prefix('.') {
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if !name.is_empty() {
            return Some(name);
        }
    }
    None
}
