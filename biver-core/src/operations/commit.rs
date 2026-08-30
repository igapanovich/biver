use crate::configuration::Configuration;
use crate::data::{ContentBlobKind, Repository, Version, VersionId};
use crate::error::Result;
use crate::operations::{
    content_blob_file_name, preview_blob_file_name, should_convert_patch_to_full,
};
use crate::repository_paths::RepositoryPaths;
use crate::{hash, nickname, repository_io, temp_file};
use chrono::Utc;
use std::fs;
use std::fs::File;

pub enum Outcome {
    Ok,
    NothingToCommit,
    HeadMustBeOnBranch,
}

pub fn commit(
    config: &Configuration,
    paths: &RepositoryPaths,
    repo: &mut Repository,
    description: Option<&str>,
) -> Result<Outcome> {
    let versioned_file = File::open(&paths.versioned_file)?;
    let versioned_file_xxh3_128 = hash::xxh3_128(&versioned_file)?;
    let versioned_file_length = fs::metadata(&paths.versioned_file)?.len();

    let parent = repo.head_version();
    let parent_id = parent.id;

    if versioned_file_xxh3_128 == parent.versioned_file_xxh3_128 {
        return Ok(Outcome::NothingToCommit);
    }

    let Some(branch) = repo.head.branch() else {
        return Ok(Outcome::HeadMustBeOnBranch);
    };

    let new_version_id = VersionId::new();

    let content_blob_file_name = content_blob_file_name(new_version_id);
    let content_blob_file_path = paths.file_path(&content_blob_file_name);

    let parent_version_file_path = temp_file::new_path()?;
    repository_io::extract_version_content(
        config,
        paths,
        repo,
        parent_id,
        &parent_version_file_path,
    )?;
    repository_io::store_version_content_patch(
        config,
        &parent_version_file_path,
        &paths.versioned_file,
        &content_blob_file_path,
    )?;
    fs::remove_file(&parent_version_file_path)?;

    let patch_length = fs::metadata(&content_blob_file_path)?.len();

    let content_blob_kind;

    if should_convert_patch_to_full(paths, repo, parent_id, patch_length)? {
        content_blob_kind = ContentBlobKind::Full;
        repository_io::store_version_content_full(&paths.versioned_file, &content_blob_file_path)?;
    } else {
        content_blob_kind = ContentBlobKind::Patch;
    }

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
        parent: Some(parent_id),
        content_blob_file_name,
        content_blob_kind,
        preview_blob_file_name,
    };

    repo.versions.push(new_version);
    repo.branches.insert(branch.to_string(), new_version_id);

    repository_io::write_data(paths, repo)?;

    Ok(Outcome::Ok)
}
