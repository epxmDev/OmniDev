use std::collections::HashSet;

use git2::{Repository, Sort};

use crate::core::model::CommitInfo;
use crate::errors::Result;
use crate::git::diff;

const SUMMARY_LIMIT: usize = 200;

pub fn walk_new_commits<F>(
    repository: &Repository,
    known: &HashSet<String>,
    mut sink: F,
) -> Result<usize>
where
    F: FnMut(CommitInfo) -> Result<()>,
{
    let mut walk = repository.revwalk()?;
    walk.set_sorting(Sort::TIME)?;
    push_reachable_tips(repository, &mut walk)?;
    let mut added = 0;
    for oid in walk {
        let oid = oid?;
        if known.contains(&oid.to_string()) {
            continue;
        }
        let commit = repository.find_commit(oid)?;
        let is_merge = commit.parent_count() > 1;
        let totals = if is_merge {
            (0, 0, 0)
        } else {
            diff::totals(repository, &commit)?
        };
        let message = String::from_utf8_lossy(commit.message_bytes());
        let summary = summarize_message(&message);
        let info = diff::build_info(&commit, totals, summary, is_merge);
        sink(info)?;
        added += 1;
    }
    Ok(added)
}

fn push_reachable_tips(repository: &Repository, walk: &mut git2::Revwalk<'_>) -> Result<()> {
    for reference in repository.references()? {
        let reference = reference?;
        let Some(name) = reference.name() else {
            tracing::warn!("skipping a reference whose name is not valid UTF-8");
            continue;
        };
        if !(name.starts_with("refs/heads/") || name.starts_with("refs/remotes/")) {
            continue;
        }
        let commit = reference.peel_to_commit()?;
        walk.push(commit.id())?;
    }
    match repository.head() {
        Ok(head) => {
            let commit = head.peel_to_commit()?;
            walk.push(commit.id())?;
        }
        Err(error) if error.code() == git2::ErrorCode::UnbornBranch => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

fn summarize_message(message: &str) -> String {
    message
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .chars()
        .take(SUMMARY_LIMIT)
        .collect()
}
