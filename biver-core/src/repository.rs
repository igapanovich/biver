use crate::data::tree::NewVersion;
use crate::data::{BranchName, Snapshot, Tree};
use crate::environment::Environment;
use crate::error::Result;
use crate::utilities::external_preview_command::create_preview;
use crate::utilities::xxh3_128;
use crate::{Config, Error};
use chrono::Utc;
pub use paths::Paths;
pub use session::*;
use std::fmt::Debug;
use std::path::Path;
use tracing::field;
use variantly::Variantly;

mod common;
mod paths;
mod preview;
mod session;

const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Variantly)]
pub enum InitializeResult {
    Ok,
    AlreadyInitialized,
}

#[tracing::instrument(ret, skip(env), fields(versioned_file_path_canonical = field::Empty))]
pub fn initialize<Env: Environment + ?Sized>(
    env: &Env,
    config: Config,
    versioned_file_path: &Path,
    branch_name: Option<BranchName>,
    description: String,
) -> Result<InitializeResult> {
    let versioned_file_path = env.canonicalize_path(versioned_file_path)?;
    tracing::Span::current().record(
        "versioned_file_path_canonical",
        field::debug(&versioned_file_path),
    );

    let paths = Paths::from_path_buf(versioned_file_path);

    if initialized_internal(env, &paths)? {
        return Ok(InitializeResult::AlreadyInitialized);
    }

    env.create_dir_all(paths.repository_dir())?;
    env.create_dir_all(paths.repository_dir())?;
    env.create_dir_all(paths.full_content_dir())?;
    env.create_dir_all(paths.reverse_patch_dir())?;
    env.create_dir_all(paths.preview_dir())?;
    env.create_dir_all(paths.snapshot_dir())?;

    env.overwrite_file(paths.schema_version_file(), &SCHEMA_VERSION.to_string())?;

    let config_str = toml::ser::to_string(&config)?;
    env.overwrite_file(paths.config_file(), &config_str)?;

    let tree = Tree::new(
        branch_name.unwrap_or_else(default_initial_branch_name),
        NewVersion {
            creation_time: Utc::now(),
            xxh3_128: xxh3_128::read(env.read_file(paths.versioned_file())?)?,
            description,
        },
    );

    let root_id = tree.head_version_id();

    let content_blob_file_path = paths.version_full_content_file(root_id);
    env.overwrite_copy_file(paths.versioned_file(), &content_blob_file_path)?;

    if let Some(preview_command) = config.preview_command {
        create_preview(
            &preview_command,
            paths.versioned_file(),
            &paths.version_preview_file(root_id),
        )?;
    }

    common::write_tree(env, &paths, &tree)?;

    Ok(InitializeResult::Ok)
}

#[tracing::instrument(skip(env), fields(versioned_file_path_canonical = field::Empty))]
pub fn start_session<Env: Environment + 'static>(
    env: Env,
    versioned_file_path: &Path,
    snapshot: Snapshot,
) -> Result<Session> {
    let versioned_file_path = env.canonicalize_path(versioned_file_path)?;
    tracing::Span::current().record(
        "versioned_file_path_canonical",
        field::debug(&versioned_file_path),
    );

    let paths = Paths::from_path_buf(versioned_file_path);

    if !initialized_internal(&env, &paths)? {
        return Err(Error::RepositoryNotInitialized);
    }

    let session = Session::new(env, paths, snapshot)?;

    Ok(session)
}

pub fn initialized<Env: Environment + ?Sized>(
    env: &Env,
    versioned_file_path: &Path,
) -> Result<bool> {
    initialized_internal(env, &Paths::new(versioned_file_path))
}

fn initialized_internal<Env: Environment + ?Sized>(env: &Env, paths: &Paths) -> Result<bool> {
    Ok(env.file_exists(paths.tree_file(Snapshot::Main))?)
}

fn default_initial_branch_name() -> BranchName {
    BranchName::new("main".to_string()).unwrap()
}
