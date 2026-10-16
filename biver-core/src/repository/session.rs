use crate::data::tree::{
    AddBranchError, MoveHeadToBranchError, MoveHeadToVersionError, NewVersion,
    OverwriteBranchTipError, OverwriteBranchTipOutcome, PopBranchOutcome, RemoveBranchError,
    RenameBranchError, VersionUpdate,
};
use crate::data::{BranchName, Tree, Version, VersionId};
use crate::error::Result;
use crate::repository::common;
use crate::repository::paths::Paths;
use crate::repository::preview::Preview;
use crate::utilities::external_preview_command::create_preview;
use crate::utilities::{external_diff_command, xxh3_128};
use crate::{Config, Error, temp_file};
use chrono::Utc;
use std::fs::{File, OpenOptions, TryLockError};
use std::io::{Read, Seek};
use std::ops::{Add, AddAssign};
use std::path::{Path, PathBuf};
use std::{fs, mem};
use variantly::Variantly;

macro_rules! use_break {
    ($self:ident, $body:block) => {{
        if $self.broken {
            return Err(Error::Broken);
        }

        let result = (|| $body)();
        if result.is_err() {
            $self.broken = true;
        }

        result
    }};
}

macro_rules! use_break_non_critical {
    ($self:ident, $body:block) => {{
        if $self.broken {
            return Err(Error::Broken);
        }

        (|| $body)()
    }};
}

#[derive(Debug)]
pub struct Session {
    config: Config,
    paths: Paths,
    tree: Tree,
    broken: bool,
    lock: File,
}

impl Session {
    pub(crate) fn new(config: Config, paths: Paths, tree: Tree) -> Result<Session> {
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(paths.lock_file())?;

        if let Err(err) = lock.try_lock() {
            return match err {
                TryLockError::WouldBlock => Err(Error::RepositoryLocked),
                TryLockError::Error(err) => Err(err.into()),
            };
        }

        let session = Session {
            config,
            paths,
            tree,
            lock,
            broken: false,
        };

        Ok(session)
    }

    pub fn versioned_file_path(&self) -> &Path {
        self.paths.versioned_file()
    }

    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    pub fn has_uncommitted_changes(&self) -> Result<bool> {
        use_break_non_critical!(self, {
            let head_version_xxh3_128 = self.tree.head_version().xxh3_128;
            let versioned_file_xxh3_128 = self.versioned_file_xxh3_128()?;

            Ok(head_version_xxh3_128 != versioned_file_xxh3_128)
        })
    }

    pub fn commit(&mut self, description: String) -> Result<CommitResult> {
        use_break!(self, {
            let Some(head_branch) = self.tree.head_branch().cloned() else {
                return Ok(CommitResult::HeadMustBeOnBranch);
            };

            let versioned_file_xxh3_128 = self.versioned_file_xxh3_128()?;

            let head_version = self.tree.head_version();
            let parent_of_new_version_id = head_version.id;

            if versioned_file_xxh3_128 == head_version.xxh3_128 {
                return Ok(CommitResult::NoUncommittedChanges);
            }

            let new_version_id = self
                .tree
                .extend_branch(
                    &head_branch,
                    NewVersion {
                        creation_time: Utc::now(),
                        xxh3_128: versioned_file_xxh3_128,
                        description,
                    },
                )
                .expect("Adding to head branch should succeed");

            let new_version_full_content_path =
                self.store_full_content(self.paths.versioned_file(), new_version_id)?;

            let parent_full_content_path = self
                .paths
                .version_full_content_file(parent_of_new_version_id);

            self.store_reverse_patch(
                new_version_id,
                &new_version_full_content_path,
                &parent_full_content_path,
            )?;

            if let Some(preview_command) = &self.config.preview_command {
                create_preview(
                    preview_command,
                    &self.paths.versioned_file(),
                    &self.paths.version_preview_file(new_version_id),
                )?;
            }

            self.flush_tree()?;

            let cleanup_result = self.cleanup_full_content_if_unused(parent_of_new_version_id);

            Ok(CommitResult::Ok(cleanup_result))
        })
    }

