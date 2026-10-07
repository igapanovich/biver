use crate::data::VersionId;
use derive_more::IsVariant;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, IsVariant)]
pub enum Head {
    Branch(String),
    Version(VersionId),
}

impl Head {
    pub fn branch(&self) -> Option<&str> {
        match self {
            Head::Branch(branch) => Some(branch),
            Head::Version(_) => None,
        }
    }
}
