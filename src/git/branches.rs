use git2::{BranchType, Repository};

use crate::errors::Result;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BranchSummary {
    pub local: usize,
    pub remote: usize,
    pub local_names: Vec<String>,
}

pub fn summarize(repository: &Repository) -> Result<BranchSummary> {
    let mut summary = BranchSummary::default();
    for entry in repository.branches(None)? {
        let (branch, kind) = entry?;
        match kind {
            BranchType::Local => {
                summary.local += 1;
                if let Some(name) = branch.name()? {
                    summary.local_names.push(name.to_owned());
                }
            }
            BranchType::Remote => {
                if let Some(name) = branch.name()? {
                    if !name.ends_with("/HEAD") {
                        summary.remote += 1;
                    }
                }
            }
        }
    }
    summary.local_names.sort();
    Ok(summary)
}
