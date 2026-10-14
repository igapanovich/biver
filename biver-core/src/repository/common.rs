use crate::data::Tree;
use crate::error::Result;
use crate::repository::paths::RepositoryPaths;
use crate::utilities::hash;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::time::{Duration, SystemTime};

pub fn write_tree(paths: &RepositoryPaths, tree: &Tree) -> Result<()> {
    assert!(tree.valid(), "Tree is not valid: {:#?}", tree);

    let backup1 = paths.tree_backup1_file();
    let backup2 = paths.tree_backup2_file();
    let backup3 = paths.tree_backup3_file();
    let backup4 = paths.tree_backup4_file();
    let backup5 = paths.tree_backup5_file();

    rotate_backup(&backup4, &backup5, Duration::from_hours(24))?;
    rotate_backup(&backup3, &backup4, Duration::from_hours(5))?;
    rotate_backup(&backup2, &backup3, Duration::from_hours(1))?;
    rotate_backup(&backup1, &backup2, Duration::from_mins(5))?;
    rotate_backup(paths.tree_file(), &backup1, Duration::from_secs(10))?;

    let using_tmp = fs::exists(paths.tree_file())?;
    let tree_tmp_path = paths.tree_file().with_extension(".tmp");

    if using_tmp {
        fs::rename(paths.tree_file(), &tree_tmp_path)?;
    }

    ciborium::ser::into_writer(&tree, File::create(paths.tree_file())?)?;

    if using_tmp {
        fs::remove_file(tree_tmp_path)?;
    }

    Ok(())
}
pub struct VersionedFileMeta {
    pub length: u64,
    pub xxh3_128: u128,
}

pub fn versioned_file_meta(paths: &RepositoryPaths) -> Result<VersionedFileMeta> {
    let xxh3_128 = hash::xxh3_128(File::open(&paths.versioned_file())?)?;
    let length = fs::metadata(&paths.versioned_file())?.len();
    Ok(VersionedFileMeta { length, xxh3_128 })
}

fn rotate_backup(previous: &Path, next: &Path, interval: Duration) -> Result<()> {
    if !fs::exists(previous)? {
        return Ok(());
    }

    if fs::exists(next)? && next.metadata()?.modified()? > SystemTime::now() - interval {
        return Ok(());
    }

    fs::copy(previous, next)?;

    Ok(())
}
