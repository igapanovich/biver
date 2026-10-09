use crate::data::VersionId;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct RepositoryPaths {
    versioned_file: PathBuf,
    repository_dir: PathBuf,
    tree_file: PathBuf,
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

        let repository_dir_path = versioned_file_path.with_extension(extension);

        let data_file_path = repository_dir_path.join("data.json");

        RepositoryPaths {
            versioned_file: versioned_file_path,
            repository_dir: repository_dir_path,
            tree_file: data_file_path,
        }
    }

    pub fn versioned_file(&self) -> &Path {
        &self.versioned_file
    }

    pub fn repository_dir(&self) -> &Path {
        &self.repository_dir
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
