use crate::data::{BranchName, VersionId};
use derive_more::IsVariant;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, IsVariant)]
pub enum Head {
    Branch(BranchName),
    Version(VersionId),
}

impl Head {
    pub fn branch(&self) -> Option<&BranchName> {
        match self {
            Head::Branch(branch) => Some(branch),
            Head::Version(_) => None,
        }
    }
}
