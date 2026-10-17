use crate::data::{BranchName, Head, Version, VersionId};
use crate::utilities::extensions::CountIsAtLeast;
use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use variantly::Variantly;

#[derive(Debug, Serialize, Deserialize)]
pub struct Tree {
    head: Head,
    branches: HashMap<BranchName, VersionId>,
    versions: Vec<Version>,
}

impl Tree {
    pub fn new(branch_name: BranchName, root_version: NewVersion) -> Tree {
        let root_version = Self::new_version(None, root_version);

        Tree {
            head: Head::Branch(branch_name.clone()),
            branches: HashMap::from([(branch_name, root_version.id)]),
            versions: vec![root_version],
        }
    }

    pub fn head(&self) -> &Head {
        &self.head
    }

    pub fn head_branch(&self) -> Option<&BranchName> {
        self.head.branch_ref()
    }

    pub fn versions(&self) -> impl Iterator<Item = &Version> {
        self.versions.iter()
    }

    pub fn version(&self, id: VersionId) -> Option<&Version> {
        self.versions.iter().find(|v| v.id == id)
    }

    pub fn head_version(&self) -> &Version {
        let head_version_id = self.head_version_id();
        self.version(head_version_id)
            .expect("Head should always point at a valid version")
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

    pub fn head_branch_tip(&self) -> Option<&Version> {
        self.head.branch_ref().and_then(|b| self.branch_tip(b))
    }

    pub fn root_version(&self) -> &Version {
        let root_version = self
            .versions
            .iter()
            .filter(|v| v.parent_id.is_none())
            .exactly_one();

        root_version.expect("A single root version should always exist")
    }

    pub fn branch_tip(&self, branch_name: &BranchName) -> Option<&Version> {
        let version_id = self.branches.get(branch_name)?;
        let version = self
            .versions
            .iter()
            .find(|v| v.id == *version_id)
            .expect("Branch tip should be in versions");

        Some(version)
    }

    pub fn valid(&self) -> bool {
        let there_is_exactly_one_root = self
            .versions
            .iter()
            .filter(|v| v.parent_id.is_none())
            .count()
            == 1;

        let all_parent_references_are_valid = self.versions.iter().all(|v| {
            if let Some(parent) = &v.parent_id {
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
                let Some(version_and_ancestors) = self.version_and_ancestors(*branch_leaf_id)
                else {
                    return false;
                };

                for v in version_and_ancestors {
                    if !versions_belonging_to_branches.insert(v.id) {
                        break;
                    }
                }
            }

            versions_belonging_to_branches.len() == self.versions.len()
        };

        let there_are_no_id_duplicates = self
            .versions
            .iter()
            .map(|v| v.id)
            .collect::<HashSet<_>>()
            .len()
            == self.versions.len();

        there_is_exactly_one_root
            && all_parent_references_are_valid
            && head_reference_is_valid
            && all_branches_reference_valid_versions
            && all_versions_belong_to_branches
            && there_are_no_id_duplicates
    }

    pub fn version_and_ancestors(
        &self,
        version_id: VersionId,
    ) -> Option<impl Iterator<Item = &Version>> {
        if self.versions.iter().all(|v| v.id != version_id) {
            return None;
        }

        Some(VersionAndAncestors {
            tree: self,
            current_version_id: Some(version_id),
        })
    }

    pub fn head_and_ancestors(&self) -> impl Iterator<Item = &Version> {
        self.version_and_ancestors(self.head_version().id)
            .expect("Should not be None for head")
    }

    pub fn children(&self, version_id: VersionId) -> impl Iterator<Item = &Version> {
        self.versions
            .iter()
            .filter(move |v| v.parent_id == Some(version_id))
    }

    pub fn branch_tip_map(&self) -> &HashMap<BranchName, VersionId> {
        &self.branches
    }

    pub fn is_branch_tip(&self, version_id: VersionId) -> bool {
        self.branches.values().contains(&version_id)
    }

    pub fn branch_names(&self) -> impl Iterator<Item = &BranchName> {
        self.branches.keys()
    }

    pub fn branch_tip_and_ancestors<'a>(
        &'a self,
        branch_name: &'a BranchName,
    ) -> Option<impl Iterator<Item = &'a Version>> {
        let branch_tip_id = self.branches.get(branch_name).copied()?;
        self.version_and_ancestors(branch_tip_id)
    }

