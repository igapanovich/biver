use crate::data::Repository;
use crate::error::Result;
use crate::hash;
use crate::repository_paths::RepositoryPaths;
use std::fs;
use std::fs::File;

pub fn has_uncommitted_changes(paths: &RepositoryPaths, repo: &Repository) -> Result<bool> {
    let versioned_file_metadata = fs::metadata(&paths.versioned_file)?;
    let head_version = repo.head_version();

    if versioned_file_metadata.len() != head_version.versioned_file_length {
        return Ok(true);
    }

    let versioned_file = File::open(&paths.versioned_file)?;

    let current_xxh3_128 = hash::xxh3_128(&versioned_file)?;

    Ok(head_version.versioned_file_xxh3_128 != current_xxh3_128)
}
