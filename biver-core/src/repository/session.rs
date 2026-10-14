use crate::cache::Cache;
use crate::data::tree::{
    AddBranchError, MoveHeadToBranchError, MoveHeadToVersionError, NewVersion,
    OverwriteBranchTipError, OverwriteBranchTipOutcome, PopBranchOutcome, RemoveBranchError,
    RenameBranchError, VersionUpdate,
};
use crate::data::{BranchName, ContentBlobKind, Tree, VersionId};
use crate::error::Result;
use crate::repository::common::{versioned_file_meta, write_tree};
use crate::repository::paths::RepositoryPaths;
use crate::repository::preview::Preview;
use crate::utilities::external_preview_command::create_preview;
use crate::utilities::{external_diff_command, hash};
use crate::{Configuration, Error, temp_file};
use chrono::Utc;
use itertools::Itertools;
use std::fs::{File, OpenOptions, TryLockError};
use std::io::{Read, Seek};
use std::path::Path;
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
    config: Configuration,
    paths: RepositoryPaths,
    tree: Tree,
    cache: Cache,
    broken: bool,
    lock: File,
}

impl Session {
    pub(crate) fn new(
        config: Configuration,
        paths: RepositoryPaths,
        tree: Tree,
        cache: Cache,
    ) -> Result<Session> {
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
            cache,
            lock,
            broken: false,
        };

