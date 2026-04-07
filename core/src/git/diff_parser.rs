use std::{ops::RangeInclusive, path::PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::git::facade::GitFacade;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Language {
    TypeScript,
    Rust,
    Python,
    Go,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub file_path: PathBuf,
    pub language: Language,
    pub changed_lines: RangeInclusive<u32>,
    pub added_lines: Vec<(u32, String)>,
    pub removed_lines: Vec<(u32, String)>,
    pub raw_hunk: String,
}

pub struct DiffParser {
    git: GitFacade,
}

impl DiffParser {
    pub fn new(repo_root: PathBuf) -> Self {
        Self {
            git: GitFacade::new(repo_root),
        }
    }

    pub fn extract_staged_diff_hunks(&self) -> Result<Vec<DiffHunk>> {
        self.parse_unified_diff(&self.git.diff_staged()?)
    }

    pub fn extract_rev_diff_hunks(&self, rev: &str) -> Result<Vec<DiffHunk>> {
        self.parse_unified_diff(&self.git.diff_rev(rev)?)
    }

    fn parse_unified_diff(&self, raw: &str) -> Result<Vec<DiffHunk>> {
        let mut hunks = Vec::new();
        let mut current_file = PathBuf::new();
        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut raw_hunk = String::new();
        let mut new_line: u32 = 0;
        let mut min_line: u32 = u32::MAX;
        let mut max_line: u32 = 0;

        for line in raw.lines() {
            if let Some(path) = line.strip_prefix("+++ b/") {
                current_file = PathBuf::from(path);
            } else if line.starts_with("@@") {
                if !raw_hunk.is_empty() {
                    hunks.push(self.finish_hunk(
                        current_file.clone(),
                        added.clone(),
                        removed.clone(),
                        raw_hunk.clone(),
                        min_line,
                        max_line,
                    ));
                    added.clear();
                    removed.clear();
                    raw_hunk.clear();
                    min_line = u32::MAX;
                    max_line = 0;
                }
                if let Some(idx) = line.find('+') {
                    let rhs = &line[idx + 1..];
                    let start = rhs
                        .split([',', ' '])
                        .next()
                        .and_then(|x| x.parse::<u32>().ok())
                        .unwrap_or(1);
                    new_line = start;
                }
                raw_hunk.push_str(line);
                raw_hunk.push('\n');
            } else if line.starts_with('+') && !line.starts_with("+++") {
                added.push((new_line, line.to_string()));
                min_line = min_line.min(new_line);
                max_line = max_line.max(new_line);
                new_line += 1;
                raw_hunk.push_str(line);
                raw_hunk.push('\n');
            } else if line.starts_with('-') && !line.starts_with("---") {
                removed.push((new_line, line.to_string()));
                raw_hunk.push_str(line);
                raw_hunk.push('\n');
            } else if !raw_hunk.is_empty() {
                new_line += 1;
                raw_hunk.push_str(line);
                raw_hunk.push('\n');
            }
        }
        if !raw_hunk.is_empty() {
            hunks.push(self.finish_hunk(
                current_file,
                added,
                removed,
                raw_hunk,
                min_line,
                max_line,
            ));
        }
        Ok(hunks)
    }

    fn finish_hunk(
        &self,
        file_path: PathBuf,
        added_lines: Vec<(u32, String)>,
        removed_lines: Vec<(u32, String)>,
        raw_hunk: String,
        min_line: u32,
        max_line: u32,
    ) -> DiffHunk {
        DiffHunk {
            language: infer_language(&file_path),
            file_path,
            changed_lines: if min_line == u32::MAX {
                1..=1
            } else {
                min_line..=max_line
            },
            added_lines,
            removed_lines,
            raw_hunk,
        }
    }
}

fn infer_language(path: &PathBuf) -> Language {
    match path.extension().and_then(|x| x.to_str()).unwrap_or_default() {
        "ts" | "tsx" | "js" | "jsx" => Language::TypeScript,
        "rs" => Language::Rust,
        "py" => Language::Python,
        "go" => Language::Go,
        _ => Language::Unknown,
    }
}
