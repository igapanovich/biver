use crate::helpers::repository_blueprint::RepositoryBlueprint;
use crate::helpers::snapshot::Snapshot;
use crate::{test_config, test_dir};
use biver_core::repository::{Session, TryStartSessionResult};
use biver_core::{Configuration, repository};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::{env, fs};

pub struct TestEnv {
    blueprint: Option<RepositoryBlueprint>,
    test_dir: PathBuf,
    config: Configuration,
    versioned_file_path: PathBuf,
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        if !self.test_dir.starts_with(env::temp_dir()) {
            panic!("Test directory must be in the system temp directory");
        }

        fs::remove_dir_all(&self.test_dir).unwrap();
    }
}

impl Debug for TestEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TestEnv({:?})", self.blueprint)
    }
}

impl TestEnv {
    pub fn uninitialized() -> TestEnv {
        let test_dir = test_dir::create().unwrap();
        let config = test_config::create();
        let versioned_file_path = test_dir.join("file");

        TestEnv {
            blueprint: None,
            test_dir,
            config,
            versioned_file_path,
        }
    }

    pub fn from_blueprint(blueprint: RepositoryBlueprint) -> TestEnv {
        let mut env = TestEnv::uninitialized();

        blueprint.build(env.config.clone(), &env.versioned_file_path);

        env.blueprint = Some(blueprint);

        env
    }

    pub fn config(&self) -> &Configuration {
        &self.config
    }

    pub fn versioned_file_path(&self) -> &Path {
        &self.versioned_file_path
    }

    pub fn start_session(&self) -> Session {
        let result = repository::try_start_session(&self.versioned_file_path).unwrap();
        match result {
            TryStartSessionResult::Ok(session) => session,
            TryStartSessionResult::NotInitialized => panic!("Repository was not initialized"),
        }
    }

    pub fn snapshot(&self, session: &Session) -> Snapshot {
        Snapshot::new(self.versioned_file_path(), session)
    }
}
