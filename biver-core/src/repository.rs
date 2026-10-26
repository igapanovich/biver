use crate::data::tree::NewVersion;
use crate::data::{BranchName, Tree};
use crate::error::Result;
use crate::utilities::external_preview_command::create_preview;
use crate::utilities::xxh3_128;
use crate::{Config, Error};
use chrono::Utc;
pub use paths::Paths;
pub use session::*;
use std::fmt::{Debug, Formatter};
use std::fs;
use std::fs::File;
use std::path::Path;
use tracing::field;
use variantly::Variantly;

mod common;
mod paths;
mod preview;
mod session;

const SCHEMA_VERSION: u32 = 1;

#[allow(clippy::large_enum_variant)]
#[derive(Variantly)]
pub enum TryStartSessionResult {
    Ok(Session),
    NotInitialized,
}

impl Debug for TryStartSessionResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TryStartSessionResult::Ok(_) => {
                write!(f, "Ok(Session)")
            }
            TryStartSessionResult::NotInitialized => {
                write!(f, "NotInitialized")
            }
        }
    }
}

#[tracing::instrument(ret, fields(versioned_file_path_canonical = field::Empty))]
pub fn try_start_session(versioned_file_path: &Path) -> Result<TryStartSessionResult> {
    let versioned_file_path = fs::canonicalize(versioned_file_path)?;
    tracing::Span::current().record(
        "versioned_file_path_canonical",
        field::debug(&versioned_file_path),
    );

    let paths = Paths::from_path_buf(versioned_file_path);

    if !initialized_internal(&paths)? {
        return Ok(TryStartSessionResult::NotInitialized);
    }

    validate_schema_version(&paths)?;

    let tree = ciborium::de::from_reader(File::open(paths.tree_file())?)?;

    let config_str = fs::read(paths.config_file())?;
    let config = toml::de::from_slice(&config_str)?;

    let session = Session::new(config, paths, tree)?;

    Ok(TryStartSessionResult::Ok(session))
}

#[derive(Debug, Variantly)]
pub enum InitializeResult {
    Ok,
    AlreadyInitialized,
}

#[tracing::instrument(ret, fields(versioned_file_path_canonical = field::Empty))]
pub fn initialize(
    config: Config,
    versioned_file_path: &Path,
    branch_name: Option<BranchName>,
    description: String,
) -> Result<InitializeResult> {
    let versioned_file_path = fs::canonicalize(versioned_file_path)?;
    tracing::Span::current().record(
        "versioned_file_path_canonical",
        field::debug(&versioned_file_path),
    );

    let paths = Paths::from_path_buf(versioned_file_path);

    if initialized_internal(&paths)? {
        return Ok(InitializeResult::AlreadyInitialized);
    }

    fs::create_dir_all(paths.repository_dir())?;
    fs::create_dir_all(paths.full_content_dir())?;
    fs::create_dir_all(paths.reverse_patch_dir())?;
    fs::create_dir_all(paths.preview_dir())?;
    fs::create_dir_all(paths.backup_dir())?;

    fs::write(paths.schema_version_file(), SCHEMA_VERSION.to_string())?;

    let config_str = toml::ser::to_string(&config)?;
    fs::write(paths.config_file(), config_str)?;

    let tree = Tree::new(
        branch_name.unwrap_or_else(default_initial_branch_name),
        NewVersion {
            creation_time: Utc::now(),
            xxh3_128: xxh3_128::from_file_path(paths.versioned_file())?,
            description,
        },
    );

    let root_id = tree.head_version_id();

    let content_blob_file_path = paths.version_full_content_file(root_id);
    fs::copy(paths.versioned_file(), &content_blob_file_path)?;

    if let Some(preview_command) = config.preview_command {
        create_preview(
            &preview_command,
            paths.versioned_file(),
            &paths.version_preview_file(root_id),
        )?;
    }

    common::write_tree(&paths, &tree)?;

    Ok(InitializeResult::Ok)
}

pub fn initialized(versioned_file_path: &Path) -> Result<bool> {
    initialized_internal(&Paths::new(versioned_file_path))
}

fn initialized_internal(paths: &Paths) -> Result<bool> {
    Ok(fs::exists(paths.tree_file())?)
}

fn validate_schema_version(paths: &Paths) -> Result<()> {
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
