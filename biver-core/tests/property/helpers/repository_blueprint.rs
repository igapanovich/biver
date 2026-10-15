use crate::helpers::extensions::{BiverResultUnwrap, IndexExt, UnwrapExt};
use crate::helpers::file_operation::{FileOperation, FileOperations};
use crate::helpers::version_path::{ResolveVersionPathExt, VersionPath};
use biver_core::data::BranchName;
use biver_core::repository::{CleanupResult, CommitResult, CreateBranchResult, Session};
use biver_core::{Config, repository};
use itertools::Itertools;
use proptest::sample::Index;
use std::fmt::Debug;
use std::iter;
use std::path::Path;

pub struct RepositoryBlueprint {
    pub initial_branch: InitialBranchBlueprint,
    pub other_branches: Vec<BranchBlueprint>,
    pub head: HeadBlueprint,
    pub uncommitted_changes: Option<Vec<FileOperation>>,
}

impl RepositoryBlueprint {
    pub fn build(&self, configuration: Config, versioned_file_path: &Path) -> Session {
        let mut session = self
            .initial_branch
            .build(configuration, &versioned_file_path);

        for b in &self.other_branches {
            b.build(&mut session, &versioned_file_path);
        }

        match &self.head {
            HeadBlueprint::Branch(index) => {
                let branches = session.tree().branch_names().sorted().collect_vec();
                let branch_name = index.get_copied(&branches);
                session
                    .check_out_branch(branch_name.clone())
                    .unwrap()
                    .unwrap();
            }
            HeadBlueprint::Version(path) => {
                let version_id = session.tree().resolve_version_path(path).id;
                session.check_out_version(version_id).unwrap().unwrap();
            }
        }

        if let Some(uncommitted_changes) = &self.uncommitted_changes {
            uncommitted_changes.execute(&versioned_file_path);
        }

        session
    }
}

impl Debug for RepositoryBlueprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "(")?;
        writeln!(f, "  Uncommitted: {:?}", self.uncommitted_changes)?;
        writeln!(f, "  Head: {:?}", self.head)?;
        writeln!(f, "  Branches:")?;
        writeln!(f, "    {:?}", self.initial_branch)?;
        for b in &self.other_branches {
            writeln!(f, "    {:?}", b)?;
        }
        write!(f, ")")?;
        Ok(())
    }
}

#[derive(Clone)]
pub enum HeadBlueprint {
    Branch(Index),
    Version(VersionPath),
}

impl Debug for HeadBlueprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeadBlueprint::Branch(index) => write!(f, "Branch({})", index.debug_fraction()),
            HeadBlueprint::Version(path) => write!(f, "Version({:?})", path),
        }
    }
}

pub struct InitialBranchBlueprint {
    pub name: Option<BranchName>,
    pub first_version: VersionBlueprint,
    pub following_versions: Vec<VersionBlueprint>,
}

impl InitialBranchBlueprint {
    fn build(&self, config: Config, versioned_file_path: &Path) -> Session {
        self.first_version.changes.execute(&versioned_file_path);

        let init_result = repository::initialize(
            config,
            versioned_file_path,
            self.name.clone(),
            self.first_version.description.clone(),
        )
        .unwrap();

        assert!(init_result.is_ok());

        let mut session = repository::try_start_session(versioned_file_path)
            .unwrap()
            .unwrap();

        for v in &self.following_versions {
            v.changes.execute(&versioned_file_path);

            match session.commit(v.description.clone()).unwrap() {
                CommitResult::Ok(CleanupResult::Ok) | CommitResult::NoUncommittedChanges => {}
                _ => panic!("Commit failed"),
            };
        }

        session
    }
}

impl Debug for InitialBranchBlueprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self
            .name
            .as_ref()
            .map(|n| n.to_string())
            .unwrap_or(String::new());

        let versions = iter::once(&self.first_version)
            .chain(self.following_versions.iter())
            .collect_vec();

        write!(f, "Initial Name({}) {:?}", name, versions)
    }
}

#[derive(Clone)]
pub struct BranchBlueprint {
    pub diverge_at: Index,
    pub name: BranchName,
    pub versions: Vec<VersionBlueprint>,
}

impl BranchBlueprint {
    fn build(&self, session: &mut Session, versioned_file_path: &Path) {
        let head_versions = session.tree().head_and_ancestors().collect_vec();

        let mut branch_name = self.name.clone();

        let diverge_at_id = self.diverge_at.get(&head_versions).id;

        loop {
            let result = session
                .create_branch(branch_name.clone(), diverge_at_id)
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
                CommitResult::Ok(CleanupResult::Ok) | CommitResult::NoUncommittedChanges => {}
                _ => panic!("Commit failed"),
            };
        }
    }
}

impl Debug for BranchBlueprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Diverge({}) Name({}) {:?}",
            self.diverge_at.debug_fraction(),
            self.name.as_str(),
            self.versions
        )
    }
}

#[derive(Clone)]
pub struct VersionBlueprint {
    pub changes: Vec<FileOperation>,
    pub description: String,
}

impl Debug for VersionBlueprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}:{:?})", &self.description, &self.changes)
    }
}