    pub fn amend(&mut self, description: Option<String>) -> Result<AmendResult> {
        use_break!(self, {
            let Some(head_branch) = self.tree.head_branch().cloned() else {
                return Ok(AmendResult::HeadMustBeOnBranch);
            };

            let versioned_file_xxh3_128 = self.versioned_file_xxh3_128()?;

            let head_version = self.tree.head_version();

            let parent_of_new_version_extr_path = if let Some(parent_id) = head_version.parent_id {
                Some(self.extraction_path(parent_id)?)
            } else {
                None
            };

            if versioned_file_xxh3_128 == head_version.xxh3_128 {
                return Ok(AmendResult::NoUncommittedChanges);
            }

            let former_head_id = head_version.id;

            let overwrite_result =
                self.tree
                    .overwrite_branch_tip(&head_branch, |previous| NewVersion {
                        creation_time: Utc::now(),
                        xxh3_128: versioned_file_xxh3_128,
                        description: description.unwrap_or(previous.description.clone()),
                    });

            let (new_version_id, removed_version) = match overwrite_result {
                Ok(ok) => match ok {
                    OverwriteBranchTipOutcome::OverwrittenVersionStillInUse {
                        new_tip_id, ..
                    } => (new_tip_id, None),
                    OverwriteBranchTipOutcome::OverwrittenVersionRemoved {
                        former_tip,
                        new_tip_id,
                    } => (new_tip_id, Some(former_tip)),
                },
                Err(err) => match err {
                    OverwriteBranchTipError::BranchNotFound => panic!("Head branch not found"),
                    OverwriteBranchTipError::CannotOverwriteRootWhenOtherBranchesExist => {
                        return Ok(AmendResult::CannotAmendRootWhenOtherBranchesExist);
                    }
                },
            };

            let new_version_full_content_path =
                self.store_full_content(self.paths.versioned_file(), new_version_id)?;

            if let Some(parent_of_new_version_extr_path) = parent_of_new_version_extr_path {
                let parent_full_content_path = temp_file::new_path();
                self.extract_path(&parent_of_new_version_extr_path, &parent_full_content_path)?;
                self.store_reverse_patch(
                    new_version_id,
                    &new_version_full_content_path,
                    &parent_full_content_path,
                )?;
            }

            if let Some(preview_command) = &self.config.preview_command {
                create_preview(
                    preview_command,
                    &self.paths.versioned_file(),
                    &self.paths.version_preview_file(new_version_id),
                )?;
            }

            self.flush_tree()?;

            let mut cleanup_result = self.cleanup_full_content_if_unused(former_head_id);

            cleanup_result += if let Some(removed_version) = removed_version {
                self.cleanup_removed_version_content([removed_version])
            } else {
                CleanupResult::Ok
            };

            Ok(AmendResult::Ok(cleanup_result))
        })
    }

    pub fn discard(&self) -> Result<()> {
        use_break_non_critical!(self, {
            let extr_path = self.extraction_path(self.tree.head_version_id())?;
            self.extract_into_versioned_file(&extr_path)?;
            Ok(())
        })
    }

    pub fn check_out_branch(&mut self, branch: BranchName) -> Result<CheckOutBranchResult> {
        use_break!(self, {
            let has_uncommitted_changes = self.has_uncommitted_changes()?;

            if let Err(err) = self.tree.move_head_to_branch(branch) {
                return match err {
                    MoveHeadToBranchError::BranchNotFound => {
                        Ok(CheckOutBranchResult::BranchNotFound)
                    }
                };
            }

            let new_head_version_id = self.tree.head_version_id();

            if !has_uncommitted_changes {
                let extr_path = self.extraction_path(new_head_version_id)?;
                self.extract_into_versioned_file(&extr_path)?;
            }

            self.flush_tree()?;

            Ok(CheckOutBranchResult::Ok)
        })
    }

    pub fn check_out_version(&mut self, version_id: VersionId) -> Result<CheckOutVersionResult> {
        use_break!(self, {
            let has_uncommitted_changes = self.has_uncommitted_changes()?;

            if let Err(err) = self.tree.move_head_to_version(version_id) {
                return match err {
                    MoveHeadToVersionError::VersionNotFound => {
                        Ok(CheckOutVersionResult::VersionNotFound)
                    }
                };
            };

            if !has_uncommitted_changes {
                let extr_path = self.extraction_path(version_id)?;
                self.extract_into_versioned_file(&extr_path)?;
            }

            self.flush_tree()?;

            Ok(CheckOutVersionResult::Ok)
        })
    }

    pub fn update_description(
        &mut self,
        version_id: VersionId,
        new_description: String,
    ) -> Result<UpdateDescriptionResult> {
        use_break!(self, {
            let updated = self.tree.update_version(
                version_id,
                VersionUpdate {
                    description: Some(new_description),
                },
            );

            if !updated {
                return Ok(UpdateDescriptionResult::VersionNotFound);
            }

            self.flush_tree()?;

            Ok(UpdateDescriptionResult::Ok)
        })
    }

