use crate::data::{BranchName, ContentBlobKind, Head, Tree, Version, VersionId};
use crate::error::Result;
use crate::utilities::{hash, nickname};
use crate::{Configuration, Error};
use chrono::Utc;
use derive_more::IsVariant;
pub use paths::RepositoryPaths;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};

mod common;
mod paths;
mod preview;
mod session;

use crate::repository::common::try_store_preview;
pub use session::*;

const SCHEMA_VERSION: u32 = 2;

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

    if !initialized_internal(&paths)? {
        return Ok(TryStartSessionResult::NotInitialized);
    }

    validate_schema_version(&paths)?;

    let tree = read_tree(&paths)?;

    Ok(TryStartSessionResult::Ok(Session::new(
        config, paths, tree,
    )?))
}

#[derive(Debug, IsVariant)]
pub enum InitializeResult {
    Ok,
    AlreadyInitialized,
}

pub fn initialize(
    config: Configuration,
    versioned_file_path: PathBuf,
    branch: Option<BranchName>,
    description: String,
) -> Result<InitializeResult> {
    let paths = RepositoryPaths::new(versioned_file_path);

    if initialized_internal(&paths)? {
        return Ok(InitializeResult::AlreadyInitialized);
    }

    fs::create_dir_all(paths.repository_dir())?;
    fs::create_dir_all(paths.content_blob_dir())?;
    fs::create_dir_all(paths.preview_dir())?;
    fs::create_dir_all(paths.backup_dir())?;

    fs::write(paths.schema_version_file(), SCHEMA_VERSION.to_string())?;

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

    Ok(InitializeResult::Ok)
}

pub fn initialized(versioned_file_path: &Path) -> Result<bool> {
    Ok(fs::exists(paths::tree_file(versioned_file_path))?)
}

fn initialized_internal(paths: &RepositoryPaths) -> Result<bool> {
    Ok(fs::exists(paths.tree_file())?)
}

fn validate_schema_version(paths: &RepositoryPaths) -> Result<()> {
    if !fs::exists(paths.schema_version_file())? {
        return Err(Error::IncompatibleSchemaVersion("".to_string()));
    }

    let schema_version_str = fs::read_to_string(paths.schema_version_file())?;

    let Ok(schema_version) = schema_version_str.parse::<u32>() else {
        return Err(Error::IncompatibleSchemaVersion(schema_version_str));
    };

    if schema_version != SCHEMA_VERSION {
        return Err(Error::IncompatibleSchemaVersion(schema_version_str));
    }

    Ok(())
}

fn read_tree(paths: &RepositoryPaths) -> Result<Tree> {
    let data_file_contents = fs::read(paths.tree_file())?;
    let repository_data = serde_json::from_slice(&data_file_contents)?;

    Ok(repository_data)
}
