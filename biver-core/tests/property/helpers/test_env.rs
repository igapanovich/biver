use crate::helpers::extensions::IndexExt;
use crate::helpers::repository_action::{FileOperation, RepositoryAction};
use crate::helpers::snapshot::Snapshot;
use crate::helpers::version_path::ResolveVersionPathExt;
use crate::{test_config, test_dir};
use biver_core::repository::{Session, TryStartSessionResult};
use biver_core::{Configuration, repository};
use itertools::Itertools;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::{env, fs};

pub struct TestEnv {
    init_actions: Vec<RepositoryAction>,
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
        write!(f, "TestEnv{:?}", self.init_actions)
    }
}

impl TestEnv {
    pub fn uninitialized() -> TestEnv {
        let test_dir = test_dir::create().unwrap();
        let config = test_config::create();
        let versioned_file_path = test_dir.join("file");

        TestEnv {
            init_actions: Vec::new(),
            test_dir,
            config,
            versioned_file_path,
        }
    }

    pub fn from_actions(actions: impl IntoIterator<Item = RepositoryAction>) -> TestEnv {
        let mut env = TestEnv::uninitialized();

        let mut session = None;

        for action in actions {
            env.run_action(&mut session, action.clone());
            env.init_actions.push(action);
        }

        env
    }

    pub fn config(&self) -> &Configuration {
        &self.config
    }

    pub fn versioned_file_path(&self) -> &Path {
        &self.versioned_file_path
    }

    pub fn start_session(&self) -> Session {
        let result =
            repository::try_start_session(self.config.clone(), self.versioned_file_path.clone())
                .unwrap();
        match result {
            TryStartSessionResult::Ok(session) => session,
            TryStartSessionResult::Uninitialized => panic!("Repository was not initialized"),
        }
    }

    pub fn snapshot(&self, session: &Session) -> Snapshot {
        Snapshot::new(self.versioned_file_path(), session)
    }

    fn run_action(&self, session: &mut Option<Session>, action: RepositoryAction) {
        match action {
            RepositoryAction::ModifyVersionedFile(file_op) => {
                self.run_versioned_file_operation(file_op);
            }
            RepositoryAction::Init => {
                let init_ok = repository::initialize(
                    self.config.clone(),
                    self.versioned_file_path.clone(),
                    None,
                    None,
                )
                .unwrap();
                *session = Some(init_ok.session);
            }
            RepositoryAction::Commit => {
                let session = session.as_mut().unwrap();
                session.commit(None).unwrap();
            }
            RepositoryAction::Discard => {
                let session = session.as_ref().unwrap();
                session.discard().unwrap();
            }
            RepositoryAction::CheckOutBranch(branch_index) => {
                let session = session.as_mut().unwrap();
                let branch_name =
                    branch_index.get_copied(&session.tree().branch_names().collect_vec());
                session.check_out_branch(branch_name.clone()).unwrap();
            }
            RepositoryAction::CheckOutVersion(version_path) => {
                let session = session.as_mut().unwrap();
                let version_id = session.tree().resolve_version_path(&version_path).id;
                session.check_out_version(version_id).unwrap();
            }
        }
    }

    fn run_versioned_file_operation(&self, file_operation: FileOperation) {
        match file_operation {
            FileOperation::Overwrite(bytes) => {
                fs::write(&self.versioned_file_path, &bytes).unwrap();
            }
            FileOperation::Splice {
                range_start,
                range_length,
                bytes,
            } => {
                let mut content = fs::read(&self.versioned_file_path).unwrap();
                if content.is_empty() {
                    fs::write(&self.versioned_file_path, &bytes).unwrap();
                } else {
                    let range_start = range_start.index(content.len());
                    let range_length = range_length.index(content.len() - range_start);
                    let range_end = range_start + range_length;
                    content.splice(range_start..range_end, bytes);
                    fs::write(&self.versioned_file_path, &content).unwrap();
                }
            }
        }
    }
}
