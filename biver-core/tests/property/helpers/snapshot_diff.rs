use crate::helpers::diff::Diff;
use crate::helpers::hash_map_diff::ValueDifference;
use crate::helpers::snapshot::{Snapshot, VersionSnapshot};
use biver_core::data::{BranchName, VersionId};
use std::collections::{HashMap, HashSet};

pub struct SnapshotDiff {
    pub upserted_branches: HashMap<BranchName, VersionId>,
    pub removed_branches: HashSet<BranchName>,
    pub inserted_versions: HashSet<VersionSnapshot>,
    pub removed_versions: HashSet<VersionSnapshot>,
}

impl<'a> Diff<'a> for Snapshot {
    type Diff = SnapshotDiff;

    fn diff(old: &'a Snapshot, new: &'a Snapshot) -> SnapshotDiff {
        let branch_diff = HashMap::diff(&old.branches, &new.branches);

        let upserted_branches = branch_diff
            .iter()
            .filter_map(|(branch_name, diff)| match diff {
                ValueDifference::Different(_, new) | ValueDifference::LeftMissing(new) => {
                    Some(((*branch_name).clone(), **new))
                }
                _ => None,
            })
            .collect();

        let removed_branches = branch_diff
            .iter()
            .filter_map(|(branch_name, diff)| {
                if diff.is_right_missing() {
                    Some((*branch_name).clone())
                } else {
                    None
                }
            })
            .collect();

        let inserted_versions = new.versions.difference(&old.versions).cloned().collect();
        let removed_versions = old.versions.difference(&new.versions).cloned().collect();

        SnapshotDiff {
            upserted_branches,
            removed_branches,
            inserted_versions,
            removed_versions,
        }
    }
}
