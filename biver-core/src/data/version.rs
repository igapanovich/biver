use crate::data::VersionId;
use crate::utilities::nickname;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct Version {
    #[serde(rename = "i")]
    pub id: VersionId,

    #[serde(rename = "p")]
    pub parent_id: Option<VersionId>,

    #[serde(rename = "t")]
    pub creation_time: DateTime<Utc>,

    #[serde(rename = "h")]
    pub xxh3_128: u128,

    #[serde(rename = "d")]
    pub description: String,
}

impl Version {
    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }

    pub fn nickname(&self) -> String {
        nickname::from(self.xxh3_128)
    }
}
