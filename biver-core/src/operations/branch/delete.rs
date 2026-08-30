use crate::data::Repository;
use crate::error::Result;
use crate::repository_io;
use crate::repository_paths::RepositoryPaths;
use std::collections::HashSet;

pub enum Outcome {
    Ok,
    BranchDoesNotExist,
    CannotDeleteHead,
}

pub fn delete(paths: &RepositoryPaths, repo: &mut Repository, name: &str) -> Result<Outcome> {
    if !repo.branches.contains_key(name) {
        return Ok(Outcome::BranchDoesNotExist);
    }

    let branch_leaf_version_id = repo.branches[name];

    let versions_on_other_branches = {
        let mut result = HashSet::new();
        let leaf_ids = repo
            .branches
            .iter()
            .filter(|(b, _)| *b != name)
            .map(|(_, v)| *v);
        for leaf_id in leaf_ids {
            for version in repo.iter_version_and_ancestors(leaf_id) {
                if !result.insert(version.id) {
                    break;
                }
            }
        }
        result
    };

    let erased_version_ids = repo
        .iter_version_and_ancestors(branch_leaf_version_id)
        .map(|v| v.id)
        .take_while(|id| !versions_on_other_branches.contains(id))
        .collect::<Vec<_>>();

    let head_version = repo.head_version();

    if erased_version_ids.contains(&head_version.id) {
        return Ok(Outcome::CannotDeleteHead);
    }

    repo.branches.remove(name);
    repo.versions
        .retain(|v| !erased_version_ids.contains(&v.id));

    repository_io::write_data(paths, repo)?;

    Ok(Outcome::Ok)
}
