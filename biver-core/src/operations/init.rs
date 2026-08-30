use crate::configuration::Configuration;
use crate::data::{ContentBlobKind, Head, Repository, Version, VersionId};
use crate::error::Result;
use crate::operations::{
    DEFAULT_BRANCH, content_blob_file_name, preview_blob_file_name, valid_branch_name,
};
use crate::repository_paths::RepositoryPaths;
use crate::{hash, nickname, repository_io};
use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::fs::File;

pub enum Outcome {
    Ok,
    AlreadyInitialized,
    InvalidBranchName,
}

pub fn init(
    config: &Configuration,
    paths: &RepositoryPaths,
    branch: Option<impl Into<String>>,
    description: Option<&str>,
) -> Result<Outcome> {
    if fs::exists(&paths.data_file)? {
        return Ok(Outcome::AlreadyInitialized);
    }

    if !fs::exists(&paths.repository_dir)? {
        fs::create_dir(&paths.repository_dir)?;
    }

    let versioned_file = File::open(&paths.versioned_file)?;
    let versioned_file_xxh3_128 = hash::xxh3_128(&versioned_file)?;
    let versioned_file_length = fs::metadata(&paths.versioned_file)?.len();

    let new_version_id = VersionId::new();

    let branch = branch
        .map(|b| b.into())
        .unwrap_or_else(|| DEFAULT_BRANCH.to_string());

    if !valid_branch_name(&branch) {
        return Ok(Outcome::InvalidBranchName);
    }

    let content_blob_file_name = content_blob_file_name(new_version_id);
    let content_blob_file_path = paths.file_path(&content_blob_file_name);

    let preview_blob_file_name = preview_blob_file_name(new_version_id);
    let preview_blob_file_path = paths.file_path(&preview_blob_file_name);

    let preview_stored = repository_io::try_store_version_preview(
        config,
        &preview_blob_file_path,
        &paths.versioned_file,
    )?;

    let preview_blob_file_name = if preview_stored {
        Some(preview_blob_file_name)
    } else {
        None
    };

    let new_version = Version {
        id: new_version_id,
        creation_time: Utc::now(),
        nickname: nickname::new_nickname(versioned_file_xxh3_128),
        versioned_file_length,
        versioned_file_xxh3_128,
        description: description.unwrap_or_default().to_string(),
        parent: None,
        content_blob_file_name,
        content_blob_kind: ContentBlobKind::Full,
        preview_blob_file_name,
    };

    let repo_data = Repository {
        head: Head::Branch(branch.clone()),
        branches: HashMap::from([(branch, new_version_id)]),
        versions: vec![new_version],
    };

    repository_io::store_version_content_full(&paths.versioned_file, &content_blob_file_path)?;
    repository_io::write_data(paths, &repo_data)?;

    Ok(Outcome::Ok)
}