    pub fn restore_version(
        &self,
        version_id: VersionId,
        destination: &Path,
    ) -> Result<RestoreVersionResult> {
        use_break_non_critical!(self, {
            let destination = if fs::exists(destination)? {
                &fs::canonicalize(destination)?
            } else {
                destination
            };

            let versioned_file = fs::canonicalize(self.paths.versioned_file())?;

            if destination == versioned_file && self.has_uncommitted_changes()? {
                return Ok(RestoreVersionResult::WouldOverwriteUncommittedChanges);
            }

            let repository_dir = fs::canonicalize(self.paths.repository_dir())?;

            if destination.starts_with(repository_dir) {
                return Ok(RestoreVersionResult::CannotRestoreIntoRepositoryDir);
            }

            if self.tree.version(version_id).is_none() {
                return Ok(RestoreVersionResult::VersionNotFound);
            }

            if let Some(dest_parent) = destination.parent() {
                fs::create_dir_all(dest_parent)?;
            }

            let extr_path = self.extraction_path(version_id)?;
            self.extract_path(&extr_path, &destination)?;

            Ok(RestoreVersionResult::Ok)
        })
    }

    pub fn soft_reset(&mut self, stop_at_version_id: VersionId) -> Result<SoftResetResult> {
        use_break!(self, {
            let Some(head_branch) = self.tree.head_branch().cloned() else {
                return Ok(SoftResetResult::HeadMustBeOnBranch);
            };

            let stop_at_version_index =
                self.tree
                    .head_and_ancestors()
                    .enumerate()
                    .find_map(|(i, v)| {
                        if v.id == stop_at_version_id {
                            Some(i)
                        } else {
                            None
                        }
                    });

            let Some(stop_at_version_index) = stop_at_version_index else {
                return Ok(SoftResetResult::InvalidStopAtVersionId);
            };

            if stop_at_version_index == 0 {
                return Ok(SoftResetResult::HeadBranchTipIsAlreadyThere);
            }

            let former_head_id = self.tree.head_version_id();

            let stop_at_version_extr_path = self.extraction_path(stop_at_version_id)?;

            let mut removed_versions = Vec::new();

            loop {
                let pop_ok = self
                    .tree
                    .pop_branch(&head_branch)
                    .expect("Head branch is valid and it will not reach past root");

                let new_tip_id = match pop_ok {
                    PopBranchOutcome::PoppedVersionStillInUse { new_tip_id, .. } => new_tip_id,
                    PopBranchOutcome::PoppedVersionRemoved {
                        new_tip_id,
                        former_tip,
                    } => {
                        removed_versions.push(former_tip);
                        new_tip_id
                    }
                };

                if new_tip_id == stop_at_version_id {
                    break;
                }
            }

            let temp = temp_file::new_path();
            self.extract_path(&stop_at_version_extr_path, &temp)?;
            self.store_full_content(&temp, stop_at_version_id)?;
            fs::remove_file(temp)?;

            self.flush_tree()?;

            let cleanup_result = self.cleanup_full_content_if_unused(former_head_id)
                + self.cleanup_removed_version_content(removed_versions);

            Ok(SoftResetResult::Ok(cleanup_result))
        })
    }

    pub fn create_branch(
        &mut self,
        branch_name: BranchName,
        branch_tip_id: VersionId,
    ) -> Result<CreateBranchResult> {
        use_break!(self, {
            if self.tree.version(branch_tip_id).is_none() {
                return Ok(CreateBranchResult::InvalidBranchTipId);
            }

            let branch_tip_extr_path = self.extraction_path(branch_tip_id)?;

            if let Err(err) = self.tree.add_branch(branch_name, branch_tip_id) {
                return match err {
                    AddBranchError::NameAlreadyExists => Ok(CreateBranchResult::AlreadyExists),
                    AddBranchError::VersionNotFound => Ok(CreateBranchResult::InvalidBranchTipId),
                };
            }

            let full_content_path = self.paths.version_full_content_file(branch_tip_id);

            if !fs::exists(&full_content_path)? {
                self.extract_path(&branch_tip_extr_path, &full_content_path)?;
            }

            self.flush_tree()?;

            Ok(CreateBranchResult::Ok)
        })
    }

    pub fn rename_branch(
        &mut self,
        old_branch_name: &BranchName,
        new_branch_name: BranchName,
    ) -> Result<RenameBranchResult> {
        use_break!(self, {
            if *old_branch_name == new_branch_name {
                return Ok(RenameBranchResult::Ok);
            }

            if let Err(err) = self.tree.rename_branch(old_branch_name, new_branch_name) {
                return match err {
                    RenameBranchError::NewNameAlreadyExists => {
                        Ok(RenameBranchResult::NewNameAlreadyExists)
                    }
                    RenameBranchError::OldNameNotFound => Ok(RenameBranchResult::OldNameNotFound),
                };
            }

            self.flush_tree()?;

            Ok(RenameBranchResult::Ok)
        })
    }

