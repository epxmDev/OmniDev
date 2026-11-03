use std::path::{Path, PathBuf};

use git2::{ErrorCode, Repository};

use crate::errors::Result;

pub fn open(path: &Path) -> Result<Repository> {
    Ok(Repository::open(path)?)
}

pub fn discover(path: &Path) -> Result<Repository> {
    Ok(Repository::discover(path)?)
}

pub fn root_path(repository: &Repository) -> PathBuf {
    repository
        .workdir()
        .unwrap_or_else(|| repository.path())
        .to_path_buf()
}

pub fn display_name(root: &Path) -> String {
    root.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.to_string_lossy().into_owned())
}

pub fn head_hash(repository: &Repository) -> Result<Option<String>> {
    match repository.head() {
        Ok(head) => Ok(head.target().map(|oid| oid.to_string())),
        Err(error) if error.code() == ErrorCode::UnbornBranch => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub fn is_empty(repository: &Repository) -> Result<bool> {
    Ok(head_hash(repository)?.is_none())
}
