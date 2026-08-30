use crate::configuration::Configuration;
use crate::data::Repository;
use crate::error::Result;
use crate::operations::has_uncommitted_changes::has_uncommitted_changes;
use crate::operations::{TargetResult, resolve_target};
use crate::repository_io;
use crate::repository_paths::RepositoryPaths;
use std::path::Path;

pub enum Outcome {
    Ok,
    BlockedByUncommittedChanges,
    InvalidTarget,
}

pub fn restore(
    config: &Configuration,
    paths: &RepositoryPaths,
    repo: &Repository,
    target: &str,
    output: Option<&Path>,
) -> Result<Outcome> {
    let has_uncommitted_changes = has_uncommitted_changes(paths, repo)?;

    if has_uncommitted_changes {
        return Ok(Outcome::BlockedByUncommittedChanges);
    }

    let target_version = match resolve_target(repo, target) {
        TargetResult::Invalid => return Ok(Outcome::InvalidTarget),
        TargetResult::Branch(branch) => repo
            .version(repo.branches[branch])
            .expect("Branch resolved from target must exist"),
        TargetResult::Version(version) => version,
    };

    let output = output.unwrap_or_else(|| &paths.versioned_file);

    repository_io::extract_version_content(config, paths, repo, target_version.id, output)?;

    Ok(Outcome::Ok)
}
