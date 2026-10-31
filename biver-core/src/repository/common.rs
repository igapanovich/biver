use crate::data::{Snapshot, Tree};
use crate::environment::Environment;
use crate::error::Result;
use crate::repository::paths::Paths;
use std::path::Path;
use std::time::Duration;

pub fn write_tree(env: &(impl Environment + ?Sized), paths: &Paths, tree: &Tree) -> Result<()> {
    assert!(tree.valid(), "Tree is not valid: {:#?}", tree);

    let main = paths.tree_file(Snapshot::Main);
    let snapshot1 = paths.tree_file(Snapshot::Snapshot1);
    let snapshot2 = paths.tree_file(Snapshot::Snapshot2);
    let snapshot3 = paths.tree_file(Snapshot::Snapshot3);
    let snapshot4 = paths.tree_file(Snapshot::Snapshot4);
    let snapshot5 = paths.tree_file(Snapshot::Snapshot5);
    let snapshot6 = paths.tree_file(Snapshot::Snapshot6);
    let snapshot7 = paths.tree_file(Snapshot::Snapshot7);
    let snapshot8 = paths.tree_file(Snapshot::Snapshot8);

    let using_tmp = env.file_exists(main)?;
    let tree_tmp_path = main.with_extension("tmp");

    if using_tmp {
        env.rename_file(main, &tree_tmp_path)?;
    }

    ciborium::ser::into_writer(&tree, env.create_file(main)?)?;

    if using_tmp {
        env.remove_file(&tree_tmp_path)?;
    }

    rotate_snapshot(env, snapshot7, snapshot8, Duration::from_hours(24 * 7))?;
    rotate_snapshot(env, snapshot6, snapshot7, Duration::from_hours(24 * 3))?;
    rotate_snapshot(env, snapshot5, snapshot6, Duration::from_hours(24))?;
    rotate_snapshot(env, snapshot4, snapshot5, Duration::from_hours(5))?;
    rotate_snapshot(env, snapshot3, snapshot4, Duration::from_hours(1))?;
    rotate_snapshot(env, snapshot2, snapshot3, Duration::from_mins(30))?;
    rotate_snapshot(env, snapshot1, snapshot2, Duration::from_mins(5))?;
    rotate_snapshot(env, main, snapshot1, Duration::from_secs(10))?;

    Ok(())
}

fn rotate_snapshot(
    env: &(impl Environment + ?Sized),
    previous: &Path,
    next: &Path,
    interval: Duration,
) -> Result<()> {
    if !env.file_exists(previous)? {
        return Ok(());
    }

    if env.file_exists(next)? && env.file_mtime(next)? > env.system_time_now() - interval {
        return Ok(());
    }

    env.overwrite_copy_file(previous, next)?;

    Ok(())
}
