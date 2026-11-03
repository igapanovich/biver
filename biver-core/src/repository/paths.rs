use crate::data::{Snapshot, VersionId};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Paths {
    versioned_file: PathBuf,
    repository_dir: PathBuf,
    full_content_dir: PathBuf,
    reverse_patch_dir: PathBuf,
    preview_dir: PathBuf,
    tree_file: PathBuf,
    snapshot_dir: PathBuf,
    tree_snapshot1_file: PathBuf,
    tree_snapshot2_file: PathBuf,
    tree_snapshot3_file: PathBuf,
    tree_snapshot4_file: PathBuf,
    tree_snapshot5_file: PathBuf,
    tree_snapshot6_file: PathBuf,
    tree_snapshot7_file: PathBuf,
    tree_snapshot8_file: PathBuf,
    config_file: PathBuf,
    schema_version_file: PathBuf,
    lock_file: PathBuf,
}

impl Paths {
    pub fn new(versioned_file_path: &Path) -> Self {
        Self::from_path_buf(versioned_file_path.to_path_buf())
    }

    pub fn from_path_buf(versioned_file_path: PathBuf) -> Self {
        let repository_dir = repository_dir(&versioned_file_path);
        let snapshot_dir = repository_dir.join("snapshots");

        Paths {
            versioned_file: versioned_file_path,
            full_content_dir: repository_dir.join("full_contents"),
            reverse_patch_dir: repository_dir.join("reverse_patches"),
            preview_dir: repository_dir.join("previews"),
            tree_file: repository_dir.join("tree"),
            tree_snapshot1_file: snapshot_dir.join("tree_snapshot1"),
            tree_snapshot2_file: snapshot_dir.join("tree_snapshot2"),
            tree_snapshot3_file: snapshot_dir.join("tree_snapshot3"),
            tree_snapshot4_file: snapshot_dir.join("tree_snapshot4"),
            tree_snapshot5_file: snapshot_dir.join("tree_snapshot5"),
            tree_snapshot6_file: snapshot_dir.join("tree_snapshot6"),
            tree_snapshot7_file: snapshot_dir.join("tree_snapshot7"),
            tree_snapshot8_file: snapshot_dir.join("tree_snapshot8"),
            snapshot_dir,
            config_file: repository_dir.join("config.toml"),
            schema_version_file: repository_dir.join("schema_version"),
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

    pub fn full_content_dir(&self) -> &Path {
        &self.full_content_dir
    }

    pub fn version_full_content_file(&self, version_id: VersionId) -> PathBuf {
        self.full_content_dir()
            .join(version_id_file_name(version_id))
    }

    pub fn reverse_patch_dir(&self) -> &Path {
        &self.reverse_patch_dir
    }

    pub fn version_reverse_patch_file(&self, version_id: VersionId) -> PathBuf {
        self.reverse_patch_dir()
            .join(version_id_file_name(version_id))
    }

    pub fn preview_dir(&self) -> &Path {
        &self.preview_dir
    }

    pub fn version_preview_file(&self, version_id: VersionId) -> PathBuf {
        self.preview_dir.join(version_id_file_name(version_id))
    }

    pub fn snapshot_dir(&self) -> &Path {
        &self.snapshot_dir
    }

    pub fn config_file(&self) -> &Path {
        &self.config_file
    }

    pub fn schema_version_file(&self) -> &Path {
        &self.schema_version_file
    }

    pub fn lock_file(&self) -> &Path {
        &self.lock_file
    }

    pub fn tree_file(&self, snapshot: Snapshot) -> &Path {
        match snapshot {
            Snapshot::Main => &self.tree_file,
            Snapshot::Snapshot1 => &self.tree_snapshot1_file,
            Snapshot::Snapshot2 => &self.tree_snapshot2_file,
            Snapshot::Snapshot3 => &self.tree_snapshot3_file,
            Snapshot::Snapshot4 => &self.tree_snapshot4_file,
            Snapshot::Snapshot5 => &self.tree_snapshot5_file,
            Snapshot::Snapshot6 => &self.tree_snapshot6_file,
            Snapshot::Snapshot7 => &self.tree_snapshot7_file,
            Snapshot::Snapshot8 => &self.tree_snapshot8_file,
        }
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
