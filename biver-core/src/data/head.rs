use crate::data::{BranchName, VersionId};
use serde::{Deserialize, Serialize};
use variantly::Variantly;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, Variantly)]
pub enum Head {
    Branch(BranchName),
    Version(VersionId),
}
