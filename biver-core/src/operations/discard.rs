use crate::configuration::Configuration;
use crate::data::Repository;
use crate::error::Result;
use crate::repository_io;
use crate::repository_paths::RepositoryPaths;

pub fn discard(config: &Configuration, paths: &RepositoryPaths, repo: &Repository) -> Result<()> {
    repository_io::extract_version_content(
        config,
        paths,
        repo,
        repo.head_version().id,
        &paths.versioned_file,
    )?;
    Ok(())
}
