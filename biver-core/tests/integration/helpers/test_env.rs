use crate::helpers::extensions::ReadRepositoryOutcomeExt;
use crate::{test_config, test_dir};
use biver_core::configuration::Configuration;
use biver_core::data::Repository;
use biver_core::{RepositoryPaths, operations};
use std::error::Error;
use std::path::PathBuf;
use std::{env, fs};

pub struct TestEnvWithVersionedFile {
    pub root_path: PathBuf,
    pub versioned_file_path: PathBuf,
    pub repo_paths: RepositoryPaths,
    pub config: Configuration,
}

impl Drop for TestEnvWithVersionedFile {
    fn drop(&mut self) {
        if !self.root_path.starts_with(env::temp_dir()) {
            panic!("Test directory must be in the system temp directory");
        }

        fs::remove_dir_all(&self.root_path).unwrap();
    }
}

pub fn create_with_versioned_file(
    versioned_file_content: &[u8],
) -> Result<TestEnvWithVersionedFile, Box<dyn Error>> {
    let root_path = test_dir::create()?;
    let versioned_file_path = root_path.join("file");
    fs::write(&versioned_file_path, versioned_file_content)?;

    Ok(TestEnvWithVersionedFile {
        root_path,
        repo_paths: RepositoryPaths::from_versioned_file_path(versioned_file_path.clone()),
        versioned_file_path,
        config: test_config::create(),
    })
}

pub struct TestEnvInitialized {
    pub root_path: PathBuf,
    pub versioned_file_path: PathBuf,
    pub repo_paths: RepositoryPaths,
    pub repo: Repository,
    pub config: Configuration,
}

impl Drop for TestEnvInitialized {
    fn drop(&mut self) {
        if !self.root_path.starts_with(env::temp_dir()) {
            panic!("Test directory must be in the system temp directory");
        }

        fs::remove_dir_all(&self.root_path).unwrap();
    }
}

pub fn create_initialized(
    versioned_file_content: &[u8],
) -> Result<TestEnvInitialized, Box<dyn Error>> {
    let root_path = test_dir::create()?;
    let versioned_file_path = root_path.join("file");
    fs::write(&versioned_file_path, versioned_file_content)?;
    let repo_paths = RepositoryPaths::from_versioned_file_path(versioned_file_path.clone());
    let config = test_config::create();

    operations::init(&config, &repo_paths, None, None)?;

    let repo = operations::read_repository(&repo_paths)?.unwrap();

    Ok(TestEnvInitialized {
        root_path,
        versioned_file_path,
        repo_paths,
        repo,
        config,
    })
}
