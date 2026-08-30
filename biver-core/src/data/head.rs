use serde::{Deserialize, Serialize};
use crate::data::VersionId;

#[derive(Debug, Serialize, Deserialize)]
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
