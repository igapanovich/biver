use crate::data::{Head, Repository};
use crate::error::Result;
use crate::operations::valid_branch_name;
use crate::repository_io;
use crate::repository_paths::RepositoryPaths;

pub enum Outcome {
    Ok,
    BranchAlreadyExists,
    InvalidBranchName,
}

pub fn create(
    paths: &RepositoryPaths,
    repo: &mut Repository,
    name: impl Into<String>,
    checkout: bool,
) -> Result<Outcome> {
    let name = name.into();

    if repo.branches.contains_key(&name) {
        return Ok(Outcome::BranchAlreadyExists);
    }

    if !valid_branch_name(&name) {
        return Ok(Outcome::InvalidBranchName);
    }

    let head_version_id = repo.head_version().id;

    repo.branches.insert(name.clone(), head_version_id);

    if checkout {
        repo.head = Head::Branch(name);
    }

    repository_io::write_data(paths, repo)?;

    Ok(Outcome::Ok)
}
