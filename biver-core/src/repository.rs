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

mod io;
mod paths;
mod session;

pub use session::*;

#[derive(IsVariant)]
pub enum TryStartSessionResult {
    Ok(Session),
    Uninitialized,
}

pub fn try_start_session(
    config: Configuration,
    versioned_file_path: PathBuf,
) -> Result<TryStartSessionResult> {
    let paths = RepositoryPaths::new(versioned_file_path);
    let tree = io::read_tree(&paths)?;

    match tree {
        Some(tree) => Ok(TryStartSessionResult::Ok(Session::new(
            config, paths, tree,
        )?)),
        None => Ok(TryStartSessionResult::Uninitialized),
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
    description: Option<String>,
) -> Result<InitializeResult> {
    let paths = RepositoryPaths::new(versioned_file_path);
    let tree = io::read_tree(&paths)?;

    if let Some(tree) = tree {
        return Ok(InitializeResult {
            session: Session::new(config, paths, tree)?,
            was_already_initialized: true,
        });
    }

    if !fs::exists(paths.repository_dir())? {
        fs::create_dir(paths.repository_dir())?;
    }

    let versioned_file_xxh3_128 = hash::xxh3_128(File::open(&paths.versioned_file())?)?;
    let versioned_file_length = fs::metadata(&paths.versioned_file())?.len();

    let new_version_id = VersionId::new();

    let branch = branch.unwrap_or_default();

    let content_blob_file_path = paths.content_blob_path(new_version_id);
    let preview_blob_file_path = paths.preview_blob_path(new_version_id);

    let has_preview =
        io::try_store_version_preview(&config, &preview_blob_file_path, &paths.versioned_file())?;

    let new_version = Version {
        id: new_version_id,
        creation_time: Utc::now(),
        nickname: nickname::new_nickname(versioned_file_xxh3_128),
        versioned_file_length,
        versioned_file_xxh3_128,
        description: description.unwrap_or_default().to_string(),
        parent: None,
        content_blob_kind: ContentBlobKind::Full,
        has_preview,
    };

    let tree = Tree {
        head: Head::Branch(branch.clone()),
        branches: HashMap::from([(branch, new_version_id)]),
        versions: vec![new_version],
    };

    io::store_version_content_full(&paths.versioned_file(), &content_blob_file_path)?;
    io::write_tree(&paths, &tree)?;

    Ok(InitializeResult {
        session: Session::new(config, paths, tree)?,
        was_already_initialized: false,
    })
}
