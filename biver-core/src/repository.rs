use crate::cache::Cache;
use crate::data::tree::NewVersion;
use crate::data::{BranchName, ContentBlobKind, Tree};
use crate::error::Result;
use crate::repository::common::versioned_file_meta;
use crate::utilities::external_preview_command::create_preview;
use crate::{Configuration, Error};
use chrono::Utc;
pub use paths::RepositoryPaths;
pub use session::*;
use std::fs;
use std::fs::File;
use std::path::Path;
use variantly::Variantly;

mod common;
mod paths;
mod preview;
mod session;

const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Variantly)]
pub enum TryStartSessionResult {
    Ok(Session),
    NotInitialized,
}

pub fn try_start_session(versioned_file_path: &Path) -> Result<TryStartSessionResult> {
    let paths = RepositoryPaths::new(versioned_file_path);

    if !initialized_internal(&paths)? {
        return Ok(TryStartSessionResult::NotInitialized);
    }

    validate_schema_version(&paths)?;

    let tree = ciborium::de::from_reader(File::open(paths.tree_file())?)?;

    let cache = Cache::read(&paths)?;

    let config_str = fs::read(paths.config_file())?;
    let config = toml::de::from_slice(&config_str)?;

    let session = Session::new(config, paths, tree, cache)?;

    Ok(TryStartSessionResult::Ok(session))
}

#[derive(Debug, Variantly)]
pub enum InitializeResult {
    Ok,
    AlreadyInitialized,
}

pub fn initialize(
    config: Configuration,
    versioned_file_path: &Path,
    branch_name: Option<BranchName>,
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
    fs::create_dir_all(paths.cache_dir())?;

    fs::write(paths.schema_version_file(), SCHEMA_VERSION.to_string())?;

    let config_str = toml::ser::to_string(&config)?;
    fs::write(paths.config_file(), config_str)?;

    let versioned_file_meta = versioned_file_meta(&paths)?;

    let preview_command = config.preview_command;

    let tree = Tree::new(
        branch_name.unwrap_or_else(default_initial_branch_name),
        NewVersion {
            creation_time: Utc::now(),
            versioned_file_length: versioned_file_meta.length,
            versioned_file_xxh3_128: versioned_file_meta.xxh3_128,
            description,
            content_blob_kind: ContentBlobKind::Full,
            has_preview: preview_command.is_some(),
        },
    );

    let root_id = tree.head_version_id();

    let content_blob_file_path = paths.version_content_blob_file(root_id);
    fs::copy(&paths.versioned_file(), &content_blob_file_path)?;

    if let Some(preview_command) = preview_command {
        create_preview(
            &preview_command,
            &paths.versioned_file(),
            &paths.version_preview_file(root_id),
        )?;
    }

    common::write_tree(&paths, &tree)?;

    Ok(InitializeResult::Ok)
}

pub fn initialized(versioned_file_path: &Path) -> Result<bool> {
    initialized_internal(&RepositoryPaths::new(versioned_file_path))
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

fn default_initial_branch_name() -> BranchName {
    BranchName::new("main".to_string()).unwrap()
}
