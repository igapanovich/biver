use crate::data::Repository;
use crate::error::Result;
use crate::extensions::CountIsAtLeast;
use crate::operations::resolve_version_id_target;
use crate::repository_io;
use crate::repository_paths::RepositoryPaths;

pub enum Outcome {
    Ok,
    HeadMustBeBranch,
    InvalidTarget,
    CannotLeaveOrphans,
}

pub fn reset(paths: &RepositoryPaths, repo: &mut Repository, target: &str) -> Result<Outcome> {
    let Some(branch) = repo.head.branch() else {
        return Ok(Outcome::HeadMustBeBranch);
    };

    let Some(target_version) = resolve_version_id_target(repo, target) else {
        return Ok(Outcome::InvalidTarget);
    };
    let target_version_id = target_version.id;

    let erased_versions: Vec<_> = repo
        .iter_head_and_ancestors()
        .take_while(|v| v.id != target_version.id)
        .collect();

    let erased_versions_have_root = erased_versions.iter().any(|v| v.is_root());
    if erased_versions_have_root {
        return Ok(Outcome::InvalidTarget);
    }

    let head_has_children = repo
        .iter_children(repo.head_version().id)
        .count_is_at_least(1);
    if head_has_children {
        return Ok(Outcome::CannotLeaveOrphans);
    }

    let erased_versions_have_multi_parents = erased_versions
        .iter()
        .any(|v| repo.iter_children(v.id).count_is_at_least(2));
    if erased_versions_have_multi_parents {
        return Ok(Outcome::CannotLeaveOrphans);
    }

    let erased_version_ids: Vec<_> = erased_versions.iter().map(|v| v.id).collect();

    repo.versions
        .retain(|v| !erased_version_ids.contains(&v.id));
    repo.branches.insert(branch.to_string(), target_version_id);

    repository_io::write_data(paths, &repo)?;

    Ok(Outcome::Ok)
}
