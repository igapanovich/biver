use crate::helpers::version_path::{ResolveVersionPathExt, VersionPath};
use biver_core::data::Repository;
use derive_more::IsVariant;
use itertools::Itertools;
use proptest::sample::Index;

#[derive(Debug, Clone, IsVariant)]
pub enum CheckoutTarget {
    Version(VersionPath),
    Branch(Index),
}

pub trait ResolveCheckoutTargetExt {
    fn resolve_checkout_target(&self, path: &CheckoutTarget) -> String;
}

impl ResolveCheckoutTargetExt for Repository {
    fn resolve_checkout_target(&self, target: &CheckoutTarget) -> String {
        match target {
            CheckoutTarget::Version(path) => self.resolve_version_path(path).id.bs58(),
            CheckoutTarget::Branch(index) => {
                let mut branches = self.branch_names().collect_vec();
                branches.sort();
                index.get(&branches).to_string()
            }
        }
    }
}
