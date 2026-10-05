use crate::configuration::Configuration;
use crate::data::{Head, Repository};
use crate::error::Result;
use crate::operations::has_uncommitted_changes::has_uncommitted_changes;
use crate::operations::{TargetResult, resolve_target};
use crate::repository_io;
use crate::repository_paths::RepositoryPaths;
use derive_more::IsVariant;

#[derive(IsVariant)]
pub enum Outcome {
    Ok,
    HasUncommittedChanges,
    InvalidTarget,
}

pub fn check_out(
    config: &Configuration,
    paths: &RepositoryPaths,
    repo: &mut Repository,
    target: &str,
) -> Result<Outcome> {
    let has_uncommitted_changes = has_uncommitted_changes(paths, repo)?;
    if has_uncommitted_changes {
        return Ok(Outcome::HasUncommittedChanges);
    }

    let new_head = match resolve_target(repo, target) {
        TargetResult::Invalid => return Ok(Outcome::InvalidTarget),
        TargetResult::Branch(branch) => Head::Branch(branch.to_string()),
        TargetResult::Version(version) => Head::Version(version.id),
    };

    repo.head = new_head;
    let new_head_version = repo.head_version();

    repository_io::write_data(paths, repo)?;

    repository_io::extract_version_content(
        config,
        paths,
        repo,
        new_head_version.id,
        &paths.versioned_file,
    )?;

    Ok(Outcome::Ok)
}
