use crate::box_environment::BoxEnvironment;
use crate::helpers::samples::SampleSeq;
use crate::{test_config, test_dir};
use biver_core::data::Snapshot;
use biver_core::environment::Environment;
use biver_core::repository;
use biver_core::repository::Session;
use std::path::{Path, PathBuf};
use std::{env, fs};

pub struct TestEnv {
    env: BoxEnvironment,
    sample_seq: SampleSeq,
    root_path: PathBuf,
    versioned_file_path: PathBuf,
}

impl TestEnv {
    pub fn new(env: impl Environment + Clone + 'static) -> TestEnv {
        let root_path = test_dir::create().unwrap();
        let versioned_file_path = root_path.join("file");
        let mut sample_seq = SampleSeq::new();
        fs::write(&versioned_file_path, sample_seq.next()).unwrap();
        let config = test_config::create();

        repository::initialize(&env, config, &versioned_file_path, None, "".to_string()).unwrap();

        TestEnv {
            env: BoxEnvironment::new(env),
            sample_seq,
            root_path,
            versioned_file_path,
        }
    }

    pub fn versioned_file_path(&self) -> &Path {
        &self.versioned_file_path
    }

    pub fn next_sample(&mut self) -> &'static [u8] {
        self.sample_seq.next()
    }

    pub fn start_session(&self) -> biver_core::Result<Session> {
        self.start_session_with(Snapshot::Main)
    }

    pub fn start_session_with(&self, snapshot: Snapshot) -> biver_core::Result<Session> {
        repository::start_session(self.env.clone(), self.versioned_file_path(), snapshot)
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        if !self.root_path.starts_with(env::temp_dir()) {
            panic!("Test directory must be in the system temp directory");
        }

        fs::remove_dir_all(&self.root_path).unwrap();
    }
}
