#![allow(dead_code)]

use crate::common::extensions::ReadRepositoryOutcomeExt;
use crate::common::test_env::{create_test_dir, test_config};
use crate::property_based::repository_action::RepositoryAction;
use biver_core::configuration::Configuration;
use biver_core::data::Repository;
use biver_core::{RepositoryPaths, operations};
use std::fs;
use std::path::PathBuf;

pub struct TestEnv {
    pub test_dir: PathBuf,
    pub config: Configuration,
    pub paths: RepositoryPaths,
    pub repo: Repository,
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        if !self.test_dir.starts_with("/tmp") {
            panic!("Test dir must be in /tmp");
        }

        fs::remove_dir_all(&self.test_dir).unwrap();
    }
}

pub fn create_with_actions(actions: impl IntoIterator<Item = RepositoryAction>) -> TestEnv {
    let test_dir = create_test_dir().unwrap();
    let config = test_config();
    let versioned_file_path = test_dir.join("file");
    let paths = RepositoryPaths::from_versioned_file_path(versioned_file_path);

    let write_versioned_file = {
        let paths = paths.clone();
        move |bytes: Vec<u8>| {
            fs::write(&paths.versioned_file, &bytes).expect("Failed to write to versioned file");
        }
    };

    for action in actions {
        match action {
            RepositoryAction::ModifyVersionedFile(bytes) => {
                write_versioned_file(bytes);
            }
            RepositoryAction::Init(bytes) => {
                write_versioned_file(bytes);
                operations::init(&config, &paths, None, None).expect("Failed to init repository");
            }
            RepositoryAction::Commit(bytes) => {
                if let Some(bytes) = bytes {
                    write_versioned_file(bytes);
                }

                let mut repo = operations::read_repository(&paths).unwrap().unwrap();
                operations::commit(&config, &paths, &mut repo, None).expect("Failed to commit");
            }
        }
    }

    TestEnv {
        test_dir,
        config,
        repo: operations::read_repository(&paths).unwrap().unwrap(),
        paths,
    }
}
