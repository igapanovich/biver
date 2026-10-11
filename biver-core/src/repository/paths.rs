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
    lock_file: PathBuf,
}

impl RepositoryPaths {
    pub fn new(versioned_file_path: PathBuf) -> Self {
        let extension = match versioned_file_path.extension() {
            Some(extension) => {
                let mut extension = OsString::from(extension);
                extension.push(".biver");
                extension
            }
            None => OsString::from("biver"),
        };

        let repository_dir = versioned_file_path.with_extension(extension);

        let backup_dir = repository_dir.join("backup");

        RepositoryPaths {
            versioned_file: versioned_file_path,
            content_blob_dir: repository_dir.join("content"),
            preview_dir: repository_dir.join("preview"),
            tree_file: repository_dir.join("data.json"),
            tree_backup1_file: backup_dir.join("data_backup1.json"),
            tree_backup2_file: backup_dir.join("data_backup2.json"),
            tree_backup3_file: backup_dir.join("data_backup3.json"),
            tree_backup4_file: backup_dir.join("data_backup4.json"),
            tree_backup5_file: backup_dir.join("data_backup5.json"),
            backup_dir,
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

    pub fn content_blob_path(&self, version_id: VersionId) -> PathBuf {
        self.content_blob_dir.join(version_id_file_name(version_id))
    }

    pub fn preview_path(&self, version_id: VersionId) -> PathBuf {
        self.preview_dir.join(version_id_file_name(version_id))
    }
}

fn version_id_file_name(version_id: VersionId) -> String {
    version_id.0.to_string()
}
