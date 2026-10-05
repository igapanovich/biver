#![allow(dead_code)]

use crate::common::extensions::ReadRepositoryOutcomeExt;
use biver_core::configuration::{Configuration, FileTypeRule};
use biver_core::data::Repository;
use biver_core::{RepositoryPaths, operations};
use std::error::Error;
use std::path::PathBuf;
use std::{env, fs};
use uuid::Uuid;

pub struct TestEnv {
    pub root_path: PathBuf,
    pub config: Configuration,
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        if !self.root_path.starts_with("/tmp") {
            panic!("Root path must be in /tmp");
        }

        fs::remove_dir_all(&self.root_path).unwrap();
    }
}

pub fn create() -> Result<TestEnv, Box<dyn Error>> {
    Ok(TestEnv {
        root_path: create_test_dir()?,
        config: test_config(),
    })
}

pub struct TestEnvWithVersionedFile {
    pub root_path: PathBuf,
    pub versioned_file_path: PathBuf,
    pub repo_paths: RepositoryPaths,
    pub config: Configuration,
}

impl Drop for TestEnvWithVersionedFile {
    fn drop(&mut self) {
        if !self.root_path.starts_with("/tmp") {
            panic!("Root path must be in /tmp");
        }

        fs::remove_dir_all(&self.root_path).unwrap();
    }
}

pub fn create_with_versioned_file(
    versioned_file_content: &[u8],
) -> Result<TestEnvWithVersionedFile, Box<dyn Error>> {
    let root_path = create_test_dir()?;
    let versioned_file_path = root_path.join("file");
    fs::write(&versioned_file_path, versioned_file_content)?;

    Ok(TestEnvWithVersionedFile {
        root_path,
        repo_paths: RepositoryPaths::from_versioned_file_path(versioned_file_path.clone()),
        versioned_file_path,
        config: test_config(),
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
        if !self.root_path.starts_with("/tmp") {
            panic!("Root path must be in /tmp");
        }

        fs::remove_dir_all(&self.root_path).unwrap();
    }
}

pub fn create_initialized(
    versioned_file_content: &[u8],
) -> Result<TestEnvInitialized, Box<dyn Error>> {
    let root_path = create_test_dir()?;
    let versioned_file_path = root_path.join("file");
    fs::write(&versioned_file_path, versioned_file_content)?;
    let repo_paths = RepositoryPaths::from_versioned_file_path(versioned_file_path.clone());
    let config = test_config();

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

pub fn create_test_dir() -> Result<PathBuf, Box<dyn Error>> {
    let id = Uuid::new_v4();
    let temp_dir = env::temp_dir();
    let test_dir = temp_dir.join(format!("biver-tests-{}", id));
    fs::create_dir(&test_dir)?;
    Ok(test_dir)
}

pub fn test_config() -> Configuration {
    fn string_vec(collection: &[&str]) -> Vec<String> {
        collection.into_iter().map(|s| s.to_string()).collect()
    }

    Configuration {
        create_patch_command: string_vec(&[
            "xdelta3", "-D", "-e", "-s", "{old}", "{new}", "{patch}",
        ]),
        apply_patch_command: string_vec(&[
            "xdelta3", "-D", "-d", "-s", "{old}", "{patch}", "{new}",
        ]),
        file_type_rules: vec![FileTypeRule {
            extensions: vec!["kra".to_string()],
            preview_command: Some(string_vec(&[
                "magick",
                "{in}",
                "-flatten",
                "-thumbnail",
                "1024x1024>",
                "jpg:{out}",
            ])),
        }],
    }
}
