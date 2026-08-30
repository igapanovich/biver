use crate::configuration::Configuration;
use crate::data::{ContentBlobKind, Repository, Version, VersionId};
use crate::error::Result;
use crate::operations::preview_blob_file_name;
use crate::repository_paths::RepositoryPaths;
use crate::{hash, nickname, repository_io, temp_file};
use chrono::Utc;
use std::fs;
use std::fs::File;

pub enum Outcome {
    Ok,
    NoUncommittedChanges,
    HeadMustBeBranch,
    CannotAmendParent,
    HeadEqualsParent,
}

pub fn amend(
    config: &Configuration,
    paths: &RepositoryPaths,
    repo: &mut Repository,
    description: Option<&str>,
) -> Result<Outcome> {
    let versioned_file = File::open(&paths.versioned_file)?;
    let versioned_file_xxh3_128 = hash::xxh3_128(&versioned_file)?;
    let versioned_file_length = fs::metadata(&paths.versioned_file)?.len();

    let head = repo.head_version();
    let head_id = head.id;
    let parent_id = head.parent;

    if versioned_file_xxh3_128 == head.versioned_file_xxh3_128 {
        return Ok(Outcome::NoUncommittedChanges);
    }

    let Some(head_branch) = repo.head.branch() else {
        return Ok(Outcome::HeadMustBeBranch);
    };

    if repo.iter_children(head.id).next().is_some() {
        return Ok(Outcome::CannotAmendParent);
    }

    if let Some(parent_id) = parent_id
        && repo.version(parent_id).unwrap().versioned_file_xxh3_128 == versioned_file_xxh3_128
    {
        return Ok(Outcome::HeadEqualsParent);
    }

    let new_version_id = VersionId::new();

    let content_blob_kind = head.content_blob_kind;
    let content_blob_file_path = paths.file_path(&head.content_blob_file_name);

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

    let description = match description {
        Some(description) => description.to_string(),
        None => head.description.clone(),
    };

    let new_head = Version {
        id: new_version_id,
        creation_time: Utc::now(),
        nickname: nickname::new_nickname(versioned_file_xxh3_128),
        versioned_file_length,
        versioned_file_xxh3_128,
        description,
        parent: parent_id,
        content_blob_file_name: head.content_blob_file_name.clone(),
        content_blob_kind,
        preview_blob_file_name,
    };

    repo.branches
        .insert(head_branch.to_string(), new_version_id);
    repo.versions.retain(|v| v.id != head_id);
    repo.versions.push(new_head);

    match content_blob_kind {
        ContentBlobKind::Full => {
            repository_io::store_version_content_full(
                &paths.versioned_file,
                &content_blob_file_path,
            )?;
        }
        ContentBlobKind::Patch => {
            let parent_version_file_path = temp_file::new_path()?;
            let parent_id = parent_id.expect("Patch node must have a parent");
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
        }
    }

    repository_io::write_data(paths, &repo)?;

    Ok(Outcome::Ok)
}