    pub fn remove_branch(&mut self, branch_name: &BranchName) -> Result<RemoveBranchResult> {
        use_break!(self, {
            let Some(branch_tip_id) = self.tree.branch_tip_ids().get(branch_name).copied() else {
                return Ok(RemoveBranchResult::BranchNotFound);
            };

            let removed_versions = match self.tree.remove_branch(branch_name) {
                Ok(ok) => ok.removed_versions,
                Err(err) => {
                    return match err {
                        RemoveBranchError::BranchNotFound => Ok(RemoveBranchResult::BranchNotFound),
                        RemoveBranchError::CannotRemoveHeadBranch => {
                            Ok(RemoveBranchResult::CannotRemoveHeadBranch)
                        }
                        RemoveBranchError::CannotRemoveHeadVersion => {
                            Ok(RemoveBranchResult::CannotRemoveHeadVersion)
                        }
                    };
                }
            };

            self.flush_tree()?;

            let cleanup_result = self.cleanup_full_content_if_unused(branch_tip_id)
                + self.cleanup_removed_version_content(removed_versions);

            Ok(RemoveBranchResult::Ok(cleanup_result))
        })
    }

    pub fn version_full_content(&self, version_id: VersionId) -> Result<Option<impl Read + Seek>> {
        if self.tree.version(version_id).is_none() {
            return Ok(None);
        }

        let path = self.paths.version_full_content_file(version_id);

        if !fs::exists(&path)? {
            return Ok(None);
        }

        Ok(Some(File::open(path)?))
    }

    pub fn version_reverse_patch(&self, version_id: VersionId) -> Result<Option<impl Read + Seek>> {
        if self.tree.version(version_id).is_none() {
            return Ok(None);
        }

        let path = self.paths.version_reverse_patch_file(version_id);

        if !fs::exists(&path)? {
            return Ok(None);
        }

        Ok(Some(File::open(path)?))
    }

    pub fn preview(&self, version_id: VersionId) -> Result<Option<Preview>> {
        if self.tree.version(version_id).is_none() {
            return Ok(None);
        }

        let path = self.paths.version_preview_file(version_id);

        if !fs::exists(&path)? {
            return Ok(None);
        }

        Ok(Some(Preview::new(path)?))
    }

    pub fn preview_versioned_file(&self) -> Result<Option<Preview>> {
        let Some(preview_command) = &self.config.preview_command else {
            return Ok(None);
        };

        let temp = temp_file::new_path();
        create_preview(preview_command, self.paths.versioned_file(), &temp)?;

        Ok(Some(Preview::new(temp)?.with_cleanup()))
    }

    fn flush_tree(&self) -> Result<()> {
        common::write_tree(&self.paths, &self.tree)?;
        Ok(())
    }

    fn extraction_path(&self, version_id: VersionId) -> Result<ExtractionPath> {
        let mut chains = self.tree.branch_tip_to_version_chains(version_id);
        chains.sort_unstable_by(|a, b| a.len().cmp(&b.len()));

        let chain = chains
            .first()
            .expect("There should be at least one path to a valid ID");

        let full_content_path = self.paths.version_full_content_file(chain[0].id);

        assert!(fs::exists(&full_content_path)?);

        let mut patches = Vec::with_capacity(chain.len() - 1);

        for v in chain.iter().take(chain.len() - 1) {
            let reverse_patch_path = self.paths.version_reverse_patch_file(v.id);
            assert!(fs::exists(&reverse_patch_path)?);
            patches.push(reverse_patch_path);
        }

        Ok(ExtractionPath {
            starting_full: full_content_path,
            patches,
        })
    }

    fn extract_path(&self, path: &ExtractionPath, destination: &Path) -> Result<()> {
        if path.patches.is_empty() {
            fs::copy(&path.starting_full, destination)?;
            return Ok(());
        }

        let a = temp_file::new_path();
        let b = temp_file::new_path();

        let mut actual = &a;
        let mut stale = &b;

        fs::copy(&path.starting_full, actual)?;

        for patch_path in &path.patches {
            if fs::exists(&stale)? {
                fs::remove_file(&stale)?;
            }

            external_diff_command::apply_patch(&self.config, &actual, patch_path, &stale)?;
            mem::swap(&mut actual, &mut stale);
        }

        fs::copy(actual, destination)?;

        fs::remove_file(&a)?;
        fs::remove_file(&b)?;

        Ok(())
    }

