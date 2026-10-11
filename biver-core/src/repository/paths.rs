use crate::data::VersionId;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct RepositoryPaths {
    versioned_file: PathBuf,
    repository_dir: PathBuf,
    tree_file: PathBuf,
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

        let tree_file = repository_dir.join("data.json");

        let lock_file = repository_dir.join("lock");

        RepositoryPaths {
            versioned_file: versioned_file_path,
            repository_dir,
            tree_file,
            lock_file,
        }
    }

    pub fn versioned_file(&self) -> &Path {
        &self.versioned_file
    }

    pub fn repository_dir(&self) -> &Path {
        &self.repository_dir
    }

    pub fn lock_file(&self) -> &Path {
        &self.lock_file
    }

    pub fn tree_file(&self) -> &Path {
        &self.tree_file
    }

    pub fn content_blob_path(&self, version_id: VersionId) -> PathBuf {
        let file_name = version_id_file_name(version_id) + "_content";
        self.file_path(&file_name)
    }

    pub fn preview_blob_path(&self, version_id: VersionId) -> PathBuf {
        let file_name = version_id_file_name(version_id) + "_preview";
        self.file_path(&file_name)
    }

    pub fn file_path(&self, file_name: impl AsRef<Path>) -> PathBuf {
        self.repository_dir.join(file_name)
    }
}

fn version_id_file_name(version_id: VersionId) -> String {
    version_id.0.to_string()
}
