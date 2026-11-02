use crate::common::repository_blueprint::RepositoryBlueprint;
use crate::common::test_env_snapshot::TestEnvSnapshot;
use crate::common::time_control_environment::TimeControlEnvironment;
use crate::common::{test_config, test_dir};
use biver_core::data::Snapshot;
use biver_core::environment::Environment;
use biver_core::repository::Session;
use biver_core::{Config, repository};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{env, fs};

pub struct TestEnv<Env: Environment + Clone + 'static> {
    env: Env,
    blueprint: Option<RepositoryBlueprint>,
    test_dir: PathBuf,
    config: Config,
    versioned_file_path: PathBuf,
    clone: bool,
}

impl<Env: Environment + Clone> Drop for TestEnv<Env> {
    fn drop(&mut self) {
        if self.clone {
            return;
        }

        if !self.test_dir.starts_with(env::temp_dir()) {
            panic!("Test directory must be in the system temp directory");
        }

        fs::remove_dir_all(&self.test_dir).unwrap();
    }
}

impl<Env: Environment + Clone> Debug for TestEnv<Env> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TestEnv({:?})", self.blueprint)
    }
}

impl<Env: Environment + Clone> TestEnv<Env> {
    pub fn uninitialized(env: Env) -> TestEnv<Env> {
        let test_dir = test_dir::create().unwrap();
        let config = test_config::create();
        let versioned_file_path = test_dir.join("file");

        TestEnv {
            env,
            blueprint: None,
            test_dir,
            config,
            versioned_file_path,
            clone: false,
        }
    }

    pub fn from_blueprint(env: Env, blueprint: RepositoryBlueprint) -> TestEnv<Env> {
        let mut test_env = TestEnv::uninitialized(env);

        blueprint.build(&test_env);

        test_env.blueprint = Some(blueprint);

        test_env
    }

    pub fn env(&self) -> &Env {
        &self.env
    }

    pub fn env_mut(&mut self) -> &mut Env {
        &mut self.env
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn versioned_file_path(&self) -> &Path {
        &self.versioned_file_path
    }

    pub fn test_dir(&self) -> &Path {
        &self.test_dir
    }

    pub fn start_session(&self) -> Session {
        self.start_session_with_snapshot(Snapshot::Main).unwrap()
    }

    pub fn try_start_session(&self) -> Result<Session, biver_core::Error> {
        self.start_session_with_snapshot(Snapshot::Main)
    }

    pub fn start_session_with_snapshot(
        &self,
        snapshot: Snapshot,
    ) -> Result<Session, biver_core::Error> {
        repository::start_session(self.env.clone(), &self.versioned_file_path, snapshot)
    }

    pub fn snapshot(&self, session: &Session) -> TestEnvSnapshot {
        TestEnvSnapshot::new(self.versioned_file_path(), session)
    }

    pub fn clone_with_env<NewEnv: Environment + Clone>(
        &self,
        create_env: fn(&Env) -> NewEnv,
    ) -> TestEnv<NewEnv> {
        TestEnv {
            env: create_env(self.env()),
            blueprint: None,
            test_dir: self.test_dir.clone(),
            config: self.config.clone(),
            versioned_file_path: self.versioned_file_path.clone(),
            clone: true,
        }
    }
}

impl TestEnv<TimeControlEnvironment> {
    pub fn advance_time(&mut self, duration: Duration) {
        self.env.advance_time(duration)
    }
}