    fn extract_into_versioned_file(&self, path: &ExtractionPath) -> Result<()> {
        self.extract_path(path, self.paths.versioned_file())?;
        Ok(())
    }

    fn store_full_content(&self, source: &Path, version: VersionId) -> Result<PathBuf> {
        let dest_path = self.paths.version_full_content_file(version);
        fs::copy(source, &dest_path)?;
        Ok(dest_path)
    }

    fn store_reverse_patch(
        &self,
        version_id: VersionId,
        version_full_content: &Path,
        parent_full_content: &Path,
    ) -> Result<()> {
        let reverse_patch_path = self.paths.version_reverse_patch_file(version_id);
        external_diff_command::create_patch(
            &self.config,
            version_full_content,
            parent_full_content,
            &reverse_patch_path,
        )?;
        Ok(())
    }

    fn cleanup_removed_version_content(
        &self,
        versions: impl IntoIterator<Item = Version>,
    ) -> CleanupResult {
        let mut cleanup_result = CleanupResult::Ok;

        for version in versions.into_iter() {
            let full_content_path = self.paths.version_full_content_file(version.id);
            let cleanup_full_content_result = fs::exists(&full_content_path).and_then(|exists| {
                if exists {
                    fs::remove_file(&full_content_path)?;
                }
                Ok(())
            });

            let reverse_patch_path = self.paths.version_reverse_patch_file(version.id);
            let cleanup_reverse_patch_result = fs::exists(&reverse_patch_path).and_then(|exists| {
                if exists {
                    fs::remove_file(&reverse_patch_path)?;
                }
                Ok(())
            });

            let preview_path = self.paths.version_preview_file(version.id);
            let cleanup_preview_result = fs::exists(&preview_path).and_then(|exists| {
                if exists {
                    fs::remove_file(&preview_path)?;
                }
                Ok(())
            });

            if cleanup_full_content_result.is_err()
                || cleanup_reverse_patch_result.is_err()
                || cleanup_preview_result.is_err()
            {
                cleanup_result = CleanupResult::Failed;
            }
        }

        cleanup_result
    }

    fn cleanup_full_content_if_unused(&self, version_id: VersionId) -> CleanupResult {
        if self.tree.is_branch_tip(version_id) {
            return CleanupResult::Ok;
        }

        let path = self.paths.version_full_content_file(version_id);

        if fs::remove_file(path).is_ok() {
            CleanupResult::Ok
        } else {
            CleanupResult::Failed
        }
    }

    fn versioned_file_xxh3_128(&self) -> Result<u128> {
        let value = xxh3_128::from_file_path(&self.paths.versioned_file())?;
        Ok(value)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // Explicit unlock in case file descriptor got shared with a child process
        let _ = self.lock.unlock();
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum CommitResult {
    Ok(CleanupResult),
    NoUncommittedChanges,
    HeadMustBeOnBranch,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum CheckOutBranchResult {
    Ok,
    BranchNotFound,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum CheckOutVersionResult {
    Ok,
    VersionNotFound,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum UpdateDescriptionResult {
    Ok,
    VersionNotFound,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum RestoreVersionResult {
    Ok,
    WouldOverwriteUncommittedChanges,
    VersionNotFound,
    CannotRestoreIntoRepositoryDir,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum SoftResetResult {
    Ok(CleanupResult),
    HeadBranchTipIsAlreadyThere,
    HeadMustBeOnBranch,
    InvalidStopAtVersionId,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum AmendResult {
    Ok(CleanupResult),
    NoUncommittedChanges,
    HeadMustBeOnBranch,
    CannotAmendRootWhenOtherBranchesExist,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum CreateBranchResult {
    Ok,
    AlreadyExists,
    InvalidBranchTipId,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum RenameBranchResult {
    Ok,
    NewNameAlreadyExists,
    OldNameNotFound,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum RemoveBranchResult {
    Ok(CleanupResult),
    BranchNotFound,
    CannotRemoveHeadBranch,
    CannotRemoveHeadVersion,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum CleanupResult {
    Ok,
    Failed,
}

impl Add for CleanupResult {
    type Output = CleanupResult;
    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (CleanupResult::Failed, _) => CleanupResult::Failed,
            (_, CleanupResult::Failed) => CleanupResult::Failed,
            _ => CleanupResult::Ok,
        }
    }
}

impl AddAssign for CleanupResult {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

#[derive(Debug)]
struct ExtractionPath {
    starting_full: PathBuf,
    patches: Vec<PathBuf>,
}
