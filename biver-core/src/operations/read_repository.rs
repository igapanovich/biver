use crate::data::Repository;
use crate::error::Result;
use crate::repository_io::RepositoryDataResult;
use crate::{RepositoryPaths, repository_io};

pub enum Outcome {
    Initialized(Repository),
    NotInitialized,
}

pub fn read_repository(repository_paths: &RepositoryPaths) -> Result<Outcome> {
    match repository_io::read_data(repository_paths)? {
        RepositoryDataResult::Initialized(data) => Ok(Outcome::Initialized(data)),
        RepositoryDataResult::NotInitialized => Ok(Outcome::NotInitialized),
    }
}
