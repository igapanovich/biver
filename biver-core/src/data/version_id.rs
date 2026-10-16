use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VersionId(pub(crate) Uuid);

impl VersionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn bs58(&self) -> String {
        bs58::encode(self.0.as_bytes()).into_string()
    }

    pub fn from_bs58(bs58: &str) -> Option<Self> {
        bs58::decode(bs58)
            .into_vec()
            .ok()
            .and_then(|bytes| Uuid::from_slice(&bytes).ok())
            .map(Self)
    }

    pub fn from_u128(number: u128) -> Self {
        Self(Uuid::from_u128(number))
    }
}

impl Display for VersionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for VersionId {
    fn default() -> Self {
        Self::new()
    }
}