    pub fn branch_tip_and_ancestors_exclusive<'a>(
        &'a self,
        branch_name: &'a BranchName,
    ) -> Option<impl Iterator<Item = &'a Version>> {
        let branch_tip_id = self.branches.get(branch_name).copied()?;

        Some(BranchTipAndAncestorsExclusive {
            tree: self,
            branch_name,
            current_version_id: Some(branch_tip_id),
            at_branch_tip: true,
        })
    }

    pub fn branch_tip_to_version_chains(&self, version_id: VersionId) -> Vec<Vec<&Version>> {
        let mut result: Vec<Vec<&Version>> = Vec::new();

        for tip_id in self.branches.values() {
            let b = self
                .version_and_ancestors(*tip_id)
                .expect("Should be found for a branch tip")
                .take_while_inclusive(|v| v.id != version_id)
                .collect_vec();

            let last_id = b.last().map(|v| v.id);

            if last_id == Some(version_id) {
                result.push(b);
            }
        }

        result
    }

    pub(crate) fn extend_branch(
        &mut self,
        branch_name: &BranchName,
        new_version: NewVersion,
    ) -> Result<VersionId, ExtendBranchError> {
        let branch_tip_id = self
            .branches
            .iter_mut()
            .find_map(|(b, id)| if b == branch_name { Some(id) } else { None });

        let Some(branch_tip_id) = branch_tip_id else {
            return Err(ExtendBranchError::BranchNotFound);
        };

        let new_version = Self::new_version(Some(*branch_tip_id), new_version);
        let id = new_version.id;

        self.versions.push(new_version);
        *branch_tip_id = id;

        Ok(id)
    }

    pub(crate) fn pop_branch(
        &mut self,
        branch_name: &BranchName,
    ) -> Result<PopBranchOutcome, PopBranchError> {
        let branch_tip_id = self
            .branches
            .iter_mut()
            .find_map(|(b, id)| if b == branch_name { Some(id) } else { None });

        let Some(branch_tip_id) = branch_tip_id else {
            return Err(PopBranchError::BranchNotFound);
        };

        let parent_id = self
            .versions
            .iter()
            .find_map(|v| {
                if v.id == *branch_tip_id {
                    Some(v.parent_id)
                } else {
                    None
                }
            })
            .expect("Branch tip must be in versions");

        let Some(parent_id) = parent_id else {
            return Err(PopBranchError::AlreadyAtRoot);
        };

        let former_branch_tip_id = *branch_tip_id;
        *branch_tip_id = parent_id;

        if self.belongs_to_some_branch(former_branch_tip_id) {
            Ok(PopBranchOutcome::PoppedVersionStillInUse {
                former_tip_id: former_branch_tip_id,
                new_tip_id: parent_id,
            })
        } else {
            let remove_index = self
                .version_index(former_branch_tip_id)
                .expect("Branch tip must be in versions");

            let former_branch_tip_version = self.versions.remove(remove_index);

            Ok(PopBranchOutcome::PoppedVersionRemoved {
                former_tip: former_branch_tip_version,
                new_tip_id: parent_id,
            })
        }
    }

    pub(crate) fn overwrite_branch_tip(
        &mut self,
        branch_name: &BranchName,
        create_new_version: impl FnOnce(&Version) -> NewVersion,
    ) -> Result<OverwriteBranchTipOutcome, OverwriteBranchTipError> {
        let branch_count = self.branches.len();

        let branch_tip_id = self
            .branches
            .iter_mut()
            .find_map(|(b, id)| if b == branch_name { Some(id) } else { None });

        let Some(branch_tip_id) = branch_tip_id else {
            return Err(OverwriteBranchTipError::BranchNotFound);
        };

        let former_branch_tip = self
            .versions
            .iter()
            .find(|v| v.id == *branch_tip_id)
            .expect("Branch tip must be in versions");

        if former_branch_tip.is_root() && branch_count != 1 {
            return Err(OverwriteBranchTipError::CannotOverwriteRootWhenOtherBranchesExist);
        }

        let former_branch_tip_id = former_branch_tip.id;

        let new_branch_tip = create_new_version(former_branch_tip);
        let new_branch_tip = Self::new_version(former_branch_tip.parent_id, new_branch_tip);
        let new_branch_tip_id = new_branch_tip.id;

        *branch_tip_id = new_branch_tip.id;

        let has_children = self.children(former_branch_tip.id).count_is_at_least(1);
        let is_tip_of_another_branch = self
            .branches
            .iter()
            .find(|(b, id)| *b != branch_name && **id == former_branch_tip.id)
            .is_some();

        self.versions.push(new_branch_tip);

        let still_in_use = has_children || is_tip_of_another_branch;

        if still_in_use {
            Ok(OverwriteBranchTipOutcome::OverwrittenVersionStillInUse {
                new_tip_id: new_branch_tip_id,
                former_tip_id: former_branch_tip_id,
            })
        } else {
            let remove_index = self
                .version_index(former_branch_tip_id)
                .expect("ID already validated");

            let former_branch_tip_version = self.versions.remove(remove_index);

            Ok(OverwriteBranchTipOutcome::OverwrittenVersionRemoved {
                new_tip_id: new_branch_tip_id,
                former_tip: former_branch_tip_version,
            })
        }
    }

    pub(crate) fn rebase_branch(
        &mut self,
        branch_name: &BranchName,
        rebased_version_id: VersionId,
        new_parent_id: VersionId,
    ) -> Result<RebaseBranchOutcome, RebaseBranchError> {
        if !self.branches.contains_key(branch_name) {
            return Err(RebaseBranchError::BranchNotFound);
        }

        if self.version(rebased_version_id).is_none() {
            return Err(RebaseBranchError::RebasedVersionNotFound);
        }

        if self.version(new_parent_id).is_none() {
            return Err(RebaseBranchError::NewParentVersionNotFound);
        }

        if rebased_version_id == self.root_version().id {
            return Err(RebaseBranchError::CannotRebaseRoot);
        }

        let rebased_range = self
            .branch_tip_and_ancestors(branch_name)
            .expect("Already validated branch name")
            .take_while_inclusive(|v| v.id != rebased_version_id)
            .collect_vec();

        if rebased_range.iter().all(|v| v.id != rebased_version_id) {
            return Err(RebaseBranchError::RebasedVersionDoesNotBelongToBranch);
        }

        let mut copied_versions = Vec::new();
        let mut copy_mappings = Vec::new();
        let mut current_rebase_parent_id = new_parent_id;
        for v in rebased_range.iter().rev() {
            let v_copy = Self::copy_version(Some(current_rebase_parent_id), v);
            let v_copy_id = v_copy.id;

            copy_mappings.push(CopiedVersion {
                new_id: v_copy_id,
                old_id: v.id,
            });

            copied_versions.push(v_copy);
            current_rebase_parent_id = v_copy_id;
        }

        self.versions.extend(copied_versions);

        let branch_tip_id = self
            .branches
            .get_mut(branch_name)
            .expect("Already validated branch name");
        *branch_tip_id = current_rebase_parent_id;

        let removed_versions = self.remove_unreachable_versions();

        Ok(RebaseBranchOutcome {
            removed_versions,
            copied_versions: copy_mappings,
        })
    }

    pub(crate) fn update_version(&mut self, id: VersionId, update: VersionUpdate) -> bool {
        let Some(version) = self.versions.iter_mut().find(|v| v.id == id) else {
            return false;
        };

        if let Some(new_description) = update.description {
            version.description = new_description;
        }

        true
    }

    pub(crate) fn move_head_to_branch(
        &mut self,
        branch: BranchName,
    ) -> Result<(), MoveHeadToBranchError> {
        if !self.branches.keys().contains(&branch) {
            return Err(MoveHeadToBranchError::BranchNotFound);
        }

        self.head = Head::Branch(branch);

        Ok(())
    }

    pub(crate) fn move_head_to_version(
        &mut self,
        version_id: VersionId,
    ) -> Result<(), MoveHeadToVersionError> {
        if !self.versions.iter().any(|v| v.id == version_id) {
            return Err(MoveHeadToVersionError::VersionNotFound);
        }

        self.head = Head::Version(version_id);

        Ok(())
    }

    pub(crate) fn add_branch(
        &mut self,
        branch_name: BranchName,
        version_id: VersionId,
    ) -> Result<(), AddBranchError> {
        if self.branches.contains_key(&branch_name) {
            return Err(AddBranchError::NameAlreadyExists);
        }

        if self.version(version_id).is_none() {
            return Err(AddBranchError::VersionNotFound);
        }

        self.branches.insert(branch_name, version_id);

        Ok(())
    }

    pub(crate) fn rename_branch(
        &mut self,
        old_name: &BranchName,
        new_name: BranchName,
    ) -> Result<(), RenameBranchError> {
        if self.branches.contains_key(&new_name) {
            return Err(RenameBranchError::NewNameAlreadyExists);
        }

        let Some(branch_tip_id) = self.branches.remove(old_name) else {
            return Err(RenameBranchError::OldNameNotFound);
        };

        if self.head.branch_ref() == Some(old_name) {
            self.head = Head::Branch(new_name.clone());
        }

        self.branches.insert(new_name, branch_tip_id);

        Ok(())
    }

    pub(crate) fn remove_branch(
        &mut self,
        branch_name: &BranchName,
    ) -> Result<RemoveBranchOutcome, RemoveBranchError> {
        if !self.branches.contains_key(branch_name) {
            return Err(RemoveBranchError::BranchNotFound);
        }

        let version_ids_to_remove = self
            .branch_tip_and_ancestors_exclusive(branch_name)
            .expect("Should not be None for a valid branch")
            .map(|v| v.id)
            .collect_vec();

        match &self.head {
            Head::Branch(b) => {
                if b == branch_name {
                    return Err(RemoveBranchError::CannotRemoveHeadBranch);
                }
            }
            Head::Version(id) => {
                if version_ids_to_remove.contains(id) {
                    return Err(RemoveBranchError::CannotRemoveHeadVersion);
                }
            }
        }

        let indexes_to_remove = self
            .versions
            .iter()
            .enumerate()
            .filter_map(|(i, v)| {
                if version_ids_to_remove.contains(&v.id) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect_vec();

        let mut removed_versions = Vec::with_capacity(indexes_to_remove.len());

        for index in indexes_to_remove.into_iter().rev() {
            removed_versions.push(self.versions.remove(index));
        }

        self.branches.remove(branch_name);

        Ok(RemoveBranchOutcome { removed_versions })
    }

    fn version_index(&self, id: VersionId) -> Option<usize> {
        self.versions
            .iter()
            .enumerate()
            .find_map(|(i, v)| if v.id == id { Some(i) } else { None })
    }

    fn new_version(parent_id: Option<VersionId>, new_version: NewVersion) -> Version {
        Version {
            id: VersionId::new(),
            creation_time: new_version.creation_time,
            xxh3_128: new_version.xxh3_128,
            description: new_version.description,
            parent_id,
        }
    }

    fn copy_version(parent_id: Option<VersionId>, source_version: &Version) -> Version {
        Version {
            id: VersionId::new(),
            creation_time: source_version.creation_time,
            xxh3_128: source_version.xxh3_128,
            description: source_version.description.clone(),
            parent_id,
        }
    }

    fn belongs_to_some_branch(&self, version_id: VersionId) -> bool {
        let branch_tip_ids = self.branches.values().copied().collect_vec();

        let mut to_check = vec![version_id];

        while !to_check.is_empty() {
            let mut current = Vec::new();
            std::mem::swap(&mut to_check, &mut current);
            for id in current {
                if branch_tip_ids.contains(&id) {
                    return true;
                }

                for c in self.children(id) {
                    to_check.push(c.id);
                }
            }
        }

        false
    }

    fn remove_unreachable_versions(&mut self) -> Vec<Version> {
        let mut reachable_ids: HashSet<VersionId> =
            HashSet::from_iter(self.branches.values().copied());

        let mut to_check_parents = VecDeque::from_iter(self.branches.values().copied());

        while let Some(id) = to_check_parents.pop_front() {
            let parent_id = self
                .versions
                .iter()
                .find(|v| v.id == id)
                .and_then(|v| v.parent_id);

            let Some(parent_id) = parent_id else {
                continue;
            };

            if reachable_ids.insert(parent_id) {
                to_check_parents.push_back(parent_id);
            }
        }

        let mut removed = Vec::new();

        for i in (0..self.versions.len() - 1).rev() {
            if !reachable_ids.contains(&self.versions[i].id) {
                removed.push(self.versions.remove(i));
            }
        }

        removed
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum ExtendBranchError {
    BranchNotFound,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum PopBranchOutcome {
    PoppedVersionStillInUse {
        former_tip_id: VersionId,
        new_tip_id: VersionId,
    },
    PoppedVersionRemoved {
        former_tip: Version,
        new_tip_id: VersionId,
    },
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum PopBranchError {
    BranchNotFound,
    AlreadyAtRoot,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum OverwriteBranchTipOutcome {
    OverwrittenVersionStillInUse {
        former_tip_id: VersionId,
        new_tip_id: VersionId,
    },
    OverwrittenVersionRemoved {
        former_tip: Version,
        new_tip_id: VersionId,
    },
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum OverwriteBranchTipError {
    BranchNotFound,
    CannotOverwriteRootWhenOtherBranchesExist,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum MoveHeadToBranchError {
    BranchNotFound,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum MoveHeadToVersionError {
    VersionNotFound,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum AddBranchError {
    NameAlreadyExists,
    VersionNotFound,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum RenameBranchError {
    OldNameNotFound,
    NewNameAlreadyExists,
}

#[derive(Debug, Clone)]
pub struct RemoveBranchOutcome {
    pub removed_versions: Vec<Version>,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum RemoveBranchError {
    BranchNotFound,
    CannotRemoveHeadBranch,
    CannotRemoveHeadVersion,
}

#[derive(Debug, Clone)]
pub struct RebaseBranchOutcome {
    pub removed_versions: Vec<Version>,
    pub copied_versions: Vec<CopiedVersion>,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Variantly)]
pub enum RebaseBranchError {
    BranchNotFound,
    RebasedVersionNotFound,
    RebasedVersionDoesNotBelongToBranch,
    NewParentVersionNotFound,
    CannotRebaseRoot,
}

pub struct NewVersion {
    pub creation_time: DateTime<Utc>,
    pub xxh3_128: u128,
    pub description: String,
}

#[derive(Default)]
pub struct VersionUpdate {
    pub description: Option<String>,
}

#[derive(Debug, Copy, Clone)]
pub struct CopiedVersion {
    pub old_id: VersionId,
    pub new_id: VersionId,
}

struct VersionAndAncestors<'a> {
    tree: &'a Tree,
    current_version_id: Option<VersionId>,
}

impl<'a> Iterator for VersionAndAncestors<'a> {
    type Item = &'a Version;

    fn next(&mut self) -> Option<Self::Item> {
        let current_version = self
            .current_version_id
            .and_then(|id| self.tree.version(id))?;

        self.current_version_id = current_version.parent_id;

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
        let current_version = self
            .current_version_id
            .and_then(|id| self.tree.version(id))?;

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
        self.current_version_id = current_version.parent_id;

        Some(current_version)
    }
}
