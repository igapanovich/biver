use crate::data::VersionId;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct RepositoryPaths {
    versioned_file: PathBuf,
    repository_dir: PathBuf,
    content_blob_dir: PathBuf,
    preview_dir: PathBuf,
    tree_file: PathBuf,
    backup_dir: PathBuf,
    tree_backup1_file: PathBuf,
    tree_backup2_file: PathBuf,
    tree_backup3_file: PathBuf,
    tree_backup4_file: PathBuf,
    tree_backup5_file: PathBuf,
    config_file: PathBuf,
    schema_version: PathBuf,
    lock_file: PathBuf,
}

impl RepositoryPaths {
    pub fn new(versioned_file_path: &Path) -> Self {
        let repository_dir = repository_dir(&versioned_file_path);
        let backup_dir = repository_dir.join("backup");

        RepositoryPaths {
            versioned_file: versioned_file_path.to_path_buf(),
            content_blob_dir: repository_dir.join("content"),
            preview_dir: repository_dir.join("preview"),
            tree_file: repository_dir.join("tree"),
            tree_backup1_file: backup_dir.join("tree_backup1"),
            tree_backup2_file: backup_dir.join("tree_backup2"),
            tree_backup3_file: backup_dir.join("tree_backup3"),
            tree_backup4_file: backup_dir.join("tree_backup4"),
            tree_backup5_file: backup_dir.join("tree_backup5"),
            backup_dir,
            config_file: repository_dir.join("config.toml"),
            schema_version: repository_dir.join("schema_version"),
            lock_file: repository_dir.join("lock"),
            repository_dir,
        }
    }

    pub fn versioned_file(&self) -> &Path {
        &self.versioned_file
    }

    pub fn repository_dir(&self) -> &Path {
        &self.repository_dir
    }

    pub fn content_blob_dir(&self) -> &Path {
        &self.content_blob_dir
    }

    pub fn preview_dir(&self) -> &Path {
        &self.preview_dir
    }

    pub fn backup_dir(&self) -> &Path {
        &self.backup_dir
    }

    pub fn config_file(&self) -> &Path {
        &self.config_file
    }

    pub fn schema_version_file(&self) -> &Path {
        &self.schema_version
    }

    pub fn lock_file(&self) -> &Path {
        &self.lock_file
    }

    pub fn tree_file(&self) -> &Path {
        &self.tree_file
    }

    pub fn tree_backup1_file(&self) -> &Path {
        &self.tree_backup1_file
    }

    pub fn tree_backup2_file(&self) -> &Path {
        &self.tree_backup2_file
    }

    pub fn tree_backup3_file(&self) -> &Path {
        &self.tree_backup3_file
    }

    pub fn tree_backup4_file(&self) -> &Path {
        &self.tree_backup4_file
    }

    pub fn tree_backup5_file(&self) -> &Path {
        &self.tree_backup5_file
    }

    pub fn version_content_blob_file(&self, version_id: VersionId) -> PathBuf {
        self.content_blob_dir.join(version_id_file_name(version_id))
    }

    pub fn version_preview_file(&self, version_id: VersionId) -> PathBuf {
        self.preview_dir.join(version_id_file_name(version_id))
    }
}

fn repository_dir(versioned_file_path: &Path) -> PathBuf {
    let extension = match versioned_file_path.extension() {
        Some(extension) => {
            let mut extension = OsString::from(extension);
            extension.push(".biver");
            extension
        }
        None => OsString::from("biver"),
    };

    versioned_file_path.with_extension(extension)
}

fn version_id_file_name(version_id: VersionId) -> String {
    version_id.0.to_string()
}
