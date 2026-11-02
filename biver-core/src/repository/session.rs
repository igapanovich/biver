use crate::data::tree::{
    AddBranchError, MoveHeadToBranchError, MoveHeadToVersionError, NewVersion,
    OverwriteBranchTipError, OverwriteBranchTipOutcome, PopBranchOutcome, RebaseBranchError,
    RemoveBranchError, RenameBranchError, VersionUpdate,
};
use crate::data::{BranchName, Snapshot, Tree, VersionId};
use crate::environment::{Environment, LockOutcome};
use crate::error::Result;
use crate::repository::paths::Paths;
use crate::repository::preview::Preview;
use crate::repository::{SCHEMA_VERSION, common};
use crate::utilities::external_preview_command::create_preview;
use crate::utilities::{external_diff_command, xxh3_128};
use crate::{Config, Error, temp_file};
use chrono::Utc;
use itertools::Itertools;
use std::collections::HashSet;
use std::io::{Read, Seek};
use std::ops::{Add, AddAssign};
use std::path::{Path, PathBuf};
use std::{io, mem};
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

pub struct Session {
    env: Box<dyn Environment>,
    config: Config,
    paths: Paths,
    tree: Tree,
    snapshot: Snapshot,
    broken: bool,
    lock: std::fs::File,
}

