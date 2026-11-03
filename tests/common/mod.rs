use std::path::Path;

use chrono::NaiveDate;
use git2::{Oid, Repository, Signature, Time};

pub fn day_to_timestamp(text: &str, hour: u32, offset_minutes: i32) -> i64 {
    let date = NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap_or(NaiveDate::MIN);
    let naive = date.and_hms_opt(hour, 0, 0).unwrap_or_default();
    naive.and_utc().timestamp() - i64::from(offset_minutes) * 60
}

pub fn init_repository(path: &Path) -> Result<Repository, git2::Error> {
    Repository::init(path)
}

pub fn commit_file(
    repository: &Repository,
    file_name: &str,
    content: &str,
    message: &str,
    author: (&str, &str),
    time: (i64, i32),
) -> Result<Oid, git2::Error> {
    let workdir = repository.workdir().unwrap_or(Path::new("."));
    let target = workdir.join(file_name);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| git2::Error::from_str(&error.to_string()))?;
    }
    std::fs::write(&target, content).map_err(|error| git2::Error::from_str(&error.to_string()))?;
    let mut index = repository.index()?;
    index.add_path(Path::new(file_name))?;
    index.write()?;
    let tree_id = index.write_tree()?;
    let tree = repository.find_tree(tree_id)?;
    let signature = signature(author, time)?;
    let parents = match repository.head() {
        Ok(head) => vec![head.peel_to_commit()?],
        Err(_) => Vec::new(),
    };
    let parent_refs: Vec<&git2::Commit<'_>> = parents.iter().collect();
    repository.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &parent_refs,
    )
}

pub fn signature<'a>(author: (&str, &str), time: (i64, i32)) -> Result<Signature<'a>, git2::Error> {
    Signature::new(author.0, author.1, &Time::new(time.0, time.1))
}
