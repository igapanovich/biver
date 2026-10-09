use crate::helpers::extensions::IndexExt;
use biver_core::data::{Tree, Version};
use itertools::Itertools;
use proptest::sample::Index;
use std::fmt::Debug;

#[derive(Clone)]
pub struct VersionPath {
    pub branch: Index,
    pub depth_from_tip: Index,
}

impl Debug for VersionPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "/branch({})/down({})",
            self.branch.debug_fraction(),
            self.depth_from_tip.debug_fraction()
        )
    }
}

pub trait ResolveVersionPathExt {
    fn resolve_version_path(&self, path: &VersionPath) -> &Version;
}

impl ResolveVersionPathExt for Tree {
    fn resolve_version_path(&self, path: &VersionPath) -> &Version {
        let mut branches = self.branch_names().collect_vec();
        branches.sort();

        let branch = *path.branch.get(&branches);
        let branch_tip_id = self.branch_tip_ids()[branch];
        let branch_versions_from_tip = self.version_and_ancestors(branch_tip_id).collect_vec();

        *path.depth_from_tip.get(&branch_versions_from_tip)
    }
}
