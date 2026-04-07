use std::path::PathBuf;

use anyhow::Result;
use git2::{DiffFormat, Repository};

pub struct GitFacade {
    repo_root: PathBuf,
}

impl GitFacade {
    pub fn new(repo_root: PathBuf) -> Self {
        Self { repo_root }
    }

    pub fn diff_staged(&self) -> Result<String> {
        let repo = Repository::discover(&self.repo_root)?;
        let head = repo.head()?.peel_to_tree()?;
        let index = repo.index()?;
        let diff = repo.diff_tree_to_index(Some(&head), Some(&index), None)?;
        let mut out = String::new();
        diff.print(DiffFormat::Patch, |_d, _h, line| {
            out.push_str(std::str::from_utf8(line.content()).unwrap_or_default());
            true
        })?;
        Ok(out)
    }

    pub fn diff_rev(&self, rev: &str) -> Result<String> {
        let repo = Repository::discover(&self.repo_root)?;
        let obj = repo.revparse_single(rev)?;
        let commit = obj.peel_to_commit()?;
        let tree = commit.tree()?;
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };
        let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;
        let mut out = String::new();
        diff.print(DiffFormat::Patch, |_d, _h, line| {
            out.push_str(std::str::from_utf8(line.content()).unwrap_or_default());
            true
        })?;
        Ok(out)
    }

    pub fn commit_message(&self, rev: Option<&str>) -> Result<String> {
        let repo = Repository::discover(&self.repo_root)?;
        let target = rev.unwrap_or("HEAD");
        let commit = repo.revparse_single(target)?.peel_to_commit()?;
        Ok(commit.message().unwrap_or_default().to_string())
    }
}
