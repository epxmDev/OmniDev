use git2::{Commit, Delta, Diff, Oid, Repository};

use crate::core::model::CommitInfo;
use crate::errors::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedFile {
    pub status: char,
    pub path: String,
}

fn commit_diff<'repository>(
    repository: &'repository Repository,
    commit: &Commit<'repository>,
) -> Result<Diff<'repository>> {
    let tree = commit.tree()?;
    let parent_tree = if commit.parent_count() > 0 {
        Some(commit.parent(0)?.tree()?)
    } else {
        None
    };
    Ok(repository.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?)
}

pub fn totals(repository: &Repository, commit: &Commit<'_>) -> Result<(u64, u64, u64)> {
    let diff = commit_diff(repository, commit)?;
    let stats = diff.stats()?;
    Ok((
        stats.insertions() as u64,
        stats.deletions() as u64,
        stats.files_changed() as u64,
    ))
}

pub fn changed_files(repository: &Repository, hash: &str) -> Result<Vec<ChangedFile>> {
    let oid = Oid::from_str(hash)?;
    let commit = repository.find_commit(oid)?;
    let diff = commit_diff(repository, &commit)?;
    let files = diff
        .deltas()
        .map(|delta| {
            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default();
            ChangedFile {
                status: status_letter(delta.status()),
                path,
            }
        })
        .collect();
    Ok(files)
}

fn status_letter(status: Delta) -> char {
    match status {
        Delta::Added => 'A',
        Delta::Deleted => 'D',
        Delta::Modified => 'M',
        Delta::Renamed => 'R',
        Delta::Copied => 'C',
        Delta::Typechange => 'T',
        _ => '?',
    }
}

pub fn build_info(
    commit: &Commit<'_>,
    totals: (u64, u64, u64),
    summary: String,
    is_merge: bool,
) -> CommitInfo {
    let author = commit.author();
    let time = commit.time();
    CommitInfo {
        hash: commit.id().to_string(),
        author_name: text_or(author.name_bytes(), "Unknown author"),
        author_email: text_or(author.email_bytes(), "unknown")
            .trim()
            .to_ascii_lowercase(),
        committed_at: time.seconds(),
        utc_offset_minutes: time.offset_minutes(),
        summary,
        is_merge,
        insertions: totals.0,
        deletions: totals.1,
        files_changed: totals.2,
    }
}

fn text_or(bytes: &[u8], fallback: &str) -> String {
    let text = String::from_utf8_lossy(bytes).trim().to_owned();
    if text.is_empty() {
        fallback.to_owned()
    } else {
        text
    }
}
