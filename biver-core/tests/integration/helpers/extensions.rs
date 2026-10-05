use biver_core::data::Repository;
use biver_core::operations;

pub trait ReadRepositoryOutcomeExt {
    fn unwrap(self) -> Repository;
}

impl ReadRepositoryOutcomeExt for operations::read_repository::Outcome {
    fn unwrap(self) -> Repository {
        match self {
            operations::read_repository::Outcome::Initialized(repo) => repo,
            operations::read_repository::Outcome::NotInitialized => panic!("not initialized"),
        }
    }
}
