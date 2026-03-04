use crate::env::Env;
use crate::repository_data::{ContentBlobKind, RepositoryData};
use crate::repository_paths::RepositoryPaths;
use crate::version_id::VersionId;
use crate::{image_magick, xdelta3};
use std::path::Path;
use std::time::{Duration, SystemTime};
use std::{fs, io};

pub enum RepositoryDataResult {
    Initialized(RepositoryData),
    NotInitialized,
}

pub fn read_data(repository_paths: &RepositoryPaths) -> io::Result<RepositoryDataResult> {
    if !repository_paths.data_file.exists() {
        return Ok(RepositoryDataResult::NotInitialized);
    }

    let data_file_contents = fs::read(&repository_paths.data_file)?;
    let repository_data = serde_json::from_slice(&data_file_contents)?;

    Ok(RepositoryDataResult::Initialized(repository_data))
}

pub fn write_data(paths: &RepositoryPaths, data: &RepositoryData) -> io::Result<()> {
    if !data.valid() {
        panic!("Repository data is not valid: {:#?}", data);
    }

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

    let data_file_content = serde_json::to_string_pretty(data)?;
    fs::write(&paths.data_file, data_file_content)?;

    Ok(())
}

pub fn store_version_content_patch(env: &Env, base_blob_file_path: &Path, content_to_store_path: &Path, patch_blob_file_path: &Path) -> io::Result<()> {
    xdelta3::create_patch(env, &base_blob_file_path, content_to_store_path, &patch_blob_file_path)?;

    Ok(())
}

pub fn store_version_content_full(content_to_store_path: &Path, full_blob_file_path: &Path) -> io::Result<()> {
    fs::copy(content_to_store_path, full_blob_file_path)?;

    Ok(())
}

pub fn extract_version_content(env: &Env, repo_paths: &RepositoryPaths, repo_data: &RepositoryData, version_id: VersionId, destination_path: &Path) -> io::Result<()> {
    let mut chain = vec![];

    for version in repo_data.iter_version_and_ancestors(version_id) {
        chain.push(version);
        if version.content_blob_kind.is_full() {
            break;
        }
    }

    chain.reverse();

    let temp_file_name = crate::fs::random_file_name();
    let temp_file_path = repo_paths.file_path(&temp_file_name);

    for version in chain {
        let blob_file_path = repo_paths.file_path(&version.content_blob_file_name);

        match version.content_blob_kind {
            ContentBlobKind::Full => {
                fs::copy(&blob_file_path, destination_path)?;
            }
            ContentBlobKind::Patch => {
                xdelta3::apply_patch(env, destination_path, &blob_file_path, &temp_file_path)?;
                fs::rename(&temp_file_path, destination_path)?;
            }
        }
    }

    Ok(())
}

pub fn store_version_preview(env: &Env, preview_blob_file_path: &Path, content_to_store_path: &Path) -> io::Result<()> {
    image_magick::create_preview(env, content_to_store_path, preview_blob_file_path)?;

    Ok(())
}

fn rotate_backup(previous: &Path, next: &Path, interval: Duration) -> io::Result<()> {
    if !previous.exists() {
        return Ok(());
    }

    if next.exists() && next.metadata()?.modified()? > SystemTime::now() - interval {
        return Ok(());
    }

    fs::copy(previous, next)?;

    Ok(())
}
