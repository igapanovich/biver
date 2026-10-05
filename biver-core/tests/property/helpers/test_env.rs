use crate::helpers::repository_action::{FileOperation, PositionInFile, RepositoryAction};
use crate::{test_config, test_dir};
use biver_core::configuration::Configuration;
use biver_core::data::Repository;
use biver_core::{RepositoryPaths, operations};
use std::ops::Range;
use std::path::PathBuf;
use std::{env, fs};

#[derive(Debug)]
pub struct TestEnv {
    test_dir: PathBuf,
    config: Configuration,
    paths: RepositoryPaths,
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        if !self.test_dir.starts_with(env::temp_dir()) {
            panic!("Test directory must be in the system temp directory");
        }

        fs::remove_dir_all(&self.test_dir).unwrap();
    }
}

impl TestEnv {
    pub fn new() -> TestEnv {
        let test_dir = test_dir::create().unwrap();
        let config = test_config::create();
        let versioned_file_path = test_dir.join("file");
        let paths = RepositoryPaths::from_versioned_file_path(versioned_file_path);

        TestEnv {
            test_dir,
            config,
            paths,
        }
    }

    pub fn from_actions(actions: impl IntoIterator<Item = RepositoryAction>) -> TestEnv {
        let env = TestEnv::new();

        env.run_actions(actions);

        env
    }

    pub fn config(&self) -> &Configuration {
        &self.config
    }

    pub fn paths(&self) -> &RepositoryPaths {
        &self.paths
    }

    pub fn read_repository(&self) -> Repository {
        let operations::read_repository::Outcome::Initialized(repo) =
            operations::read_repository(&self.paths).unwrap()
        else {
            panic!("Repository was not initialized");
        };

        repo
    }

    pub fn run_actions(&self, actions: impl IntoIterator<Item = RepositoryAction>) {
        for action in actions {
            self.run_action(action);
        }
    }

    pub fn run_action(&self, action: RepositoryAction) {
        match action {
            RepositoryAction::ModifyVersionedFile(file_op) => {
                self.run_versioned_file_operation(file_op);
            }
            RepositoryAction::Init => {
                operations::init(&self.config, &self.paths, None, None).unwrap();
            }
            RepositoryAction::Commit => {
                operations::commit(&self.config, &self.paths, &mut self.read_repository(), None)
                    .unwrap();
            }
        }
    }

    pub fn run_versioned_file_operations(
        &self,
        file_operations: impl IntoIterator<Item = FileOperation>,
    ) {
        for file_operation in file_operations {
            self.run_versioned_file_operation(file_operation);
        }
    }

    pub fn run_versioned_file_operation(&self, file_operation: FileOperation) {
        match file_operation {
            FileOperation::Overwrite(bytes) => {
                fs::write(&self.paths.versioned_file, &bytes).unwrap();
            }
            FileOperation::Insert { position, bytes } => {
                let mut content = fs::read(&self.paths.versioned_file).unwrap();
                let range = fit_range(position, 0, content.len());
                content.splice(range, bytes);
                fs::write(&self.paths.versioned_file, &content).unwrap();
            }
            FileOperation::RemoveRange { start, length } => {
                let mut content = fs::read(&self.paths.versioned_file).unwrap();
                let range = fit_range(start, length, content.len());
                content.splice(range, []);
                fs::write(&self.paths.versioned_file, &content).unwrap();
            }
        }
    }
}

fn fit_range(
    range_start: PositionInFile,
    range_length: usize,
    total_length: usize,
) -> Range<usize> {
    if range_length >= total_length {
        return 0..total_length;
    }

    let max_start = total_length - range_length;

    let start = match range_start {
        PositionInFile::FromStart(pos) => pos % max_start,
        PositionInFile::FromEnd(pos) => max_start - (pos % max_start),
    };

    let end = start + range_length;
    start..end
}
