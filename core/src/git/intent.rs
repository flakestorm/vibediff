use std::path::PathBuf;

use anyhow::Result;
use chrono::{DateTime, Utc};
use git2::Repository;
use serde::{Deserialize, Serialize};

use crate::git::facade::GitFacade;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentRecord {
    pub commit_hash: String,
    pub commit_message: String,
    pub commit_type: CommitType,
    pub scope: Option<String>,
    pub pr_body: Option<String>,
    pub ticket_ref: Option<String>,
    pub author: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommitType {
    Feature,
    Fix,
    Refactor,
    Chore,
    Docs,
    Test,
    Perf,
    Style,
    Unknown,
}

pub struct IntentExtractor {
    git: GitFacade,
}

impl IntentExtractor {
    pub fn new(repo_root: PathBuf) -> Self {
        Self {
            git: GitFacade::new(repo_root),
        }
    }

    pub fn extract(&self, rev: Option<&str>) -> Result<IntentRecord> {
        let repo = Repository::discover(".")?;
        let target = rev.unwrap_or("HEAD");
        let commit = repo.revparse_single(target)?.peel_to_commit()?;
        let msg = self.git.commit_message(rev)?;
        let (commit_type, scope) = parse_commit_type_and_scope(msg.trim());
        Ok(IntentRecord {
            commit_hash: commit.id().to_string(),
            commit_message: msg.trim().to_string(),
            commit_type,
            scope,
            pr_body: std::env::var("VIBEDIFF_PR_BODY").ok(),
            ticket_ref: extract_ticket_ref(msg.trim()),
            author: commit.author().name().unwrap_or("unknown").to_string(),
            timestamp: DateTime::<Utc>::from_timestamp(commit.time().seconds(), 0).unwrap_or(Utc::now()),
        })
    }
}

fn extract_ticket_ref(message: &str) -> Option<String> {
    let re = regex::Regex::new(r"([A-Z]{2,10}-\d+|GH-\d+)").ok()?;
    re.captures(message)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

fn parse_commit_type_and_scope(message: &str) -> (CommitType, Option<String>) {
    let first = message.lines().next().unwrap_or_default();
    let (prefix, maybe_scope) = if let Some(idx) = first.find(':') {
        (&first[..idx], None)
    } else {
        (first, None)
    };
    let (kind, scope) = if let Some(open) = prefix.find('(') {
        let close = prefix.find(')').unwrap_or(prefix.len());
        (&prefix[..open], Some(prefix[open + 1..close].to_string()))
    } else {
        (prefix, maybe_scope)
    };
    let c = match kind {
        "feat" => CommitType::Feature,
        "fix" => CommitType::Fix,
        "refactor" => CommitType::Refactor,
        "chore" => CommitType::Chore,
        "docs" => CommitType::Docs,
        "test" => CommitType::Test,
        "perf" => CommitType::Perf,
        "style" => CommitType::Style,
        _ => CommitType::Unknown,
    };
    (c, scope)
}
