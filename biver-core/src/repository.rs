use crate::Configuration;
use crate::data::{BranchName, ContentBlobKind, Head, Tree, Version, VersionId};
use crate::error::Result;
use crate::utilities::{hash, nickname};
use chrono::Utc;
use derive_more::IsVariant;
use paths::RepositoryPaths;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::path::PathBuf;

mod common;
mod paths;
mod preview;
mod session;

use crate::repository::common::try_store_preview;
pub use session::*;

#[derive(Debug, IsVariant)]
pub enum TryStartSessionResult {
    Ok(Session),
    NotInitialized,
}

pub fn try_start_session(
    config: Configuration,
    versioned_file_path: PathBuf,
) -> Result<TryStartSessionResult> {
    let paths = RepositoryPaths::new(versioned_file_path);
    let tree = common::read_tree(&paths)?;

    match tree {
        Some(tree) => Ok(TryStartSessionResult::Ok(Session::try_new(
            config, paths, tree,
        )?)),
        None => Ok(TryStartSessionResult::NotInitialized),
    }
}

pub struct InitializeResult {
    pub session: Session,
    pub was_already_initialized: bool,
}

pub fn initialize(
    config: Configuration,
    versioned_file_path: PathBuf,
    branch: Option<BranchName>,
    description: String,
) -> Result<InitializeResult> {
    let paths = RepositoryPaths::new(versioned_file_path);
    let tree = common::read_tree(&paths)?;

    if let Some(tree) = tree {
        return Ok(InitializeResult {
            session: Session::try_new(config, paths, tree)?,
            was_already_initialized: true,
        });
    }

    fs::create_dir_all(paths.repository_dir())?;
    fs::create_dir_all(paths.content_blob_dir())?;
    fs::create_dir_all(paths.preview_dir())?;
    fs::create_dir_all(paths.backup_dir())?;

    let versioned_file_xxh3_128 = hash::xxh3_128(File::open(&paths.versioned_file())?)?;
    let versioned_file_length = fs::metadata(&paths.versioned_file())?.len();

    let new_version_id = VersionId::new();

    let branch = branch.unwrap_or_default();

    let content_blob_file_path = paths.content_blob_path(new_version_id);
    let preview_blob_file_path = paths.preview_path(new_version_id);

    let has_preview = try_store_preview(&config, &paths.versioned_file(), &preview_blob_file_path)?;

    let new_version = Version {
        id: new_version_id,
        creation_time: Utc::now(),
        nickname: nickname::new_nickname(versioned_file_xxh3_128),
        versioned_file_length,
        versioned_file_xxh3_128,
        description,
        parent: None,
        content_blob_kind: ContentBlobKind::Full,
        has_preview,
    };

    let tree = Tree {
        head: Head::Branch(branch.clone()),
        branches: HashMap::from([(branch, new_version_id)]),
        versions: vec![new_version],
    };

    fs::copy(&paths.versioned_file(), &content_blob_file_path)?;
    common::write_tree(&paths, &tree)?;

    Ok(InitializeResult {
        session: Session::try_new(config, paths, tree)?,
        was_already_initialized: false,
    })
}
