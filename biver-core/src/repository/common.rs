use crate::configuration::Configuration;
use crate::data::Tree;
use crate::error::{Error, Result};
use crate::repository::paths::RepositoryPaths;
use crate::utilities::external_command;
use itertools::Itertools;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

pub fn write_tree(paths: &RepositoryPaths, repo: &Tree) -> Result<()> {
    assert!(repo.valid(), "Repository data is not valid: {:#?}", repo);

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

    let tree_content = serde_json::to_string_pretty(repo)?;
    let tree_tmp = paths.tree_file().with_extension(".tmp");

    if fs::exists(paths.tree_file())? {
        fs::rename(paths.tree_file(), &tree_tmp)?;
    }

    fs::write(paths.tree_file(), tree_content)?;

    if fs::exists(&tree_tmp)? {
        fs::remove_file(tree_tmp)?;
    }

    Ok(())
}

pub fn try_store_preview(
    config: &Configuration,
    preview_source: &Path,
    destination: &Path,
) -> Result<bool> {
    let Some(extension) = preview_source.extension().and_then(|e| e.to_str()) else {
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
        .collect_vec();

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
            ("in", &*preview_source.to_string_lossy()),
            ("out", &*destination.to_string_lossy()),
        ],
    )?;

    Ok(true)
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
