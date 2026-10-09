use crate::data::{BranchName, ContentBlobKind, Head, Tree, Version, VersionId};
use crate::error::Result;
use crate::repository::io;
use crate::repository::paths::RepositoryPaths;
use crate::utilities::{hash, nickname};
use crate::{Configuration, Error, temp_file};
use chrono::Utc;
use derive_more::IsVariant;
use itertools::Itertools;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;

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

pub struct Session {
    config: Configuration,
    paths: RepositoryPaths,
    tree: Tree,
    broken: bool,
}

impl Session {
    pub(crate) fn new(
        config: Configuration,
        paths: RepositoryPaths,
        tree: Tree,
    ) -> Result<Session> {
        // TODO FS mutex

        Ok(Session {
            config,
            paths,
            tree,
            broken: false,
        })
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

    pub fn commit(&mut self, description: Option<String>) -> Result<CommitResult> {
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

            let parent_version_file_path = temp_file::new_path()?;
            self.extract_version_content(head.id, &parent_version_file_path)?;
            self.store_version_content_patch(
                &parent_version_file_path,
                self.paths.versioned_file(),
                &content_blob_file_path,
            )?;
            fs::remove_file(&parent_version_file_path)?;

            let has_preview = io::try_store_version_preview(
                &self.config,
                &self.paths.preview_blob_path(new_version_id),
                &self.paths.versioned_file(),
            )?;

            let new_version = Version {
                id: new_version_id,
                creation_time: Utc::now(),
                nickname: nickname::new_nickname(versioned_file_xxh3_128),
                versioned_file_length,
                versioned_file_xxh3_128,
                description: description.unwrap_or_default().to_string(),
                parent: Some(head.id),
                content_blob_kind: ContentBlobKind::Patch,
                has_preview,
            };

            self.tree.versions.push(new_version);

            let head_branch_tip = self
                .tree
                .branches
                .get_mut(head_branch)
                .expect("Head must point to a valid branch");
            *head_branch_tip = new_version_id;

            io::write_tree(&self.paths, &self.tree)?;

            Ok(CommitResult::Ok)
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
                .exclusive_branch_tip_and_ancestors(head_branch)
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
                .expect("Head must point to a valid branch");

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
                return Ok(RenameBranchResult::InvalidOldName);
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
                return Ok(DeleteBranchResult::BranchDoesNotExist);
            };

            let erased_versions = self
                .tree
                .exclusive_branch_tip_and_ancestors(&branch_name)
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

    pub fn raw_content_blob(&self, version_id: VersionId) -> Result<Option<impl Read>> {
        if self.tree.version(version_id).is_none() {
            return Ok(None);
        }

        let path = self.paths.content_blob_path(version_id);

        let file = File::open(path)?;

        Ok(Some(file))
    }

    fn write_tree(&self) -> Result<()> {
        io::write_tree(&self.paths, &self.tree)?;
        Ok(())
    }

    fn overwrite_versioned_file_with_version(&self, version_id: VersionId) -> Result<()> {
        self.extract_version_content(version_id, self.paths.versioned_file())?;
        Ok(())
    }

    fn extract_version_content(&self, version_id: VersionId, path: &Path) -> Result<()> {
        io::extract_version_content(&self.config, &self.paths, &self.tree, version_id, path)?;
        Ok(())
    }

    fn store_version_content_patch(
        &self,
        old_path: &Path,
        new_path: &Path,
        patch: &Path,
    ) -> Result<()> {
        io::store_version_content_patch(&self.config, old_path, new_path, patch)?;
        Ok(())
    }

    fn cleanup_erased_version_blobs(
        &self,
        version_ids: impl IntoIterator<Item = VersionId>,
    ) -> CleanupResult {
        let mut cleanup_result = CleanupResult::Ok;

        for id in version_ids.into_iter() {
            if fs::remove_file(self.paths.content_blob_path(id)).is_err()
                || fs::remove_file(self.paths.preview_blob_path(id)).is_err()
            {
                cleanup_result = CleanupResult::Failed;
            }
        }

        cleanup_result
    }
}

#[derive(IsVariant)]
pub enum CommitResult {
    Ok,
    NoUncommittedChanges,
    HeadMustBeOnBranch,
}

#[derive(IsVariant)]
pub enum CheckOutBranchResult {
    Ok,
    HasUncommittedChanges,
    BranchNotFound,
}

#[derive(IsVariant)]
pub enum CheckOutVersionResult {
    Ok,
    HasUncommittedChanges,
    VersionNotFound,
}

#[derive(IsVariant)]
pub enum UpdateDescriptionResult {
    Ok,
    VersionNotFound,
}

#[derive(IsVariant)]
pub enum RestoreVersionResult {
    Ok,
    WouldOverwriteUncommittedChanges,
}

#[derive(IsVariant)]
pub enum SoftResetResult {
    Ok(CleanupResult),
    HeadMustBeBranch,
    InvalidResetRange,
    ResetRangeOverlapsWithOtherBranches,
}

#[derive(IsVariant)]
pub enum CreateBranchResult {
    Ok,
    AlreadyExists,
    InvalidBranchTipId,
}

#[derive(IsVariant)]
pub enum RenameBranchResult {
    Ok,
    NewNameAlreadyExists,
    InvalidOldName,
}

#[derive(IsVariant)]
pub enum DeleteBranchResult {
    Ok(CleanupResult),
    BranchDoesNotExist,
    CannotDeleteHead,
}

#[derive(IsVariant)]
pub enum CleanupResult {
    Ok,
    Failed,
}
