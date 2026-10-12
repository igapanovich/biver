use crate::data::{BranchName, ContentBlobKind, Head, Tree, Version, VersionId};
use crate::error::Result;
use crate::repository::common::{maybe_store_preview, write_tree};
use crate::repository::paths::RepositoryPaths;
use crate::repository::preview::Preview;
use crate::utilities::external_preview_command::create_preview;
use crate::utilities::{external_diff_command, hash, nickname};
use crate::{Configuration, Error, temp_file};
use chrono::Utc;
use derive_more::IsVariant;
use itertools::Itertools;
use std::fs::{File, OpenOptions, TryLockError};
use std::io::{Read, Seek};
use std::path::Path;
use std::{fs, mem};

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
    broken: bool,
    _lock: File,
}

impl Session {
    pub(crate) fn new(
        config: Configuration,
        paths: RepositoryPaths,
        tree: Tree,
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
            _lock: lock,
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
            let Some(head_branch) = self.tree.head.branch() else {
                return Ok(CommitResult::HeadMustBeOnBranch);
            };

            let versioned_file_xxh3_128 =
                hash::xxh3_128(File::open(&self.paths.versioned_file())?)?;
            let versioned_file_length = fs::metadata(&self.paths.versioned_file())?.len();

            let head = self.tree.head_version();

            if versioned_file_xxh3_128 == head.versioned_file_xxh3_128 {
                return Ok(CommitResult::NoUncommittedChanges);
            }

            let new_version_id = VersionId::new();

            let content_blob_file_path = self.paths.content_blob_path(new_version_id);

            let temp_parent_version_file_path = temp_file::new_path();
            self.extract_version_content(head.id, &temp_parent_version_file_path)?;
            self.store_patch(
                &temp_parent_version_file_path,
                self.paths.versioned_file(),
                &content_blob_file_path,
            )?;
            fs::remove_file(temp_parent_version_file_path)?;

            let has_preview = if let Some(preview_command) = &self.config.preview_command {
                create_preview(
                    preview_command,
                    &self.paths.versioned_file(),
                    &self.paths.preview_path(new_version_id),
                )?;

                true
            } else {
                false
            };

            let new_version = Version {
                id: new_version_id,
                creation_time: Utc::now(),
                nickname: nickname::new_nickname(versioned_file_xxh3_128),
                versioned_file_length,
                versioned_file_xxh3_128,
                description,
                parent: Some(head.id),
                content_blob_kind: ContentBlobKind::Patch,
                has_preview,
            };

            self.tree.versions.push(new_version);

            let head_branch_tip = self
                .tree
                .branches
                .get_mut(head_branch)
                .expect("Head must point at a valid branch");
            *head_branch_tip = new_version_id;

            write_tree(&self.paths, &self.tree)?;

            Ok(CommitResult::Ok)
        })
    }

    pub fn amend(&mut self, description: Option<String>) -> Result<AmendResult> {
        use_break!(self, {
            let Some(head_branch) = self.tree.head.branch() else {
                return Ok(AmendResult::HeadMustBeBranch);
            };

            let Some(head_branch_tip) = self
                .tree
                .branch_tip_and_ancestors_exclusive(head_branch)
                .next()
            else {
                return Ok(AmendResult::CannotAmendVersionBelongingToAnotherBranch);
            };

            let head_branch_tip_index = self
                .tree
                .versions
                .iter()
                .enumerate()
                .filter_map(|(i, v)| {
                    if v.id == head_branch_tip.id {
                        Some(i)
                    } else {
                        None
                    }
                })
                .next()
                .expect("Head branch tip should be in versions");

            let erased_version = self.tree.versions.remove(head_branch_tip_index);

            let versioned_file_xxh3_128 =
                hash::xxh3_128(File::open(&self.paths.versioned_file())?)?;
            let versioned_file_length = fs::metadata(&self.paths.versioned_file())?.len();

            let head = self.tree.head_version();

            if versioned_file_xxh3_128 == head.versioned_file_xxh3_128 {
                return Ok(AmendResult::NoUncommittedChanges);
            }

            let new_version_id = VersionId::new();

            let content_blob_file_path = self.paths.content_blob_path(new_version_id);
            let content_blob_kind = if erased_version.is_root() {
                fs::copy(self.paths.versioned_file(), &content_blob_file_path)?;

                ContentBlobKind::Full
            } else {
                let temp_parent_version_file = temp_file::new_path();
                self.extract_version_content(head.id, &temp_parent_version_file)?;
                self.store_patch(
                    &temp_parent_version_file,
                    self.paths.versioned_file(),
                    &content_blob_file_path,
                )?;
                fs::remove_file(temp_parent_version_file)?;

                ContentBlobKind::Patch
            };

            let has_preview = maybe_store_preview(&self.config, &self.paths, new_version_id)?;

            let new_version = Version {
                id: new_version_id,
                creation_time: Utc::now(),
                nickname: nickname::new_nickname(versioned_file_xxh3_128),
                versioned_file_length,
                versioned_file_xxh3_128,
                description: description.unwrap_or(erased_version.description),
                parent: erased_version.parent,
                content_blob_kind,
                has_preview,
            };

            self.tree.versions.push(new_version);
            self.tree.versions.retain(|v| v.id != erased_version.id);

            let head_branch_tip = self
                .tree
                .branches
                .get_mut(head_branch)
                .expect("Head must point at a valid branch");
            *head_branch_tip = new_version_id;

            self.write_tree()?;

            let cleanup_result = self.cleanup_erased_version_blobs([erased_version.id]);

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

            if !self.tree.branches.contains_key(&branch) {
                return Ok(CheckOutBranchResult::BranchNotFound);
            }

            self.tree.head = Head::Branch(branch);
            let new_head_version_id = self.tree.head_version_id();

            self.write_tree()?;
            self.overwrite_versioned_file_with_version(new_head_version_id)?;

            Ok(CheckOutBranchResult::Ok)
        })
    }

    pub fn check_out_version(&mut self, version_id: VersionId) -> Result<CheckOutVersionResult> {
        use_break!(self, {
            if self.has_uncommitted_changes()? {
                return Ok(CheckOutVersionResult::HasUncommittedChanges);
            }

            if self.tree.version(version_id).is_none() {
                return Ok(CheckOutVersionResult::VersionNotFound);
            };

            self.tree.head = Head::Version(version_id);

            self.write_tree()?;
            self.overwrite_versioned_file_with_version(version_id)?;

            Ok(CheckOutVersionResult::Ok)
        })
    }

    pub fn update_description(
        &mut self,
        version_id: VersionId,
        new_description: String,
    ) -> Result<UpdateDescriptionResult> {
        use_break!(self, {
            let Some(version) = self.tree.version_mut(version_id) else {
                return Ok(UpdateDescriptionResult::VersionNotFound);
            };

            version.description = new_description;

            self.write_tree()?;

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
            let Some(head_branch) = self.tree.head.branch() else {
                return Ok(SoftResetResult::HeadMustBeBranch);
            };

            let erased_versions = self
                .tree
                .branch_tip_and_ancestors_exclusive(head_branch)
                .take_while(|v| v.id != stop_at_version_id)
                .collect_vec();

            let erased_versions_contain_root = erased_versions.iter().any(|v| v.is_root());
            if erased_versions_contain_root {
                return Ok(SoftResetResult::InvalidResetRange);
            }

            let erased_versions_reached_stop_at_version =
                erased_versions.last().and_then(|v| v.parent) == Some(stop_at_version_id);
            if !erased_versions_reached_stop_at_version {
                return Ok(SoftResetResult::ResetRangeOverlapsWithOtherBranches);
            }

            let erased_version_ids = erased_versions.iter().map(|v| v.id).collect_vec();

            self.tree
                .versions
                .retain(|v| !erased_version_ids.contains(&v.id));

            let head_branch_tip = self
                .tree
                .branches
                .get_mut(&head_branch)
                .expect("Head must point at a valid branch");

            *head_branch_tip = stop_at_version_id;

            self.write_tree()?;

            let cleanup_result = self.cleanup_erased_version_blobs(erased_version_ids);

            Ok(SoftResetResult::Ok(cleanup_result))
        })
    }

    pub fn create_branch(
        &mut self,
        branch_name: BranchName,
        branch_tip_id: VersionId,
    ) -> Result<CreateBranchResult> {
        use_break!(self, {
            if self.tree.branches.contains_key(&branch_name) {
                return Ok(CreateBranchResult::AlreadyExists);
            }

            if self.tree.version(branch_tip_id).is_none() {
                return Ok(CreateBranchResult::InvalidBranchTipId);
            }

            self.tree.branches.insert(branch_name, branch_tip_id);
            self.write_tree()?;

            Ok(CreateBranchResult::Ok)
        })
    }

    pub fn rename_branch(
        &mut self,
        old_branch_name: BranchName,
        new_branch_name: BranchName,
    ) -> Result<RenameBranchResult> {
        use_break!(self, {
            if old_branch_name == new_branch_name {
                return Ok(RenameBranchResult::Ok);
            }

            if self.tree.branches.contains_key(&new_branch_name) {
                return Ok(RenameBranchResult::NewNameAlreadyExists);
            }

            let Some(branch_version_id) = self.tree.branches.remove(&old_branch_name) else {
                return Ok(RenameBranchResult::OldBranchNotFound);
            };

            if matches!(&self.tree.head, Head::Branch(b) if b == &old_branch_name) {
                self.tree.head = Head::Branch(new_branch_name.clone());
            }

            self.tree
                .branches
                .insert(new_branch_name, branch_version_id);

            self.write_tree()?;

            Ok(RenameBranchResult::Ok)
        })
    }

    pub fn delete_branch(&mut self, branch_name: BranchName) -> Result<DeleteBranchResult> {
        use_break!(self, {
            if !self.tree.branches.contains_key(&branch_name) {
                return Ok(DeleteBranchResult::BranchNotFound);
            };

            let erased_versions = self
                .tree
                .branch_tip_and_ancestors_exclusive(&branch_name)
                .collect_vec();

            let head_version_id = self.tree.head_version_id();

            let erased_version_ids = erased_versions.iter().map(|v| v.id).collect_vec();

            if erased_version_ids.contains(&head_version_id) {
                return Ok(DeleteBranchResult::CannotDeleteHead);
            }

            self.tree.branches.remove(&branch_name);

            self.tree
                .versions
                .retain(|v| !erased_version_ids.contains(&v.id));

            self.write_tree()?;
            let cleanup_result = self.cleanup_erased_version_blobs(erased_version_ids);

            Ok(DeleteBranchResult::Ok(cleanup_result))
        })
    }

    pub fn raw_content_blob(&self, version_id: VersionId) -> Result<Option<impl Read + Seek>> {
        if self.tree.version(version_id).is_none() {
            return Ok(None);
        }

        let path = self.paths.content_blob_path(version_id);

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

        let path = self.paths.preview_path(version_id);

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

    fn write_tree(&self) -> Result<()> {
        write_tree(&self.paths, &self.tree)?;
        Ok(())
    }

    fn overwrite_versioned_file_with_version(&self, version_id: VersionId) -> Result<()> {
        self.extract_version_content(version_id, self.paths.versioned_file())?;
        Ok(())
    }

    fn extract_version_content(&self, version_id: VersionId, destination: &Path) -> Result<()> {
        let mut chain = self
            .tree
            .version_and_ancestors(version_id)
            .take_while_inclusive(|v| v.content_blob_kind.is_patch())
            .collect_vec();

        assert!(!chain.is_empty());

        chain.reverse();

        let version = chain[0];
        assert!(version.content_blob_kind.is_full());
        let full_blob = self.paths.content_blob_path(version.id);

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

        for version in chain.iter().skip(1) {
            assert!(version.content_blob_kind.is_patch());

            let patch = self.paths.content_blob_path(version.id);

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
            if fs::remove_file(self.paths.content_blob_path(id)).is_err()
                || fs::remove_file(self.paths.preview_path(id)).is_err()
            {
                cleanup_result = CleanupResult::Failed;
            }
        }

        cleanup_result
    }
}

