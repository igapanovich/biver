use crate::data::{BranchName, Head, Version, VersionId};
use crate::utilities::extensions::CountIsAtLeast;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Serialize, Deserialize)]
pub struct Tree {
    pub(crate) head: Head,
    pub(crate) branches: HashMap<BranchName, VersionId>,
    pub(crate) versions: Vec<Version>,
}

impl Tree {
    pub fn head(&self) -> &Head {
        &self.head
    }

    pub fn versions(&self) -> impl Iterator<Item = &Version> {
        self.versions.iter()
    }

    pub fn version(&self, id: VersionId) -> Option<&Version> {
        self.versions.iter().find(|v| v.id == id)
    }

    pub fn version_mut(&mut self, id: VersionId) -> Option<&mut Version> {
        self.versions.iter_mut().find(|v| v.id == id)
    }

    pub fn head_version(&self) -> &Version {
        let head_version = match &self.head {
            Head::Branch(branch) => {
                let head_version_id = self
                    .branches
                    .get(branch)
                    .expect("The branch pointed at by head should always exist");
                self.version(*head_version_id)
            }
            Head::Version(version_id) => self.version(*version_id),
        };

        head_version.expect("Head should always point to a valid version")
    }

    pub fn head_version_id(&self) -> VersionId {
        match &self.head {
            Head::Branch(branch) => *self
                .branches
                .get(branch)
                .expect("The branch pointed at by head should always exist"),
            Head::Version(version_id) => *version_id,
        }
    }

    pub fn root_version(&self) -> &Version {
        let root_version = self
            .versions
            .iter()
            .filter(|v| v.parent.is_none())
            .exactly_one();

        root_version.expect("A single root version should always exist")
    }

    pub fn branch_tip_version(&self, branch_name: &BranchName) -> Option<&Version> {
        let version_id = self.branches.get(branch_name)?;
        let version = self
            .versions
            .iter()
            .find(|v| v.id == *version_id)
            .expect("Failed to get branch tip version");

        Some(version)
    }

    pub fn valid(&self) -> bool {
        let there_is_exactly_one_root =
            self.versions.iter().filter(|v| v.parent.is_none()).count() == 1;

        let all_parent_references_are_valid = self.versions.iter().all(|v| {
            if let Some(parent) = &v.parent {
                self.versions.iter().any(|v2| v2.id == *parent)
            } else {
                true
            }
        });

        let head_reference_is_valid = match &self.head {
            Head::Branch(branch) => self.branches.contains_key(branch),
            Head::Version(version_id) => self.versions.iter().any(|v| v.id == *version_id),
        };

        let all_branches_reference_valid_versions = self
            .branches
            .values()
            .all(|branch_version_id| self.versions.iter().any(|v| v.id == *branch_version_id));

        let all_versions_belong_to_branches = {
            let mut versions_belonging_to_branches = HashSet::new();

            for branch_leaf_id in self.branches.values() {
                for v in self.version_and_ancestors(*branch_leaf_id) {
                    if !versions_belonging_to_branches.insert(v.id) {
                        break;
                    }
                }
            }

            versions_belonging_to_branches.len() == self.versions.len()
        };

        there_is_exactly_one_root
            && all_parent_references_are_valid
            && head_reference_is_valid
            && all_branches_reference_valid_versions
            && all_versions_belong_to_branches
    }

    pub fn version_and_ancestors(&self, version_id: VersionId) -> impl Iterator<Item = &Version> {
        VersionAndAncestors {
            tree: self,
            current_version_id: Some(version_id),
        }
    }

    pub fn head_and_ancestors(&self) -> impl Iterator<Item = &Version> {
        self.version_and_ancestors(self.head_version().id)
    }

    pub fn children(&self, version_id: VersionId) -> impl Iterator<Item = &Version> {
        self.versions
            .iter()
            .filter(move |v| v.parent == Some(version_id))
    }

    pub fn branch_tip_ids(&self) -> &HashMap<BranchName, VersionId> {
        &self.branches
    }

    pub fn branch_names(&self) -> impl Iterator<Item = &BranchName> {
        self.branches.keys().into_iter()
    }

    pub fn branch_leaf(&self, branch: &BranchName) -> Option<&Version> {
        self.branches
            .get(branch)
            .and_then(|version_id| self.version(*version_id))
    }

    pub fn branch_tip_and_ancestors_exclusive<'a>(
        &'a self,
        branch_name: &'a BranchName,
    ) -> impl Iterator<Item = &'a Version> {
        BranchTipAndAncestorsExclusive {
            tree: self,
            branch_name,
            current_version_id: self.branches.get(branch_name).copied(),
            at_branch_tip: true,
        }
    }
}

struct VersionAndAncestors<'a> {
    tree: &'a Tree,
    current_version_id: Option<VersionId>,
}

impl<'a> Iterator for VersionAndAncestors<'a> {
    type Item = &'a Version;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(current_version) = self.current_version_id.and_then(|id| self.tree.version(id))
        else {
            return None;
        };

        self.current_version_id = current_version.parent;

        Some(current_version)
    }
}

struct BranchTipAndAncestorsExclusive<'a> {
    tree: &'a Tree,
    branch_name: &'a BranchName,
    current_version_id: Option<VersionId>,
    at_branch_tip: bool,
}

impl<'a> Iterator for BranchTipAndAncestorsExclusive<'a> {
    type Item = &'a Version;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(current_version) = self.current_version_id.and_then(|id| self.tree.version(id))
        else {
            return None;
        };

        let children_check_count = if self.at_branch_tip { 1 } else { 2 };

        let mut children = self.tree.children(current_version.id);
        if children.count_is_at_least(children_check_count) {
            self.current_version_id = None;
            return None;
        }

        for (name, tip) in self.tree.branches.iter() {
            if name == self.branch_name {
                continue;
            }

            if *tip == current_version.id {
                self.current_version_id = None;
                return None;
            }
        }

        self.at_branch_tip = false;

        Some(current_version)
    }
}
