use crate::data::Repository;
use crate::error::Result;
use crate::operations::resolve_target_strict_mut;
use crate::repository_io;
use crate::repository_paths::RepositoryPaths;

pub enum Outcome {
    Ok,
    InvalidTarget,
}

pub fn reword(
    paths: &RepositoryPaths,
    repo: &mut Repository,
    target: &str,
    description: impl Into<String>,
) -> Result<Outcome> {
    let Some(target_version) = resolve_target_strict_mut(repo, target) else {
        return Ok(Outcome::InvalidTarget);
    };

    target_version.description = description.into();

    repository_io::write_data(paths, repo)?;

    Ok(Outcome::Ok)
}
