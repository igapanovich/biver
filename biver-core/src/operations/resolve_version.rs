use crate::data::{Repository, Version};
use crate::operations::{TargetResult, resolve_target};

pub enum Outcome<'a> {
    Ok(&'a Version),
    InvalidTarget,
}

pub fn resolve_version<'a>(repo: &'a Repository, target: &str) -> Outcome<'a> {
    let version = match resolve_target(repo, target) {
        TargetResult::Invalid => return Outcome::InvalidTarget,
        TargetResult::Version(version) => version,
        TargetResult::Branch(branch) => repo
            .branch_leaf(branch)
            .expect("Branch resolved from target must exist"),
    };

    Outcome::Ok(version)
}