        Ok(session)
    }

    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    pub fn has_uncommitted_changes(&self) -> Result<bool> {
        use_break_non_critical!(self, {
            let versioned_file_metadata = fs::metadata(self.paths.versioned_file())?;
            let head_version = self.tree.head_version();

            if versioned_file_metadata.len() != head_version.versioned_file_length {
                return Ok(true);
            }

            let current_xxh3_128 = hash::xxh3_128(File::open(self.paths.versioned_file())?)?;

            Ok(head_version.versioned_file_xxh3_128 != current_xxh3_128)
        })
    }

    pub fn commit(&mut self, description: String) -> Result<CommitResult> {
        use_break!(self, {
            let Some(head_branch) = self.tree.head_branch().cloned() else {
                return Ok(CommitResult::HeadMustBeOnBranch);
            };

            let versioned_file_meta = versioned_file_meta(&self.paths)?;

            let head = self.tree.head_version();
            let head_version_id = head.id;

            if versioned_file_meta.xxh3_128 == head.versioned_file_xxh3_128 {
                return Ok(CommitResult::NoUncommittedChanges);
            }

            let preview_command = &self.config.preview_command;

            let new_version_id = self
                .tree
                .extend_branch(
                    &head_branch,
                    NewVersion {
                        creation_time: Utc::now(),
                        versioned_file_length: versioned_file_meta.length,
                        versioned_file_xxh3_128: versioned_file_meta.xxh3_128,
                        description,
                        content_blob_kind: ContentBlobKind::Patch,
                        has_preview: preview_command.is_some(),
                    },
                )
                .expect("Adding to head branch should succeed");

            let content_blob_file_path = self.paths.version_content_blob_file(new_version_id);
            let temp_parent_version_file_path = temp_file::new_path();
            self.extract_version_content(head_version_id, &temp_parent_version_file_path)?;
            self.store_patch(
                &temp_parent_version_file_path,
                self.paths.versioned_file(),
                &content_blob_file_path,
            )?;
            fs::remove_file(temp_parent_version_file_path)?;

            if let Some(preview_command) = preview_command {
                create_preview(
                    preview_command,
                    &self.paths.versioned_file(),
                    &self.paths.version_preview_file(new_version_id),
                )?;
            }

            self.cache
                .set_branch_tip(head_branch, new_version_id, self.paths.versioned_file())?;

            self.flush()?;

            Ok(CommitResult::Ok)
        })
    }

    pub fn amend(&mut self, description: Option<String>) -> Result<AmendResult> {
        use_break!(self, {
            let Some(head_branch) = self.tree.head_branch().cloned() else {
                return Ok(AmendResult::HeadMustBeBranch);
            };

            let head = self.tree.head_version();

            let versioned_file_meta = versioned_file_meta(&self.paths)?;

            if versioned_file_meta.xxh3_128 == head.versioned_file_xxh3_128 {
                return Ok(AmendResult::NoUncommittedChanges);
            }

            let head_branch_tip = self
                .tree
                .head_branch_tip()
                .expect("Already validated head is on a branch");

            let head_branch_tip_parent = head_branch_tip.parent;

            let content_blob_kind = if head_branch_tip.is_root() {
                ContentBlobKind::Full
            } else {
                ContentBlobKind::Patch
            };

            let preview_command = &self.config.preview_command;

            let overwrite_result =
                self.tree
                    .overwrite_branch_tip(&head_branch, |previous| NewVersion {
                        creation_time: Utc::now(),
                        versioned_file_length: versioned_file_meta.length,
                        versioned_file_xxh3_128: versioned_file_meta.xxh3_128,
                        description: description.unwrap_or(previous.description.clone()),
                        content_blob_kind,
                        has_preview: preview_command.is_some(),
                    });

            let (new_version_id, removed_version_id) = match overwrite_result {
                Ok(ok) => match ok {
                    OverwriteBranchTipOutcome::OverwrittenVersionStillInUse {
                        new_tip_id, ..
                    } => (new_tip_id, None),
                    OverwriteBranchTipOutcome::OverwrittenVersionRemoved {
                        former_tip,
                        new_tip_id,
                    } => (new_tip_id, Some(former_tip.id)),
                },
                Err(err) => match err {
                    OverwriteBranchTipError::BranchNotFound => panic!("Head branch not found"),
                    OverwriteBranchTipError::CannotOverwriteRootWhenOtherBranchesExist => {
                        return Ok(AmendResult::CannotAmendRootWhenOtherBranchesExist);
                    }
                },
            };

            let content_blob_file_path = self.paths.version_content_blob_file(new_version_id);

            match head_branch_tip_parent {
                Some(parent_id) => {
                    let temp_parent_version_file = temp_file::new_path();
                    self.extract_version_content(parent_id, &temp_parent_version_file)?;
                    self.store_patch(
                        &temp_parent_version_file,
                        self.paths.versioned_file(),
                        &content_blob_file_path,
                    )?;
                    fs::remove_file(temp_parent_version_file)?;
                }
                None => {
                    fs::copy(self.paths.versioned_file(), &content_blob_file_path)?;
                }
            }

            self.cache.set_branch_tip(
                head_branch,
                new_version_id,
                &self.paths.versioned_file().to_path_buf(),
            )?;

            self.flush()?;

            let cleanup_result = if let Some(removed_version_id) = removed_version_id {
                self.cleanup_erased_version_blobs([removed_version_id])
            } else {
                CleanupResult::Ok
            };

            Ok(AmendResult::Ok(cleanup_result))
        })
    }

    pub fn discard(&self) -> Result<()> {
        use_break_non_critical!(self, {
            self.overwrite_versioned_file_with_version(self.tree.head_version_id())?;
            Ok(())
        })
    }

    pub fn check_out_branch(&mut self, branch: BranchName) -> Result<CheckOutBranchResult> {
        use_break!(self, {
            if self.has_uncommitted_changes()? {
                return Ok(CheckOutBranchResult::HasUncommittedChanges);
            }

            if let Err(err) = self.tree.move_head_to_branch(branch.clone()) {
                return match err {
                    MoveHeadToBranchError::BranchNotFound => {
                        Ok(CheckOutBranchResult::BranchNotFound)
                    }
                };
            }

            let new_head_version_id = self.tree.head_version_id();

            self.overwrite_versioned_file_with_version(new_head_version_id)?;
            self.cache
                .set_branch_tip(branch, new_head_version_id, self.paths.versioned_file())?;

            self.flush()?;

            Ok(CheckOutBranchResult::Ok)
        })
    }

    pub fn check_out_version(&mut self, version_id: VersionId) -> Result<CheckOutVersionResult> {
        use_break!(self, {
            if self.has_uncommitted_changes()? {
                return Ok(CheckOutVersionResult::HasUncommittedChanges);
            }

            if let Err(err) = self.tree.move_head_to_version(version_id) {
                return match err {
                    MoveHeadToVersionError::VersionNotFound => {
                        Ok(CheckOutVersionResult::VersionNotFound)
                    }
                };
            };

            self.overwrite_versioned_file_with_version(version_id)?;

            self.flush()?;

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

            self.flush()?;

            Ok(UpdateDescriptionResult::Ok)
        })
    }

    pub fn restore_version(
        &self,
        version_id: VersionId,
        destination: &Path,
    ) -> Result<RestoreVersionResult> {
        use_break_non_critical!(self, {
            if fs::canonicalize(destination)? == fs::canonicalize(self.paths.versioned_file())?
                && self.has_uncommitted_changes()?
            {
                return Ok(RestoreVersionResult::WouldOverwriteUncommittedChanges);
            }

            if self.tree.version(version_id).is_none() {
                return Ok(RestoreVersionResult::VersionNotFound);
            }

            self.extract_version_content(version_id, destination)?;

            Ok(RestoreVersionResult::Ok)
        })
    }

    pub fn soft_reset(&mut self, stop_at_version_id: VersionId) -> Result<SoftResetResult> {
        use_break!(self, {
            let Some(head_branch) = self.tree.head_branch().cloned() else {
                return Ok(SoftResetResult::HeadMustBeBranch);
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

            let mut removed_version_ids = Vec::new();

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
                        removed_version_ids.push(former_tip.id);
                        new_tip_id
                    }
                };

                if new_tip_id == stop_at_version_id {
                    break;
                }
            }

            self.cache.invalidate_branch_tip(&head_branch);

            self.flush()?;

            let cleanup_result = self.cleanup_erased_version_blobs(removed_version_ids);

            Ok(SoftResetResult::Ok(cleanup_result))
        })
    }

    pub fn create_branch(
        &mut self,
        branch_name: BranchName,
        branch_tip_id: VersionId,
    ) -> Result<CreateBranchResult> {
        use_break!(self, {
            if let Err(err) = self.tree.add_branch(branch_name, branch_tip_id) {
                return match err {
                    AddBranchError::NameAlreadyExists => Ok(CreateBranchResult::AlreadyExists),
                    AddBranchError::VersionNotFound => Ok(CreateBranchResult::InvalidBranchTipId),
                };
            }

            self.flush()?;

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

            self.flush()?;

            Ok(RenameBranchResult::Ok)
        })
    }

    pub fn remove_branch(&mut self, branch_name: &BranchName) -> Result<RemoveBranchResult> {
        use_break!(self, {
            let removed_versions = match self.tree.remove_branch(branch_name) {
                Ok(ok) => ok.removed_versions,
                Err(err) => {
                    return match err {
                        RemoveBranchError::BranchNotFound => Ok(RemoveBranchResult::BranchNotFound),
                        RemoveBranchError::CannotRemoveHeadBranch => {
                            Ok(RemoveBranchResult::CannotRemoveHeadBranch)
                        }
                    };
                }
            };

            self.flush()?;

            let cleanup_result =
                self.cleanup_erased_version_blobs(removed_versions.iter().map(|v| v.id));

            Ok(RemoveBranchResult::Ok(cleanup_result))
        })
    }

    pub fn raw_content_blob(&self, version_id: VersionId) -> Result<Option<impl Read + Seek>> {
        if self.tree.version(version_id).is_none() {
            return Ok(None);
        }

        let path = self.paths.version_content_blob_file(version_id);

        let file = File::open(path)?;

        Ok(Some(file))
    }

    pub fn preview(&self, version_id: VersionId) -> Result<Option<Preview>> {
        let Some(version) = self.tree.version(version_id) else {
            return Ok(None);
        };

        if !version.has_preview {
            return Ok(None);
        }

        let path = self.paths.version_preview_file(version_id);

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

    fn flush(&mut self) -> Result<()> {
        let result = write_tree(&self.paths, &self.tree);

        if result.is_err() {
            self.cache.invalidate_all();
        }
        self.cache.flush()?;

        result
    }

    fn overwrite_versioned_file_with_version(&self, version_id: VersionId) -> Result<()> {
        self.extract_version_content(version_id, self.paths.versioned_file())?;
        Ok(())
    }

    fn extract_version_content(&self, version_id: VersionId, destination: &Path) -> Result<()> {
        let mut chain = self
            .tree
            .version_and_ancestors(version_id)
            .expect("version_id was invalid")
            .map(|v| (v, self.cache.get(v.id)))
            .take_while_inclusive(|(v, cache)| v.content_blob_kind.is_patch() && cache.is_none())
            .collect_vec();

        assert!(!chain.is_empty());

        chain.reverse();

        let (starting_version, cache) = &chain[0];

        let full_blob = if let Some(cache) = cache {
            cache
        } else {
            assert!(starting_version.content_blob_kind.is_full());
            &self.paths.version_content_blob_file(starting_version.id)
        };

        let a = temp_file::new_path();
        let b = temp_file::new_path();

        let mut actual = &a;
        let mut stale = &b;

        if chain.len() == 1 {
            fs::copy(&full_blob, destination)?;
            return Ok(());
        } else {
            fs::copy(&full_blob, actual)?;
        }

        for (version, _) in chain.iter().skip(1) {
            assert!(version.content_blob_kind.is_patch());

            let patch = self.paths.version_content_blob_file(version.id);

            if fs::exists(&stale)? {
                fs::remove_file(&stale)?;
            }

            external_diff_command::apply_patch(&self.config, &actual, &patch, &stale)?;
            mem::swap(&mut actual, &mut stale);
        }

        fs::copy(actual, destination)?;

        fs::remove_file(&a)?;
        fs::remove_file(&b)?;

        Ok(())
    }

    fn store_patch(&self, old_path: &Path, new_path: &Path, patch: &Path) -> Result<()> {
        external_diff_command::create_patch(&self.config, old_path, new_path, patch)?;
        Ok(())
    }

    fn cleanup_erased_version_blobs(
        &self,
        version_ids: impl IntoIterator<Item = VersionId>,
    ) -> CleanupResult {
        let mut cleanup_result = CleanupResult::Ok;

        for id in version_ids.into_iter() {
            if fs::remove_file(self.paths.version_content_blob_file(id)).is_err()
                || fs::remove_file(self.paths.version_preview_file(id)).is_err()
            {
                cleanup_result = CleanupResult::Failed;
            }
        }

        cleanup_result
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
    Ok,
    NoUncommittedChanges,
    HeadMustBeOnBranch,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum CheckOutBranchResult {
    Ok,
    HasUncommittedChanges,
    BranchNotFound,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum CheckOutVersionResult {
    Ok,
    HasUncommittedChanges,
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
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum SoftResetResult {
    Ok(CleanupResult),
    HeadBranchTipIsAlreadyThere,
    HeadMustBeBranch,
    InvalidStopAtVersionId,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum AmendResult {
    Ok(CleanupResult),
    NoUncommittedChanges,
    HeadMustBeBranch,
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
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum CleanupResult {
    Ok,
    Failed,
}