impl Session {
    pub(crate) fn new(
        env: impl Environment + 'static,
        paths: Paths,
        snapshot: Snapshot,
    ) -> Result<Session> {
        let lock = match env.try_lock_file(paths.lock_file())? {
            LockOutcome::Acquired(lock) => lock,
            LockOutcome::Locked => return Err(Error::RepositoryLocked),
        };

        validate_schema_version(&env, &paths)?;

        let Some(tree) = Self::read_tree(&env, &paths, snapshot)? else {
            return Err(Error::SnapshotUnavailable);
        };

        let config_str = env.read_file_to_end(paths.config_file())?;
        let config = toml::de::from_slice(&config_str)?;

        let session = Session {
            env: Box::new(env),
            config,
            paths,
            tree,
            snapshot,
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

    #[tracing::instrument(skip(self), fields(config = ?self.config, versioned_file = ?self.paths.versioned_file()
    ), ret)]
    pub fn commit(&mut self, description: String) -> Result<CommitResult> {
        self.assert_not_read_only()?;

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
                    self.paths.versioned_file(),
                    &self.paths.version_preview_file(new_version_id),
                )?;
            }

            self.flush_tree()?;

            let cleanup_result = self.cleanup(&[parent_of_new_version_id]);

            Ok(CommitResult::Ok(cleanup_result))
        })
    }

    #[tracing::instrument(skip(self), fields(config = ?self.config, versioned_file = ?self.paths.versioned_file()
    ), ret)]
    pub fn amend(&mut self, description: Option<String>) -> Result<AmendResult> {
        self.assert_not_read_only()?;

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
                    self.paths.versioned_file(),
                    &self.paths.version_preview_file(new_version_id),
                )?;
            }

            self.flush_tree()?;

            let cleanup_result = self.cleanup(
                &removed_version
                    .iter()
                    .map(|x| x.id)
                    .chain([former_head_id])
                    .collect_vec(),
            );

            Ok(AmendResult::Ok(cleanup_result))
        })
    }

    #[tracing::instrument(skip(self), fields(config = ?self.config, versioned_file = ?self.paths.versioned_file()
    ), ret)]
    pub fn discard(&self) -> Result<()> {
        self.assert_not_read_only()?;

        use_break_non_critical!(self, {
            let extr_path = self.extraction_path(self.tree.head_version_id())?;
            self.extract_into_versioned_file(&extr_path)?;
            Ok(())
        })
    }

    #[tracing::instrument(skip(self), fields(config = ?self.config, versioned_file = ?self.paths.versioned_file()
    ), ret)]
    pub fn check_out_branch(&mut self, branch: BranchName) -> Result<CheckOutBranchResult> {
        self.assert_not_read_only()?;

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

    #[tracing::instrument(skip(self), fields(config = ?self.config, versioned_file = ?self.paths.versioned_file()
    ), ret)]
    pub fn check_out_version(&mut self, version_id: VersionId) -> Result<CheckOutVersionResult> {
        self.assert_not_read_only()?;

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

    #[tracing::instrument(skip(self), fields(config = ?self.config, versioned_file = ?self.paths.versioned_file()
    ), ret)]
    pub fn update_description(
        &mut self,
        version_id: VersionId,
        new_description: String,
    ) -> Result<UpdateDescriptionResult> {
        self.assert_not_read_only()?;

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

    #[tracing::instrument(skip(self), fields(config = ?self.config, versioned_file = ?self.paths.versioned_file()
    ), ret)]
    pub fn restore_version(
        &self,
        version_id: VersionId,
        destination: &Path,
    ) -> Result<RestoreVersionResult> {
        use_break_non_critical!(self, {
            let destination = if self.env.file_exists(destination)? {
                &self.env.canonicalize_path(destination)?
            } else {
                destination
            };

            let versioned_file = self.env.canonicalize_path(self.paths.versioned_file())?;

            if destination == versioned_file {
                if self.snapshot != Snapshot::Main {
                    return Err(Error::RepositoryReadOnly);
                }

                if self.has_uncommitted_changes()? {
                    return Ok(RestoreVersionResult::WouldOverwriteUncommittedChanges);
                }
            }

            let repository_dir = self.env.canonicalize_path(self.paths.repository_dir())?;

            if destination.starts_with(repository_dir) {
                return Ok(RestoreVersionResult::CannotRestoreIntoRepositoryDir);
            }

            if self.tree.version(version_id).is_none() {
                return Ok(RestoreVersionResult::VersionNotFound);
            }

            if let Some(dest_parent) = destination.parent() {
                self.env.create_dir_all(dest_parent)?;
            }

            let extr_path = self.extraction_path(version_id)?;
            self.extract_path(&extr_path, destination)?;

            Ok(RestoreVersionResult::Ok)
        })
    }

    #[tracing::instrument(skip(self), fields(config = ?self.config, versioned_file = ?self.paths.versioned_file()
    ), ret)]
    pub fn soft_reset(&mut self, stop_at_version_id: VersionId) -> Result<SoftResetResult> {
        self.assert_not_read_only()?;

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
            self.env.remove_file(&temp)?;

            self.flush_tree()?;

            let cleanup_result = self.cleanup(
                &removed_versions
                    .iter()
                    .map(|v| v.id)
                    .chain([former_head_id])
                    .collect_vec(),
            );

            Ok(SoftResetResult::Ok(cleanup_result))
        })
    }

    #[tracing::instrument(skip(self), fields(config = ?self.config, versioned_file = ?self.paths.versioned_file()
    ), ret)]
    pub fn rebase(
        &mut self,
        rebased_version_id: VersionId,
        new_parent_id: VersionId,
    ) -> Result<RebaseResult> {
        self.assert_not_read_only()?;

        use_break!(self, {
            if self.tree.version(rebased_version_id).is_none() {
                return Ok(RebaseResult::InvalidRebasedVersion);
            }

            if self.tree.version(new_parent_id).is_none() {
                return Ok(RebaseResult::InvalidNewParentVersion);
            }

            let Some(head_branch) = self.tree.head_branch().cloned() else {
                return Ok(RebaseResult::HeadMustBeOnBranch);
            };

            let rebased_version_extr_path = self.extraction_path(rebased_version_id)?;
            let new_parent_extr_path = self.extraction_path(new_parent_id)?;

            let rebase_outcome =
                match self
                    .tree
                    .rebase_branch(&head_branch, rebased_version_id, new_parent_id)
                {
                    Ok(rebase_outcome) => rebase_outcome,
                    Err(err) => {
                        return Ok(match err {
                            RebaseBranchError::BranchNotFound => panic!("Head branch not found"),
                            RebaseBranchError::RebasedVersionNotFound => {
                                RebaseResult::InvalidRebasedVersion
                            }
                            RebaseBranchError::NewParentVersionNotFound => {
                                RebaseResult::InvalidNewParentVersion
                            }
                            RebaseBranchError::CannotRebaseRoot => RebaseResult::CannotRebaseRoot,
                            RebaseBranchError::RebasedVersionDoesNotBelongToBranch => {
                                RebaseResult::RebasedVersionDoesNotBelongToHeadBranch
                            }
                        });
                    }
                };

            let new_rebased_version_id = rebase_outcome
                .copied_versions
                .iter()
                .find(|c| c.old_id == rebased_version_id)
                .expect("Rebased version must be copied")
                .new_id;

            let new_parent_content_tmp = temp_file::new_path();
            let rebased_version_content_tmp = temp_file::new_path();
            self.extract_path(&new_parent_extr_path, &new_parent_content_tmp)?;
            self.extract_path(&rebased_version_extr_path, &rebased_version_content_tmp)?;
            self.store_reverse_patch(
                new_rebased_version_id,
                &rebased_version_content_tmp,
                &new_parent_content_tmp,
            )?;
            self.env.remove_file(&new_parent_content_tmp)?;
            self.env.remove_file(&rebased_version_content_tmp)?;

            let new_branch_tip_id = self.tree.head_branch_tip().expect("Validated above").id;

            for copied in &rebase_outcome.copied_versions {
                if copied.old_id != rebased_version_id {
                    let old_reverse_patch = self.paths.version_reverse_patch_file(copied.old_id);
                    let new_reverse_patch = self.paths.version_reverse_patch_file(copied.new_id);

                    self.env
                        .overwrite_copy_file(&old_reverse_patch, &new_reverse_patch)?;
                }

                if copied.new_id == new_branch_tip_id {
                    let old_full_content = self.paths.version_full_content_file(copied.old_id);
                    let new_full_content = self.paths.version_full_content_file(copied.new_id);

                    self.env
                        .overwrite_copy_file(&old_full_content, &new_full_content)?;
                }
            }

            self.flush_tree()?;

            let cleanup_result = self.cleanup(
                &rebase_outcome
                    .removed_versions
                    .iter()
                    .map(|v| v.id)
                    .chain(rebase_outcome.copied_versions.iter().map(|c| c.old_id))
                    .collect_vec(),
            );

            Ok(RebaseResult::Ok(cleanup_result))
        })
    }

    #[tracing::instrument(skip(self), fields(config = ?self.config, versioned_file = ?self.paths.versioned_file()
    ), ret)]
    pub fn create_branch(
        &mut self,
        branch_name: BranchName,
        branch_tip_id: VersionId,
    ) -> Result<CreateBranchResult> {
        self.assert_not_read_only()?;

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

            if !self.env.file_exists(&full_content_path)? {
                self.extract_path(&branch_tip_extr_path, &full_content_path)?;
            }

            self.flush_tree()?;

            Ok(CreateBranchResult::Ok)
        })
    }

    #[tracing::instrument(skip(self), fields(config = ?self.config, versioned_file = ?self.paths.versioned_file()
    ), ret)]
    pub fn rename_branch(
        &mut self,
        old_branch_name: &BranchName,
        new_branch_name: BranchName,
    ) -> Result<RenameBranchResult> {
        self.assert_not_read_only()?;

        use_break!(self, {
            if self.tree.branch_tip(&old_branch_name).is_none() {
                return Ok(RenameBranchResult::OldNameNotFound);
            }

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

    #[tracing::instrument(skip(self), fields(config = ?self.config, versioned_file = ?self.paths.versioned_file()
    ), ret)]
    pub fn remove_branch(&mut self, branch_name: &BranchName) -> Result<RemoveBranchResult> {
        self.assert_not_read_only()?;

        use_break!(self, {
            let Some(branch_tip_id) = self.tree.branch_tip_map().get(branch_name).copied() else {
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

            let cleanup_result = self.cleanup(
                &removed_versions
                    .iter()
                    .map(|v| v.id)
                    .chain([branch_tip_id])
                    .collect_vec(),
            );

            Ok(RemoveBranchResult::Ok(cleanup_result))
        })
    }

    pub fn version_full_content(&self, version_id: VersionId) -> Result<Option<impl Read + Seek>> {
        use_break_non_critical!(self, {
            if self.tree.version(version_id).is_none() {
                return Ok(None);
            }

            let path = self.paths.version_full_content_file(version_id);

            if !self.env.file_exists(&path)? {
                return Ok(None);
            }

            Ok(Some(self.env.read_file(&path)?))
        })
    }

    pub fn version_reverse_patch(&self, version_id: VersionId) -> Result<Option<impl Read + Seek>> {
        use_break_non_critical!(self, {
            if self.tree.version(version_id).is_none() {
                return Ok(None);
            }

            let path = self.paths.version_reverse_patch_file(version_id);

            if !self.env.file_exists(&path)? {
                return Ok(None);
            }

            Ok(Some(self.env.read_file(&path)?))
        })
    }

    pub fn preview(&self, version_id: VersionId) -> Result<Option<Preview>> {
        use_break_non_critical!(self, {
            if self.tree.version(version_id).is_none() {
                return Ok(None);
            }

            let path = self.paths.version_preview_file(version_id);

            if !self.env.file_exists(&path)? {
                return Ok(None);
            }

            Ok(Some(Preview::new(path)?))
        })
    }

    pub fn preview_versioned_file(&self) -> Result<Option<Preview>> {
        use_break_non_critical!(self, {
            let Some(preview_command) = &self.config.preview_command else {
                return Ok(None);
            };

            let temp = temp_file::new_path();
            create_preview(preview_command, self.paths.versioned_file(), &temp)?;

            Ok(Some(Preview::new(temp)?.with_cleanup()))
        })
    }

    #[tracing::instrument(skip(self), fields(config = ?self.config, versioned_file = ?self.paths.versioned_file()
    ), ret)]
    pub fn tidy_up(&self) -> Result<()> {
        self.assert_not_read_only()?;

        use_break_non_critical!(self, {
            let snapshot_trees = self.snapshot_trees()?;
            let all_trees = snapshot_trees.iter().chain([&self.tree]).collect_vec();

            let all_version_ids = all_trees
                .iter()
                .flat_map(|t| t.versions().map(|v| v.id))
                .collect_vec();

            let all_head_tips = all_trees
                .iter()
                .flat_map(|t| t.branch_tip_map().values())
                .copied()
                .collect_vec();

            let retained_full_content_paths = self.canonicalize_existing_paths(
                all_head_tips
                    .iter()
                    .map(|id| self.paths.version_full_content_file(*id)),
            )?;

            let retained_reverse_patch_paths = self.canonicalize_existing_paths(
                all_version_ids
                    .iter()
                    .map(|id| self.paths.version_reverse_patch_file(*id)),
            )?;

            let retained_preview_paths = self.canonicalize_existing_paths(
                all_version_ids
                    .iter()
                    .map(|id| self.paths.version_preview_file(*id)),
            )?;

            for file in self.env.read_dir(self.paths.full_content_dir())? {
                let file = self.env.canonicalize_path(&file)?;
                if !retained_full_content_paths.contains(&file) {
                    self.env.remove_file(&file)?;
                }
            }

            for file in self.env.read_dir(self.paths.reverse_patch_dir())? {
                let file = self.env.canonicalize_path(&file)?;
                if !retained_reverse_patch_paths.contains(&file) {
                    self.env.remove_file(&file)?;
                }
            }

            for file in self.env.read_dir(self.paths.preview_dir())? {
                let file = self.env.canonicalize_path(&file)?;
                if !retained_preview_paths.contains(&file) {
                    self.env.remove_file(&file)?;
                }
            }

            Ok(())
        })
    }

    fn flush_tree(&mut self) -> Result<()> {
        assert_eq!(self.snapshot, Snapshot::Main);
        common::write_tree(self.env.as_mut(), &self.paths, &self.tree)?;
        Ok(())
    }

    fn read_tree(
        env: &(impl Environment + ?Sized),
        paths: &Paths,
        snapshot: Snapshot,
    ) -> Result<Option<Tree>> {
        let tree_path = paths.tree_file(snapshot);

        if !env.file_exists(tree_path)? {
            return Ok(None);
        }

        let tree = ciborium::de::from_reader(env.read_file(tree_path)?)?;
        Ok(Some(tree))
    }

    fn extraction_path(&self, version_id: VersionId) -> Result<ExtractionPath> {
        let mut chains = self.tree.branch_tip_to_version_chains(version_id);
        chains.sort_unstable_by_key(|a| a.len());

        let chain = chains
            .first()
            .expect("There should be at least one path to a valid ID");

        let control_xxh3_128 = chain
            .last()
            .expect("There should be at least one version")
            .xxh3_128;
        let full_content_path = self.paths.version_full_content_file(chain[0].id);

        assert!(self.env.file_exists(&full_content_path)?);

        let mut patches = Vec::with_capacity(chain.len() - 1);

        for v in chain.iter().take(chain.len() - 1) {
            let reverse_patch_path = self.paths.version_reverse_patch_file(v.id);
            assert!(self.env.file_exists(&reverse_patch_path)?);
            patches.push(reverse_patch_path);
        }

        Ok(ExtractionPath {
            control_xxh3_128,
            starting_full: full_content_path,
            patches,
        })
    }

    fn extract_path(&self, path: &ExtractionPath, destination: &Path) -> Result<()> {
        if path.patches.is_empty() {
            assert_eq!(self.xxh3_128(&path.starting_full)?, path.control_xxh3_128);
            self.env
                .overwrite_copy_file(&path.starting_full, destination)?;
            return Ok(());
        }

        let a = temp_file::new_path();
        let b = temp_file::new_path();

        let mut actual = &a;
        let mut stale = &b;
        let mut stale_exists = false;

        self.env.overwrite_copy_file(&path.starting_full, actual)?;

        for patch_path in &path.patches {
            if stale_exists {
                self.env.remove_file(stale)?;
            }

            external_diff_command::apply_patch(&self.config, actual, patch_path, stale)?;
            mem::swap(&mut actual, &mut stale);
            stale_exists = true;
        }

        assert_eq!(self.xxh3_128(actual)?, path.control_xxh3_128);
        self.env.overwrite_copy_file(actual, destination)?;

        self.env.remove_file(&a)?;
        self.env.remove_file(&b)?;

        Ok(())
    }

    fn extract_into_versioned_file(&self, path: &ExtractionPath) -> Result<()> {
        self.extract_path(path, self.paths.versioned_file())?;
        Ok(())
    }

    fn store_full_content(&self, source: &Path, version: VersionId) -> Result<PathBuf> {
        let dest_path = self.paths.version_full_content_file(version);
        self.env.overwrite_copy_file(source, &dest_path)?;
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

    fn remove_file_if_exists(&self, path: &Path) -> CleanupResult {
        let result = self.env.file_exists(path).and_then(|exists| {
            if exists {
                self.env.remove_file(path)?;
            }
            Ok(())
        });

        if result.is_ok() {
            CleanupResult::Ok
        } else {
            CleanupResult::Failed
        }
    }

    fn canonicalize_existing_paths(
        &self,
        paths: impl IntoIterator<Item = impl AsRef<Path>>,
    ) -> Result<Vec<PathBuf>> {
        let paths = paths
            .into_iter()
            .filter_map(|path| match self.env.file_exists(path.as_ref()) {
                Ok(false) => None,
                Ok(true) => Some(self.env.canonicalize_path(path.as_ref())),
                Err(err) => Some(Err(err)),
            })
            .collect::<std::result::Result<Vec<_>, io::Error>>()?;

        Ok(paths)
    }

    fn snapshot_trees(&self) -> Result<Vec<Tree>> {
        let trees = [
            Self::read_tree(self.env.as_ref(), &self.paths, Snapshot::Snapshot1),
            Self::read_tree(self.env.as_ref(), &self.paths, Snapshot::Snapshot2),
            Self::read_tree(self.env.as_ref(), &self.paths, Snapshot::Snapshot3),
            Self::read_tree(self.env.as_ref(), &self.paths, Snapshot::Snapshot4),
            Self::read_tree(self.env.as_ref(), &self.paths, Snapshot::Snapshot5),
            Self::read_tree(self.env.as_ref(), &self.paths, Snapshot::Snapshot6),
            Self::read_tree(self.env.as_ref(), &self.paths, Snapshot::Snapshot7),
            Self::read_tree(self.env.as_ref(), &self.paths, Snapshot::Snapshot8),
        ]
        .into_iter()
        .collect::<Result<Vec<Option<_>>>>()?
        .into_iter()
        .flatten()
        .collect_vec();

        Ok(trees)
    }

    fn cleanup(&self, version_ids: &[VersionId]) -> CleanupResult {
        let Ok(snapshot_trees) = self.snapshot_trees() else {
            return CleanupResult::Failed;
        };

        let all_trees = snapshot_trees.iter().chain([&self.tree]).collect_vec();

        let mut cleanup_result = CleanupResult::Ok;

        let all_branch_tips = all_trees
            .iter()
            .flat_map(|t| t.branch_tip_map().values())
            .copied()
            .collect::<HashSet<_>>();

        for version_id in version_ids {
            let version_is_tip = all_branch_tips.contains(version_id);
            if version_is_tip {
                continue;
            }

            cleanup_result +=
                self.remove_file_if_exists(&self.paths.version_full_content_file(*version_id));

            let version_is_alive = all_trees.iter().any(|t| t.version(*version_id).is_some());
            if !version_is_alive {
                cleanup_result +=
                    self.remove_file_if_exists(&self.paths.version_reverse_patch_file(*version_id));
                cleanup_result +=
                    self.remove_file_if_exists(&self.paths.version_preview_file(*version_id));
            }
        }

        cleanup_result
    }

    fn versioned_file_xxh3_128(&self) -> Result<u128> {
        self.xxh3_128(self.paths.versioned_file())
    }

    fn xxh3_128(&self, path: &Path) -> Result<u128> {
        Ok(xxh3_128::read(self.env.read_file(path)?)?)
    }

    fn read_only(&self) -> bool {
        !matches!(self.snapshot, Snapshot::Main)
    }

    fn assert_not_read_only(&self) -> Result<()> {
        if self.read_only() {
            return Err(Error::RepositoryReadOnly);
        }

        Ok(())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // Explicit unlock in case file descriptor got shared with a child process
        let _ = self.env.unlock_file(&self.lock);
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
pub enum RebaseResult {
    Ok(CleanupResult),
    HeadMustBeOnBranch,
    InvalidRebasedVersion,
    InvalidNewParentVersion,
    CannotRebaseRoot,
    RebasedVersionDoesNotBelongToHeadBranch,
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
    control_xxh3_128: u128,
    starting_full: PathBuf,
    patches: Vec<PathBuf>,
}

fn validate_schema_version(env: &impl Environment, paths: &Paths) -> Result<()> {
    if !env.file_exists(paths.schema_version_file())? {
        return Err(Error::IncompatibleSchemaVersion("".to_string()));
    }

    let schema_version_str = env.read_file_to_string(paths.schema_version_file())?;

    let Ok(schema_version) = schema_version_str.parse::<u32>() else {
        return Err(Error::IncompatibleSchemaVersion(schema_version_str));
    };

    if schema_version != SCHEMA_VERSION {
        return Err(Error::IncompatibleSchemaVersion(schema_version_str));
    }

    Ok(())
}
