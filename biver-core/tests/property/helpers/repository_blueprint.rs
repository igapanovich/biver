use crate::helpers::extensions::{BiverResultUnwrap, IndexExt, UnwrapExt};
use crate::helpers::file_operation::{FileOperation, FileOperations};
use biver_core::data::BranchName;
use biver_core::repository::{CommitResult, CreateBranchResult, Session};
use biver_core::{Configuration, repository};
use itertools::Itertools;
use proptest::sample::Index;
use std::path::Path;

#[derive(Debug)]
pub struct RepositoryBlueprint {
    pub initial_branch: InitialBranchBlueprint,
    pub other_branches: Vec<BranchBlueprint>,
    pub head: HeadBlueprint,
    pub uncommitted_changes: Option<Vec<FileOperation>>,
}

#[derive(Debug, Clone, Copy)]
pub enum HeadBlueprint {
    Branch(Index),
    Version {
        branch_index: Index,
        version_in_branch_index: Index,
    },
}

#[derive(Debug)]
pub struct InitialBranchBlueprint {
    pub name: Option<BranchName>,
    pub first_version: VersionBlueprint,
    pub following_versions: Vec<VersionBlueprint>,
}

#[derive(Debug)]
pub struct BranchBlueprint {
    pub diverge_at: Index,
    pub name: BranchName,
    pub versions: Vec<VersionBlueprint>,
}

#[derive(Debug)]
pub struct VersionBlueprint {
    pub changes: Vec<FileOperation>,
    pub description: String,
}

impl InitialBranchBlueprint {
    fn build(&self, config: Configuration, versioned_file_path: &Path) -> Session {
        self.first_version.changes.execute(&versioned_file_path);

        let init_result = repository::initialize(
            config.clone(),
            versioned_file_path.to_path_buf(),
            self.name.clone(),
            self.first_version.description.clone(),
        )
        .unwrap();

        assert!(init_result.is_ok());

        let mut session = repository::try_start_session(config, versioned_file_path.to_path_buf())
            .unwrap()
            .unwrap();

        for v in &self.following_versions {
            v.changes.execute(&versioned_file_path);

            match session.commit(v.description.clone()).unwrap() {
                CommitResult::Ok | CommitResult::NoUncommittedChanges => {}
                CommitResult::HeadMustBeOnBranch => panic!("Commit failed"),
            };
        }

        session
    }
}

impl BranchBlueprint {
    fn build(&self, session: &mut Session, versioned_file_path: &Path) {
        let head_versions = session.tree().head_and_ancestors().collect_vec();

        let mut branch_name = self.name.clone();

        let diverge_at_id = self.diverge_at.get(&head_versions).id;

        loop {
            let result = session
                .create_branch(self.name.clone(), diverge_at_id)
                .unwrap();

            match result {
                CreateBranchResult::Ok => break,
                CreateBranchResult::AlreadyExists => {
                    let branch_name_str = branch_name.as_str();
                    let mut new_name = String::with_capacity(branch_name_str.len() + 1);
                    new_name.push_str(branch_name_str);
                    new_name.push('_');
                    branch_name = BranchName::new(new_name).unwrap()
                }
                _ => panic!("CreateBranchResult was {:?}", result),
            }
        }

        session
            .check_out_branch(self.name.clone())
            .unwrap()
            .unwrap();

        for v in &self.versions {
            v.changes.execute(&versioned_file_path);

            match session.commit(v.description.clone()).unwrap() {
                CommitResult::Ok | CommitResult::NoUncommittedChanges => {}
                CommitResult::HeadMustBeOnBranch => panic!("Commit failed"),
            };
        }
    }
}

impl RepositoryBlueprint {
    pub fn build(&self, configuration: Configuration, versioned_file_path: &Path) -> Session {
        let mut session = self
            .initial_branch
            .build(configuration, &versioned_file_path);

        for b in &self.other_branches {
            b.build(&mut session, &versioned_file_path);
        }

        match self.head {
            HeadBlueprint::Branch(index) => {
                let branches = session.tree().branch_names().sorted().collect_vec();
                let branch_name = index.get_copied(&branches);
                session
                    .check_out_branch(branch_name.clone())
                    .unwrap()
                    .unwrap();
            }
            HeadBlueprint::Version {
                branch_index,
                version_in_branch_index,
            } => {
                let branches = session.tree().branch_names().sorted().collect_vec();
                let branch_name = branch_index.get_copied(&branches);
                let branch_tip_id = session.tree().branch_tip_ids()[branch_name];
                let branch_versions = session
                    .tree()
                    .version_and_ancestors(branch_tip_id)
                    .collect_vec();
                let checkout_version = version_in_branch_index.get_copied(&branch_versions);
                session
                    .check_out_version(checkout_version.id)
                    .unwrap()
                    .unwrap();
            }
        }

        if let Some(uncommitted_changes) = &self.uncommitted_changes {
            uncommitted_changes.execute(&versioned_file_path);
        }

        session
    }
}
