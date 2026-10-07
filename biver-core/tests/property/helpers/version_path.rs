use biver_core::data::Repository;

#[derive(Debug, Clone)]
pub struct VersionPath {
    pub start: VersionPathStart,
    pub nodes: Vec<VersionPathNode>,
}

#[derive(Debug, Copy, Clone)]
pub enum VersionPathStart {
    Root,
    Head,
    TipOfBranch(usize),
}

#[derive(Debug, Copy, Clone)]
pub enum VersionPathNode {
    Child(usize),
    Parent,
}

pub trait VersionPathIdExtension {
    fn version_path_id(&self, path: &VersionPath) -> String;
}

impl VersionPathIdExtension for Repository {
    fn version_path_id(&self, path: &VersionPath) -> String {
        let mut current_version = match path.start {
            VersionPathStart::Root => self.root_version(),
            VersionPathStart::Head => self.head_version(),
            VersionPathStart::TipOfBranch(branch_num) => {
                let mut branches = self.branch_names().collect::<Vec<_>>();
                branches.sort();
                let branch_num = branch_num % branches.len();
                let branch = branches[branch_num];
                self.branch_tip_version(branch).unwrap()
            }
        };

        for node in &path.nodes {
            match node {
                VersionPathNode::Child(child_num) => {
                    let children = self.children(current_version.id).collect::<Vec<_>>();

                    if !children.is_empty() {
                        let child_num = child_num % children.len();
                        current_version = children[child_num];
                    }
                }
                VersionPathNode::Parent => {
                    if let Some(parent_id) = current_version.parent {
                        current_version = self.version(parent_id).unwrap();
                    }
                }
            }
        }

        current_version.id.bs58()
    }
}
