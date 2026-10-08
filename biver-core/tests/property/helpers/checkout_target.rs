use crate::helpers::version_path::{VersionPath, VersionPathIdExtension};
use biver_core::data::Repository;
use derive_more::IsVariant;
use itertools::Itertools;

#[derive(Debug, Clone, IsVariant)]
pub enum CheckoutTarget {
    Version(VersionPath),
    Branch(usize),
}

pub trait ResolveCheckoutTargetExtension {
    fn resolve_checkout_target(&self, path: &CheckoutTarget) -> String;
}

impl ResolveCheckoutTargetExtension for Repository {
    fn resolve_checkout_target(&self, path: &CheckoutTarget) -> String {
        match path {
            CheckoutTarget::Version(version) => self.version_path_id(version),
            CheckoutTarget::Branch(index) => {
                let branches = self.branch_names().collect_vec();
                let index = index % branches.len();
                branches[index].to_string()
            }
        }
    }
}
