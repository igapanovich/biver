use biver_core::data::{Repository, Version};
use itertools::Itertools;
use proptest::sample::Index;

#[derive(Debug, Clone)]
pub struct VersionPath {
    pub start: VersionPathStart,
    pub nodes: Vec<VersionPathNode>,
}

#[derive(Debug, Copy, Clone)]
pub enum VersionPathStart {
    Root,
    Head,
    TipOfBranch(Index),
}

#[derive(Debug, Copy, Clone)]
pub enum VersionPathNode {
    Child(Index),
    Parent,
}

pub trait ResolveVersionPathExt {
    fn resolve_version_path(&self, path: &VersionPath) -> &Version;
}

impl ResolveVersionPathExt for Repository {
    fn resolve_version_path(&self, path: &VersionPath) -> &Version {
        let mut current_version = match path.start {
            VersionPathStart::Root => self.root_version(),
            VersionPathStart::Head => self.head_version(),
            VersionPathStart::TipOfBranch(index) => {
                let mut branches = self.branch_names().collect_vec();
                branches.sort();
                let index = index.index(branches.len());
                self.branch_tip_version(branches[index]).unwrap()
            }
        };

        for node in &path.nodes {
            match node {
                VersionPathNode::Child(index) => {
                    let children = self.children(current_version.id).collect_vec();

                    if !children.is_empty() {
                        let index = index.index(children.len());
                        current_version = children[index];
                    }
                }
                VersionPathNode::Parent => {
                    if let Some(parent_id) = current_version.parent {
                        current_version = self.version(parent_id).unwrap();
                    }
                }
            }
        }

        current_version
    }
}
