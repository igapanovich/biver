use crate::common::extensions::IndexExt;
use crate::common::file_operation::FileOperation;
use biver_core::data::{BranchName, VersionId};
use biver_core::repository::{Paths, Session};
use itertools::Itertools;
use proptest::sample::Index;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum RepositoryOperation {
    ChangeVersionedFile(FileOperation),
    Commit(String),
    Amend(Option<String>),
    Discard,
    CheckOutBranch(TargetBranch),
    CheckOutVersion(TargetVersion),
    UpdateDescription(TargetVersion, String),
    RestoreVersion(TargetVersion, TargetFile),
    SoftReset(TargetVersion),
    Rebase(TargetVersion, TargetVersion),
    CreateBranch(BranchName, TargetVersion),
    RenameBranch(TargetBranch, BranchName),
    RemoveBranch(TargetBranch),
    TidyUp,
}

impl RepositoryOperation {
    pub fn execute(self, paths: &Paths, session: &mut Session) -> biver_core::Result<()> {
        match self {
            RepositoryOperation::ChangeVersionedFile(file_op) => {
                file_op.execute(paths.versioned_file());
            }
            RepositoryOperation::Commit(description) => {
                session.commit(description)?;
            }
            RepositoryOperation::Amend(description) => {
                session.amend(description)?;
            }
            RepositoryOperation::Discard => {
                session.discard()?;
            }
            RepositoryOperation::CheckOutBranch(target_branch) => {
                let target_branch = target_branch.resolve(session);
                session.check_out_branch(target_branch)?;
            }
            RepositoryOperation::CheckOutVersion(target_version) => {
                let target_version = target_version.resolve(session);
                session.check_out_version(target_version)?;
            }
            RepositoryOperation::UpdateDescription(target_version, new_description) => {
                let target_version = target_version.resolve(session);
                session.update_description(target_version, new_description)?;
            }
            RepositoryOperation::RestoreVersion(target_version, dest) => {
                let target_version = target_version.resolve(session);
                let dest = dest.resolve(paths.versioned_file());
                session.restore_version(target_version, &dest)?;
            }
            RepositoryOperation::SoftReset(target_version) => {
                let target_version = target_version.resolve(session);
                session.soft_reset(target_version)?;
            }
            RepositoryOperation::Rebase(rebased_version, new_parent) => {
                let rebased_version = rebased_version.resolve(session);
                let new_parent = new_parent.resolve(session);
                session.rebase(rebased_version, new_parent)?;
            }
            RepositoryOperation::CreateBranch(branch_name, version) => {
                let version = version.resolve(session);
                session.create_branch(branch_name, version)?;
            }
            RepositoryOperation::RenameBranch(old_name, new_name) => {
                let old_name = old_name.resolve(session);
                session.rename_branch(&old_name, new_name)?;
            }
            RepositoryOperation::RemoveBranch(name) => {
                let name = name.resolve(session);
                session.remove_branch(&name)?;
            }
            RepositoryOperation::TidyUp => {
                session.tidy_up()?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum TargetBranch {
    Valid(Index),
    Any(BranchName),
}

impl TargetBranch {
    pub fn resolve(self, session: &Session) -> BranchName {
        match self {
            TargetBranch::Valid(index) => {
                let branches = session.tree().branch_names().collect_vec();
                index.get_copied(&branches).clone()
            }
            TargetBranch::Any(name) => name,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TargetVersion {
    Valid(Index),
    Any(VersionId),
}

impl TargetVersion {
    pub fn resolve(self, session: &Session) -> VersionId {
        match self {
            TargetVersion::Valid(index) => {
                let versions = session.tree().versions().map(|v| v.id).collect_vec();
                index.get_copied(&versions)
            }
            TargetVersion::Any(id) => id,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TargetFile {
    VersionedFile,
    Any(PathBuf),
}

impl TargetFile {
    pub fn resolve(self, versioned_file_path: &Path) -> PathBuf {
        match self {
            TargetFile::VersionedFile => versioned_file_path.to_path_buf(),
            TargetFile::Any(name) => versioned_file_path.parent().unwrap().join(name),
        }
    }
}