#[derive(Debug, IsVariant)]
pub enum CommitResult {
    Ok,
    NoUncommittedChanges,
    HeadMustBeOnBranch,
}

#[derive(Debug, IsVariant)]
pub enum CheckOutBranchResult {
    Ok,
    HasUncommittedChanges,
    BranchNotFound,
}

#[derive(Debug, IsVariant)]
pub enum CheckOutVersionResult {
    Ok,
    HasUncommittedChanges,
    VersionNotFound,
}

#[derive(Debug, IsVariant)]
pub enum UpdateDescriptionResult {
    Ok,
    VersionNotFound,
}

#[derive(Debug, IsVariant)]
pub enum RestoreVersionResult {
    Ok,
    WouldOverwriteUncommittedChanges,
    VersionNotFound,
}

#[derive(Debug, IsVariant)]
pub enum SoftResetResult {
    Ok(CleanupResult),
    HeadMustBeBranch,
    InvalidResetRange,
    ResetRangeOverlapsWithOtherBranches,
}

#[derive(Debug, IsVariant)]
pub enum AmendResult {
    Ok(CleanupResult),
    NoUncommittedChanges,
    HeadMustBeBranch,
    CannotAmendVersionBelongingToAnotherBranch,
}

#[derive(Debug, IsVariant)]
pub enum CreateBranchResult {
    Ok,
    AlreadyExists,
    InvalidBranchTipId,
}

#[derive(Debug, IsVariant)]
pub enum RenameBranchResult {
    Ok,
    NewNameAlreadyExists,
    OldBranchNotFound,
}

#[derive(Debug, IsVariant)]
pub enum DeleteBranchResult {
    Ok(CleanupResult),
    BranchNotFound,
    CannotDeleteHead,
}

#[derive(Debug, IsVariant)]
pub enum CleanupResult {
    Ok,
    Failed,
}
