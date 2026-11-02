use crate::common::diff::Diff;
use crate::common::hash_map_diff::ValueDifference;
use biver_core::data::{BranchName, Head, Version, VersionId};
use biver_core::repository::Session;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TestEnvSnapshot {
    pub versioned_file_content: Vec<u8>,
    pub has_uncommitted_changes: bool,
    pub head: Head,
    pub branches: HashMap<BranchName, VersionId>,
    pub versions: HashSet<Version>,
}

impl TestEnvSnapshot {
    pub fn new(versioned_file_path: &Path, session: &Session) -> Self {
        TestEnvSnapshot {
            versioned_file_content: fs::read(&versioned_file_path).unwrap(),
            has_uncommitted_changes: session.has_uncommitted_changes().unwrap(),
            head: session.tree().head().clone(),
            branches: session.tree().branch_tip_map().clone(),
            versions: session.tree().versions().cloned().collect(),
        }
    }
}

impl<'a> Diff<'a> for TestEnvSnapshot {
    type Diff = TestEnvSnapshotDiff;

    fn diff(old: &'a TestEnvSnapshot, new: &'a TestEnvSnapshot) -> TestEnvSnapshotDiff {
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

        TestEnvSnapshotDiff {
            upserted_branches,
            removed_branches,
            inserted_versions,
            removed_versions,
        }
    }
}

pub struct TestEnvSnapshotDiff {
    pub upserted_branches: HashMap<BranchName, VersionId>,
    pub removed_branches: HashSet<BranchName>,
    pub inserted_versions: HashSet<Version>,
    pub removed_versions: HashSet<Version>,
}
