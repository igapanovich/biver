use crate::configuration::Configuration;
use crate::data::{ContentBlobKind, Repository, VersionId};
use crate::error::{Error, Result};
use crate::repository_paths::RepositoryPaths;
use crate::{diff, external_command, temp_file};
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

pub enum RepositoryDataResult {
    Initialized(Repository),
    NotInitialized,
}

pub fn read_data(paths: &RepositoryPaths) -> Result<RepositoryDataResult> {
    if !paths.data_file.exists() {
        return Ok(RepositoryDataResult::NotInitialized);
    }

    let data_file_contents = fs::read(&paths.data_file)?;
    let repository_data = serde_json::from_slice(&data_file_contents)?;

    Ok(RepositoryDataResult::Initialized(repository_data))
}

pub fn write_data(paths: &RepositoryPaths, repo: &Repository) -> Result<()> {
    assert!(repo.valid(), "Repository data is not valid: {:#?}", repo);

    let backup1 = paths.file_path("data_backup1.json");
    let backup2 = paths.file_path("data_backup2.json");
    let backup3 = paths.file_path("data_backup3.json");
    let backup4 = paths.file_path("data_backup4.json");
    let backup5 = paths.file_path("data_backup5.json");

    rotate_backup(&backup4, &backup5, Duration::from_hours(24))?;
    rotate_backup(&backup3, &backup4, Duration::from_hours(5))?;
    rotate_backup(&backup2, &backup3, Duration::from_hours(1))?;
    rotate_backup(&backup1, &backup2, Duration::from_mins(5))?;
    rotate_backup(&paths.data_file, &backup1, Duration::from_secs(10))?;

    let data_file_content = serde_json::to_string_pretty(repo)?;
    fs::write(&paths.data_file, data_file_content)?;

    Ok(())
}

pub fn store_version_content_patch(
    config: &Configuration,
    base_blob_file_path: &Path,
    content_to_store_path: &Path,
    patch_blob_file_path: &Path,
) -> Result<()> {
    if fs::exists(patch_blob_file_path)? {
        fs::remove_file(patch_blob_file_path)?;
    }

    diff::create_patch(
        &config,
        &base_blob_file_path,
        content_to_store_path,
        &patch_blob_file_path,
    )?;

    Ok(())
}

pub fn store_version_content_full(
    content_to_store_path: &Path,
    full_blob_file_path: &Path,
) -> Result<()> {
    fs::copy(content_to_store_path, full_blob_file_path)?;

    Ok(())
}

pub fn extract_version_content(
    config: &Configuration,
    paths: &RepositoryPaths,
    repo: &Repository,
    version_id: VersionId,
    destination_path: &Path,
) -> Result<()> {
    let mut chain = vec![];

    for version in repo.iter_version_and_ancestors(version_id) {
        chain.push(version);
        if version.content_blob_kind.is_full() {
            break;
        }
    }

    chain.reverse();

    if let Some(destination_path_parent) = destination_path.parent() {
        fs::create_dir_all(destination_path_parent)?;
    }

    for version in chain {
        let blob_file_path = paths.file_path(&version.content_blob_file_name);

        match version.content_blob_kind {
            ContentBlobKind::Full => {
                fs::copy(&blob_file_path, destination_path)?;
            }
            ContentBlobKind::Patch => {
                let temp_file_path = temp_file::new_path()?;
                diff::apply_patch(&config, destination_path, &blob_file_path, &temp_file_path)?;
                fs::copy(&temp_file_path, destination_path)?;
                fs::remove_file(&temp_file_path)?;
            }
        }
    }

    Ok(())
}

pub fn try_store_version_preview(
    config: &Configuration,
    preview_blob_file_path: &Path,
    content_to_store_path: &Path,
) -> Result<bool> {
    let Some(extension) = content_to_store_path.extension().and_then(|e| e.to_str()) else {
        return Ok(false);
    };

    let file_type_rules = config
        .file_type_rules
        .iter()
        .filter_map(|fr| {
            if fr.extensions.iter().any(|e| e == extension) {
                Some(fr)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    if file_type_rules.len() > 1 {
        return Err(Error::InvalidConfig(format!(
            "Extension '{}' matches more than one file type rule",
            extension
        )));
    }

    let Some(file_type_rule) = file_type_rules.get(0) else {
        return Ok(false);
    };

    let Some(preview_command_template) = file_type_rule.preview_command.as_ref() else {
        return Ok(false);
    };

    external_command::run_templated_command(
        preview_command_template,
        &[
            ("in", &*content_to_store_path.to_string_lossy()),
            ("out", &*preview_blob_file_path.to_string_lossy()),
        ],
    )?;

    Ok(true)
}

fn rotate_backup(previous: &Path, next: &Path, interval: Duration) -> Result<()> {
    if !previous.exists() {
        return Ok(());
    }

    if next.exists() && next.metadata()?.modified()? > SystemTime::now() - interval {
        return Ok(());
    }

    fs::copy(previous, next)?;

    Ok(())
}
