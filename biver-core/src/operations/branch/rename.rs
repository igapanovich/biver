use crate::data::Repository;
use crate::error::Result;
use crate::repository_io;
use crate::repository_paths::RepositoryPaths;

pub enum Outcome {
    Ok,
    AnotherBranchExistsWithSameName,
    BranchDoesNotExist,
}

pub fn rename(
    paths: &RepositoryPaths,
    repo: &mut Repository,
    old_name: &str,
    new_name: impl Into<String>,
) -> Result<Outcome> {
    let new_name = new_name.into();

    if old_name == new_name {
        return Ok(Outcome::Ok);
    }

    if repo.branches.contains_key(&new_name) {
        return Ok(Outcome::AnotherBranchExistsWithSameName);
    }

    let Some(branch_version_id) = repo.branches.remove(old_name) else {
        return Ok(Outcome::BranchDoesNotExist);
    };

    repo.branches.insert(new_name, branch_version_id);

    repository_io::write_data(paths, repo)?;

    Ok(Outcome::Ok)
}
